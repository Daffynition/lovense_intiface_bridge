use std::collections::HashMap;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use axum::{
    body::{Body, Bytes},
    extract::{Query, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Json, Response},
    routing::post,
    Router,
};
use buttplug_client::{ButtplugClient, ButtplugClientDevice};
use tokio::sync::{watch, RwLock};
use tower::Service;
use tower_http::cors::{Any, CorsLayer};

use crate::lovense::{
    apply_device_strength, build_get_toys_response, generate_toy_id, LovenseCommandRequest,
    LovenseResponse, PatternAction,
};

pub fn is_debug_logging_enabled() -> bool {
    cfg!(debug_assertions)
        || std::env::var("LOVENSE_DEBUG")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
}

pub async fn debug_logging_middleware(req: Request, next: Next) -> Response {
    if !is_debug_logging_enabled() {
        return next.run(req).await;
    }

    let method = req.method().clone();
    let uri = req.uri().clone();
    let headers = req.headers().clone();

    let (parts, body) = req.into_parts();
    let req_bytes = match axum::body::to_bytes(body, 10 * 1024 * 1024).await {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[Dev Debug] Error reading request body: {}", e);
            Bytes::new()
        }
    };

    let req_body_str = String::from_utf8_lossy(&req_bytes);

    println!("------------------------------------------------------------");
    println!("[Dev Debug] >>> Incoming Request:");
    println!("  Method:  {}", method);
    println!("  URI:     {}", uri);
    println!("  Headers:");
    for (name, val) in &headers {
        println!("    {}: {}", name, val.to_str().unwrap_or("<binary>"));
    }
    if !req_body_str.is_empty() {
        println!("  Body:    {}", req_body_str);
    } else {
        println!("  Body:    <empty>");
    }

    let req = Request::from_parts(parts, Body::from(req_bytes));
    let res = next.run(req).await;

    let status = res.status();
    let res_headers = res.headers().clone();
    let (parts, res_body) = res.into_parts();
    let res_bytes = match axum::body::to_bytes(res_body, 10 * 1024 * 1024).await {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[Dev Debug] Error reading response body: {}", e);
            Bytes::new()
        }
    };

    let res_body_str = String::from_utf8_lossy(&res_bytes);

    println!("[Dev Debug] <<< Outgoing Response:");
    println!("  Status:  {}", status);
    println!("  Headers:");
    for (name, val) in &res_headers {
        println!("    {}: {}", name, val.to_str().unwrap_or("<binary>"));
    }
    if !res_body_str.is_empty() {
        println!("  Body:    {}", res_body_str);
    } else {
        println!("  Body:    <empty>");
    }
    println!("------------------------------------------------------------\n");

    Response::from_parts(parts, Body::from(res_bytes))
}

pub struct ConnectionGuard {
    addr: String,
    protocol: &'static str,
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        println!("[-] [{}] Client disconnected: {}", self.protocol, self.addr);
    }
}

#[derive(Clone)]
pub struct TrackedConnectionService<S> {
    inner: S,
    _guard: Arc<ConnectionGuard>,
}

impl<S, Req> Service<Req> for TrackedConnectionService<S>
where
    S: Service<Req>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Req) -> Self::Future {
        self.inner.call(req)
    }
}

pub struct TrackedMakeServiceFuture<F> {
    fut: F,
    guard: Option<Arc<ConnectionGuard>>,
}

impl<F, S, E> Future for TrackedMakeServiceFuture<F>
where
    F: Future<Output = Result<S, E>>,
{
    type Output = Result<TrackedConnectionService<S>, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let fut = unsafe { Pin::new_unchecked(&mut this.fut) };
        match fut.poll(cx) {
            Poll::Ready(Ok(service)) => {
                let guard = this.guard.take().unwrap();
                Poll::Ready(Ok(TrackedConnectionService {
                    inner: service,
                    _guard: guard,
                }))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[derive(Clone)]
pub struct ConnectionTrackerMakeService<M> {
    inner: M,
    protocol: &'static str,
}

impl<M> ConnectionTrackerMakeService<M> {
    pub fn new(inner: M, protocol: &'static str) -> Self {
        Self { inner, protocol }
    }
}

fn format_target_address<T: std::fmt::Debug>(target: &T) -> String {
    let debug_str = format!("{:?}", target);
    if let Some(pos) = debug_str.find("remote_addr: ") {
        let rest = &debug_str[pos + "remote_addr: ".len()..];
        let end = rest.find(|c: char| c == ',' || c == '}' || c == ' ' || c == ')').unwrap_or(rest.len());
        let addr = rest[..end].trim();
        if !addr.is_empty() {
            return addr.to_string();
        }
    }
    if let Some(pos) = debug_str.find("peer: ") {
        let rest = &debug_str[pos + "peer: ".len()..];
        let end = rest.find(|c: char| c == ',' || c == '}' || c == ' ' || c == ')').unwrap_or(rest.len());
        let addr = rest[..end].trim();
        if !addr.is_empty() {
            return addr.to_string();
        }
    }
    if let Some(pos) = debug_str.find("addr: ") {
        let rest = &debug_str[pos + "addr: ".len()..];
        let end = rest.find(|c: char| c == ',' || c == '}' || c == ' ' || c == ')').unwrap_or(rest.len());
        let addr = rest[..end].trim();
        if !addr.is_empty() {
            return addr.to_string();
        }
    }
    debug_str
}

impl<M, Target> Service<Target> for ConnectionTrackerMakeService<M>
where
    M: Service<Target>,
    Target: std::fmt::Debug,
{
    type Response = TrackedConnectionService<M::Response>;
    type Error = M::Error;
    type Future = TrackedMakeServiceFuture<M::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, target: Target) -> Self::Future {
        let addr = format_target_address(&target);
        println!("[+] [{}] Client connected: {}", self.protocol, addr);
        let guard = Arc::new(ConnectionGuard {
            addr,
            protocol: self.protocol,
        });
        TrackedMakeServiceFuture {
            fut: self.inner.call(target),
            guard: Some(guard),
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub client: Arc<ButtplugClient>,
    pub pattern_storage: Arc<RwLock<HashMap<String, Vec<PatternAction>>>>,
    pub active_cancellations: Arc<RwLock<HashMap<String, watch::Sender<bool>>>>,
}

impl AppState {
    pub fn new(client: Arc<ButtplugClient>) -> Self {
        Self {
            client,
            pattern_storage: Arc::new(RwLock::new(HashMap::new())),
            active_cancellations: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn cancel_device_tasks(&self, toy_ids: &[String]) {
        let mut cancellations = self.active_cancellations.write().await;
        if toy_ids.is_empty() {
            for (_, tx) in cancellations.drain() {
                let _ = tx.send(true);
            }
        } else {
            for id in toy_ids {
                if let Some(tx) = cancellations.remove(id) {
                    let _ = tx.send(true);
                }
            }
        }
    }

    pub async fn register_cancellation(&self, toy_id: String, tx: watch::Sender<bool>) {
        let mut cancellations = self.active_cancellations.write().await;
        if let Some(old_tx) = cancellations.insert(toy_id, tx) {
            let _ = old_tx.send(true);
        }
    }
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/command", post(handle_command_post).get(handle_command_get))
        .route("/api/lan/v2/command", post(handle_command_post).get(handle_command_get))
        .route("/", post(handle_command_post).get(handle_command_get))
        .fallback(handle_fallback)
        .layer(axum::middleware::from_fn(debug_logging_middleware))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state)
}

pub async fn handle_command_get(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let command = params
        .get("command")
        .or_else(|| params.get("cmd"))
        .or_else(|| params.get("action"))
        .cloned()
        .unwrap_or_else(|| "GetToys".to_string());

    let mut request = LovenseCommandRequest::default();
    request.command = Some(command);
    if let Some(toy) = params.get("toy").or_else(|| params.get("t")) {
        request.toy = Some(serde_json::Value::String(toy.clone()));
    }
    if let Some(action) = params.get("action") {
        request.action = Some(action.clone());
    }
    if let Some(value) = params.get("value") {
        request.value = Some(serde_json::Value::String(value.clone()));
    }
    if let Some(name) = params.get("name") {
        request.name = Some(name.clone());
    }
    if let Some(rule) = params.get("rule") {
        request.rule = Some(rule.clone());
    }
    if let Some(strength) = params.get("strength") {
        request.strength = Some(strength.clone());
    }
    if let Some(req_type) = params.get("type") {
        request.req_type = Some(req_type.clone());
    }
    if let Some(sec) = params.get("timeSec").or_else(|| params.get("sec")) {
        if let Ok(s) = sec.parse::<f64>() {
            request.time_sec = Some(s);
        }
    }

    dispatch_command(state, request).await
}

pub async fn handle_command_post(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    body: Bytes,
) -> Response {
    let mut request: LovenseCommandRequest = if !body.is_empty() {
        match serde_json::from_slice::<LovenseCommandRequest>(&body) {
            Ok(req) => req,
            Err(_) => {
                if let Ok(form_map) = serde_urlencoded::from_bytes::<HashMap<String, String>>(&body) {
                    let mut req = LovenseCommandRequest::default();
                    req.command = form_map.get("command").or_else(|| form_map.get("cmd")).cloned();
                    if let Some(t) = form_map.get("toy") {
                        req.toy = Some(serde_json::Value::String(t.clone()));
                    }
                    req.action = form_map.get("action").cloned();
                    if let Some(v) = form_map.get("value") {
                        req.value = Some(serde_json::Value::String(v.clone()));
                    }
                    req.name = form_map.get("name").cloned();
                    req.rule = form_map.get("rule").cloned();
                    req.strength = form_map.get("strength").cloned();
                    req.req_type = form_map.get("type").cloned();
                    if let Some(s) = form_map.get("timeSec").or_else(|| form_map.get("sec")) {
                        req.time_sec = s.parse::<f64>().ok();
                    }
                    req
                } else {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(LovenseResponse::<serde_json::Value>::error(
                            400,
                            "Invalid JSON or payload body",
                        )),
                    )
                        .into_response();
                }
            }
        }
    } else {
        LovenseCommandRequest::default()
    };

    if request.command.is_none() {
        if let Some(cmd) = params.get("command").or_else(|| params.get("cmd")) {
            request.command = Some(cmd.clone());
        }
    }

    dispatch_command(state, request).await
}

pub async fn handle_fallback(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    body: Bytes,
) -> Response {
    handle_command_post(State(state), Query(params), body).await
}

pub async fn dispatch_command(state: AppState, request: LovenseCommandRequest) -> Response {
    let command_name = request.command.as_deref().unwrap_or("GetToys");

    match command_name.to_lowercase().as_str() {
        "gettoys" => handle_get_toys(state).await,
        "gettoyname" => handle_get_toy_name(state).await,
        "function" => handle_function(state, request).await,
        "position" => handle_position(state, request).await,
        "pattern" => handle_pattern(state, request).await,
        "patternv2" => handle_pattern_v2(state, request).await,
        "preset" => handle_preset(state, request).await,
        other => {
            println!("[Lovense API] Unhandled command: {}", other);
            (
                StatusCode::OK,
                Json(LovenseResponse::<serde_json::Value>::error(
                    400,
                    format!("Command '{}' is not recognized or not yet implemented", command_name),
                )),
            )
                .into_response()
        }
    }
}

pub async fn handle_get_toys(state: AppState) -> Response {
    let devices_map = state.client.devices();
    let devices: Vec<Arc<ButtplugClientDevice>> = devices_map
        .into_values()
        .map(Arc::new)
        .collect();

    let toys_data = build_get_toys_response(&devices).await;
    let response = LovenseResponse::ok(toys_data);

    (StatusCode::OK, Json(response)).into_response()
}

pub async fn handle_get_toy_name(state: AppState) -> Response {
    let devices_map = state.client.devices();
    let mut names: Vec<String> = devices_map
        .values()
        .map(|d| d.name().to_string())
        .collect();

    if names.is_empty() {
        names.push("Domi".to_string());
    }

    let response = LovenseResponse::ok(names);
    (StatusCode::OK, Json(response)).into_response()
}

fn resolve_target_devices(
    client: &Arc<ButtplugClient>,
    target_ids: &[String],
) -> Vec<Arc<ButtplugClientDevice>> {
    let devices_map = client.devices();
    if target_ids.is_empty() {
        devices_map.into_values().map(Arc::new).collect()
    } else {
        devices_map
            .into_values()
            .filter(|d| {
                let id = generate_toy_id(d);
                target_ids.iter().any(|tid| tid.eq_ignore_ascii_case(&id) || tid.eq_ignore_ascii_case(d.name()))
            })
            .map(Arc::new)
            .collect()
    }
}

pub async fn handle_function(state: AppState, request: LovenseCommandRequest) -> Response {
    let target_toy_ids = request.get_target_toys();
    let devices = resolve_target_devices(&state.client, &target_toy_ids);

    if request.stop_previous.unwrap_or(1) != 0 {
        state.cancel_device_tasks(&target_toy_ids).await;
    }

    let action_str = request.action.clone().unwrap_or_default();
    let is_stop = action_str.eq_ignore_ascii_case("stop");

    if is_stop {
        for device in &devices {
            let _ = device.stop().await;
        }
        return (StatusCode::OK, Json(LovenseResponse::ok_simple())).into_response();
    }

    let parsed_actions = parse_function_actions(&action_str);
    let time_sec = request.time_sec.unwrap_or(0.0);
    let loop_running = request.loop_running_sec.unwrap_or(0.0);
    let loop_pause = request.loop_pause_sec.unwrap_or(0.0);

    let (tx, mut rx) = watch::channel(false);
    let task_id = if target_toy_ids.is_empty() {
        "_global_".to_string()
    } else {
        target_toy_ids.join(",")
    };
    state.register_cancellation(task_id, tx).await;

    tokio::spawn(async move {
        let run_actions = |devices: &[Arc<ButtplugClientDevice>]| {
            let devices = devices.to_vec();
            let parsed = parsed_actions.clone();
            async move {
                for device in &devices {
                    for (feature, strength) in &parsed {
                        apply_device_strength(device, feature, *strength).await;
                    }
                }
            }
        };

        let stop_devices = |devices: &[Arc<ButtplugClientDevice>]| {
            let devices = devices.to_vec();
            async move {
                for device in &devices {
                    let _ = device.stop().await;
                }
            }
        };

        if loop_running > 0.0 && loop_pause > 0.0 {
            let start = tokio::time::Instant::now();
            loop {
                if *rx.borrow() {
                    break;
                }
                if time_sec > 0.0 && start.elapsed().as_secs_f64() >= time_sec {
                    break;
                }

                run_actions(&devices).await;

                let run_dur = Duration::from_secs_f64(loop_running);
                tokio::select! {
                    _ = rx.changed() => break,
                    _ = tokio::time::sleep(run_dur) => {}
                }

                if *rx.borrow() {
                    break;
                }
                if time_sec > 0.0 && start.elapsed().as_secs_f64() >= time_sec {
                    break;
                }

                stop_devices(&devices).await;

                let pause_dur = Duration::from_secs_f64(loop_pause);
                tokio::select! {
                    _ = rx.changed() => break,
                    _ = tokio::time::sleep(pause_dur) => {}
                }
            }
            stop_devices(&devices).await;
        } else {
            run_actions(&devices).await;
            if time_sec > 0.0 {
                let dur = Duration::from_secs_f64(time_sec);
                tokio::select! {
                    _ = rx.changed() => {},
                    _ = tokio::time::sleep(dur) => {
                        stop_devices(&devices).await;
                    }
                }
            }
        }
    });

    (StatusCode::OK, Json(LovenseResponse::ok_simple())).into_response()
}

fn parse_function_actions(action_str: &str) -> Vec<(String, f64)> {
    let mut results = Vec::new();
    for part in action_str.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((feat, val_str)) = part.split_once(':') {
            let feat = feat.trim();
            let val = if let Some((_min, max)) = val_str.split_once('-') {
                max.trim().parse::<f64>().unwrap_or(0.0)
            } else {
                val_str.trim().parse::<f64>().unwrap_or(0.0)
            };

            let ratio = match feat.to_lowercase().as_str() {
                "pump" | "depth" => val / 3.0,
                "stroke" => val / 100.0,
                _ => val / 20.0,
            };
            results.push((feat.to_string(), ratio));
        } else {
            // e.g. "Vibrate10"
            let (feat, val_num) = extract_feature_and_number(part);
            let ratio = val_num / 20.0;
            results.push((feat, ratio));
        }
    }

    if results.is_empty() && !action_str.is_empty() {
        results.push(("Vibrate".to_string(), 1.0));
    }
    results
}

fn extract_feature_and_number(s: &str) -> (String, f64) {
    let num_start = s.find(|c: char| c.is_ascii_digit()).unwrap_or(s.len());
    let (feat, num) = s.split_at(num_start);
    let val = num.parse::<f64>().unwrap_or(20.0);
    (feat.to_string(), val)
}

pub async fn handle_position(state: AppState, request: LovenseCommandRequest) -> Response {
    let target_toy_ids = request.get_target_toys();
    let devices = resolve_target_devices(&state.client, &target_toy_ids);

    let pos_val = match &request.value {
        Some(serde_json::Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(serde_json::Value::String(s)) => s.parse::<f64>().unwrap_or(0.0),
        _ => 0.0,
    };

    let ratio = (pos_val / 100.0).clamp(0.0, 1.0);
    for device in devices {
        apply_device_strength(&device, "position", ratio).await;
    }

    (StatusCode::OK, Json(LovenseResponse::ok_simple())).into_response()
}

pub async fn handle_pattern(state: AppState, request: LovenseCommandRequest) -> Response {
    let target_toy_ids = request.get_target_toys();
    let devices = resolve_target_devices(&state.client, &target_toy_ids);

    state.cancel_device_tasks(&target_toy_ids).await;

    let rule = request.rule.unwrap_or_default();
    let interval_ms = parse_pattern_rule_interval(&rule).max(100);
    let strengths: Vec<f64> = request
        .strength
        .as_deref()
        .unwrap_or("20")
        .split(';')
        .filter_map(|s| s.trim().parse::<f64>().ok())
        .map(|s| s / 20.0)
        .collect();

    let strengths = if strengths.is_empty() {
        vec![1.0]
    } else {
        strengths
    };

    let time_sec = request.time_sec.unwrap_or(0.0);
    let (tx, mut rx) = watch::channel(false);
    let task_id = if target_toy_ids.is_empty() {
        "_global_".to_string()
    } else {
        target_toy_ids.join(",")
    };
    state.register_cancellation(task_id, tx).await;

    tokio::spawn(async move {
        let start = tokio::time::Instant::now();
        let interval = Duration::from_millis(interval_ms);
        let mut idx = 0;

        loop {
            if *rx.borrow() {
                break;
            }
            if time_sec > 0.0 && start.elapsed().as_secs_f64() >= time_sec {
                break;
            }

            let ratio = strengths[idx % strengths.len()];
            idx += 1;

            for device in &devices {
                apply_device_strength(device, "vibrate", ratio).await;
            }

            tokio::select! {
                _ = rx.changed() => break,
                _ = tokio::time::sleep(interval) => {}
            }
        }

        for device in &devices {
            let _ = device.stop().await;
        }
    });

    (StatusCode::OK, Json(LovenseResponse::ok_simple())).into_response()
}

fn parse_pattern_rule_interval(rule: &str) -> u64 {
    for part in rule.split(';') {
        let part = part.trim_end_matches('#').trim();
        if let Some((k, v)) = part.split_once(':') {
            if k.eq_ignore_ascii_case("s") {
                if let Ok(ms) = v.parse::<u64>() {
                    return ms;
                }
            }
        }
    }
    1000
}

pub async fn handle_pattern_v2(state: AppState, request: LovenseCommandRequest) -> Response {
    let sub_type = request.req_type.as_deref().unwrap_or("Setup");

    match sub_type.to_lowercase().as_str() {
        "setup" => {
            let actions = request.actions.unwrap_or_default();
            let mut storage = state.pattern_storage.write().await;
            storage.insert("_default_".to_string(), actions);
            (StatusCode::OK, Json(LovenseResponse::ok_simple())).into_response()
        }
        "play" => {
            let target_toy_ids = request.get_target_toys();
            let devices = resolve_target_devices(&state.client, &target_toy_ids);
            state.cancel_device_tasks(&target_toy_ids).await;

            let actions = {
                let storage = state.pattern_storage.read().await;
                storage.get("_default_").cloned().unwrap_or_default()
            };

            let start_time = request.start_time.unwrap_or(0);
            let offset_time = request.offset_time.unwrap_or(0);
            let time_ms = request.time_ms;

            spawn_pattern_v2_player(state.clone(), devices, actions, start_time, offset_time, time_ms, target_toy_ids).await;
            (StatusCode::OK, Json(LovenseResponse::ok_simple())).into_response()
        }
        "initplay" => {
            let target_toy_ids = request.get_target_toys();
            let devices = resolve_target_devices(&state.client, &target_toy_ids);

            if request.stop_previous.unwrap_or(0) == 1 {
                state.cancel_device_tasks(&target_toy_ids).await;
            }

            let actions = request.actions.unwrap_or_default();
            {
                let mut storage = state.pattern_storage.write().await;
                storage.insert("_default_".to_string(), actions.clone());
            }

            let start_time = request.start_time.unwrap_or(0);
            let offset_time = request.offset_time.unwrap_or(0);
            let time_ms = request.time_ms;

            spawn_pattern_v2_player(state.clone(), devices, actions, start_time, offset_time, time_ms, target_toy_ids).await;
            (StatusCode::OK, Json(LovenseResponse::ok_simple())).into_response()
        }
        "stop" => {
            let target_toy_ids = request.get_target_toys();
            let devices = resolve_target_devices(&state.client, &target_toy_ids);
            state.cancel_device_tasks(&target_toy_ids).await;
            for device in devices {
                let _ = device.stop().await;
            }
            (StatusCode::OK, Json(LovenseResponse::ok_simple())).into_response()
        }
        "synctime" => {
            (StatusCode::OK, Json(LovenseResponse::ok_simple())).into_response()
        }
        _ => {
            (StatusCode::OK, Json(LovenseResponse::ok_simple())).into_response()
        }
    }
}

async fn spawn_pattern_v2_player(
    state: AppState,
    devices: Vec<Arc<ButtplugClientDevice>>,
    actions: Vec<PatternAction>,
    start_time: u64,
    offset_time: u64,
    time_ms: Option<f64>,
    target_toy_ids: Vec<String>,
) {
    if actions.is_empty() {
        return;
    }

    let (tx, mut rx) = watch::channel(false);
    let task_id = if target_toy_ids.is_empty() {
        "_global_".to_string()
    } else {
        target_toy_ids.join(",")
    };
    state.register_cancellation(task_id, tx).await;

    tokio::spawn(async move {
        let effective_start_ms = start_time + offset_time;
        let mut filtered_actions: Vec<&PatternAction> = actions
            .iter()
            .filter(|a| a.ts >= effective_start_ms)
            .collect();

        if filtered_actions.is_empty() {
            filtered_actions = actions.iter().collect();
        }

        let base_ts = filtered_actions.first().map(|a| a.ts).unwrap_or(0);
        let play_start = tokio::time::Instant::now();

        for action in filtered_actions {
            if *rx.borrow() {
                break;
            }

            let target_elapsed_ms = action.ts.saturating_sub(base_ts);
            let current_elapsed_ms = play_start.elapsed().as_millis() as u64;

            if let Some(max_time) = time_ms {
                if current_elapsed_ms as f64 >= max_time {
                    break;
                }
            }

            if target_elapsed_ms > current_elapsed_ms {
                let delay = Duration::from_millis(target_elapsed_ms - current_elapsed_ms);
                tokio::select! {
                    _ = rx.changed() => break,
                    _ = tokio::time::sleep(delay) => {}
                }
            }

            if *rx.borrow() {
                break;
            }

            let ratio = (action.pos as f64 / 100.0).clamp(0.0, 1.0);
            for device in &devices {
                apply_device_strength(device, "position", ratio).await;
            }
        }

        for device in &devices {
            let _ = device.stop().await;
        }
    });
}

pub async fn handle_preset(state: AppState, request: LovenseCommandRequest) -> Response {
    let target_toy_ids = request.get_target_toys();
    let devices = resolve_target_devices(&state.client, &target_toy_ids);

    state.cancel_device_tasks(&target_toy_ids).await;

    let preset_name = request.name.as_deref().unwrap_or("pulse").to_lowercase();
    let time_sec = request.time_sec.unwrap_or(9.0);

    let (tx, mut rx) = watch::channel(false);
    let task_id = if target_toy_ids.is_empty() {
        "_global_".to_string()
    } else {
        target_toy_ids.join(",")
    };
    state.register_cancellation(task_id, tx).await;

    tokio::spawn(async move {
        let start = tokio::time::Instant::now();
        let tick_rate = Duration::from_millis(100);

        while !*rx.borrow() {
            let elapsed = start.elapsed().as_secs_f64();
            if time_sec > 0.0 && elapsed >= time_sec {
                break;
            }

            let strength: f64 = match preset_name.as_str() {
                "pulse" => {
                    if (elapsed % 1.0) < 0.5 {
                        1.0
                    } else {
                        0.0
                    }
                }
                "wave" => {
                    ((elapsed * std::f64::consts::PI).sin() + 1.0) / 2.0
                }
                "fireworks" => {
                    let phase = elapsed % 2.0;
                    if phase < 0.3 {
                        1.0
                    } else if phase < 0.8 {
                        0.5
                    } else {
                        0.0
                    }
                }
                "earthquake" => {
                    let phase = (elapsed * 10.0) as u64 % 3;
                    match phase {
                        0 => 0.4,
                        1 => 0.8,
                        _ => 1.0,
                    }
                }
                _ => 1.0,
            };

            for device in &devices {
                apply_device_strength(device, "vibrate", strength).await;
            }

            tokio::select! {
                _ = rx.changed() => break,
                _ = tokio::time::sleep(tick_rate) => {}
            }
        }

        for device in &devices {
            let _ = device.stop().await;
        }
    });

    (StatusCode::OK, Json(LovenseResponse::ok_simple())).into_response()
}

pub async fn start_servers(
    client: Arc<ButtplugClient>,
    http_port: u16,
    https_port: u16,
) -> anyhow::Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let state = AppState::new(client);
    let app = create_router(state);

    let http_addr = SocketAddr::from(([0, 0, 0, 0], http_port));
    println!("Starting HTTP server on http://0.0.0.0:{}", http_port);
    let app_http = app.clone();
    let http_handle = tokio::spawn(async move {
        let listener = match tokio::net::TcpListener::bind(http_addr).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Failed to bind HTTP port {}: {}", http_port, e);
                return;
            }
        };
        let make_service = ConnectionTrackerMakeService::new(app_http.into_make_service(), "HTTP");
        if let Err(e) = axum::serve(listener, make_service).await {
            eprintln!("HTTP server error: {}", e);
        }
    });

    let https_addr = SocketAddr::from(([0, 0, 0, 0], https_port));
    println!("Starting HTTPS server on https://0.0.0.0:{}", https_port);
    let subject_alt_names = vec![
        "localhost".to_string(),
        "127.0.0.1".to_string(),
        "0.0.0.0".to_string(),
    ];
    let cert = rcgen::generate_simple_self_signed(subject_alt_names)?;
    let cert_pem = cert.cert.pem();
    let key_pem = cert.key_pair.serialize_pem();
    let rustls_config = axum_server::tls_rustls::RustlsConfig::from_pem(
        cert_pem.into_bytes(),
        key_pem.into_bytes(),
    )
    .await?;

    let app_https = app;
    let https_handle = tokio::spawn(async move {
        let make_service = ConnectionTrackerMakeService::new(app_https.into_make_service(), "HTTPS");
        if let Err(e) = axum_server::bind_rustls(https_addr, rustls_config)
            .serve(make_service)
            .await
        {
            eprintln!("HTTPS server error: {}", e);
        }
    });

    tokio::select! {
        _ = http_handle => {},
        _ = https_handle => {},
    }

    Ok(())
}
