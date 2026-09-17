use std::collections::HashMap;
use std::sync::Arc;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use buttplug_client::ButtplugClient;
use http_body_util::BodyExt;
use serde_json::Value;

use lovense_intiface_bridge::lovense::{GetToysData, LovenseResponse, ToyInfo};
use lovense_intiface_bridge::server::{create_router, AppState};

#[tokio::test]
async fn test_get_toys_empty() {
    let client = Arc::new(ButtplugClient::new("TestClient"));
    let state = AppState::new(client);
    let app = create_router(state);

    let request = Request::builder()
        .uri("/command")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"command": "GetToys"}"#))
        .unwrap();

    let response = tower::ServiceExt::oneshot(app, request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["code"], 200);
    assert_eq!(json["type"], "OK");
    assert_eq!(json["data"]["toys"], "{}");
    assert_eq!(json["data"]["platform"], "ios");
    assert_eq!(json["data"]["appType"], "remote");
}

#[tokio::test]
async fn test_get_toys_get_method() {
    let client = Arc::new(ButtplugClient::new("TestClient"));
    let state = AppState::new(client);
    let app = create_router(state);

    let request = Request::builder()
        .uri("/command?command=GetToys")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = tower::ServiceExt::oneshot(app, request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["code"], 200);
    assert_eq!(json["type"], "OK");
    assert_eq!(json["data"]["toys"], "{}");
}

#[tokio::test]
async fn test_get_toy_name() {
    let client = Arc::new(ButtplugClient::new("TestClient"));
    let state = AppState::new(client);
    let app = create_router(state);

    let request = Request::builder()
        .uri("/command")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"command": "GetToyName"}"#))
        .unwrap();

    let response = tower::ServiceExt::oneshot(app, request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["code"], 200);
    assert_eq!(json["type"], "OK");
    assert!(json["data"].is_array());
}

#[tokio::test]
async fn test_function_command() {
    let client = Arc::new(ButtplugClient::new("TestClient"));
    let state = AppState::new(client);
    let app = create_router(state);

    let request = Request::builder()
        .uri("/command")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(r#"{
            "command": "Function",
            "action": "Vibrate:16",
            "timeSec": 20,
            "loopRunningSec": 9,
            "loopPauseSec": 4,
            "toy": "ff922f7fd345",
            "apiVer": 1
        }"#))
        .unwrap();

    let response = tower::ServiceExt::oneshot(app, request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["code"], 200);
    assert_eq!(json["type"], "ok");
}

#[tokio::test]
async fn test_position_command() {
    let client = Arc::new(ButtplugClient::new("TestClient"));
    let state = AppState::new(client);
    let app = create_router(state);

    let request = Request::builder()
        .uri("/command")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(r#"{
            "command": "Position",
            "value": "38",
            "toy": "ff922f7fd345",
            "apiVer": 1
        }"#))
        .unwrap();

    let response = tower::ServiceExt::oneshot(app, request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["code"], 200);
    assert_eq!(json["type"], "ok");
}

#[tokio::test]
async fn test_pattern_command() {
    let client = Arc::new(ButtplugClient::new("TestClient"));
    let state = AppState::new(client);
    let app = create_router(state);

    let request = Request::builder()
        .uri("/command")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(r#"{
            "command": "Pattern",
            "rule": "V:1;F:v,r,p;S:100#",
            "strength": "20;20;5;20;10",
            "timeSec": 9,
            "apiVer": 2
        }"#))
        .unwrap();

    let response = tower::ServiceExt::oneshot(app, request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["code"], 200);
    assert_eq!(json["type"], "ok");
}

#[tokio::test]
async fn test_pattern_v2_all_types() {
    let client = Arc::new(ButtplugClient::new("TestClient"));
    let state = AppState::new(client);
    let app = create_router(state);

    // 1. Setup
    let req_setup = Request::builder()
        .uri("/command")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(r#"{
            "command": "PatternV2",
            "type": "Setup",
            "actions": [
                { "ts": 0, "pos": 10 },
                { "ts": 100, "pos": 100 }
            ],
            "apiVer": 1
        }"#))
        .unwrap();
    let res = tower::ServiceExt::oneshot(app.clone(), req_setup).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 2. Play
    let req_play = Request::builder()
        .uri("/command")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(r#"{
            "command": "PatternV2",
            "type": "Play",
            "startTime": 0,
            "offsetTime": 0,
            "apiVer": 1
        }"#))
        .unwrap();
    let res = tower::ServiceExt::oneshot(app.clone(), req_play).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 3. InitPlay
    let req_initplay = Request::builder()
        .uri("/command")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(r#"{
            "command": "PatternV2",
            "type": "InitPlay",
            "actions": [{ "ts": 0, "pos": 10 }],
            "apiVer": 1
        }"#))
        .unwrap();
    let res = tower::ServiceExt::oneshot(app.clone(), req_initplay).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 4. Stop
    let req_stop = Request::builder()
        .uri("/command")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(r#"{
            "command": "PatternV2",
            "type": "Stop",
            "apiVer": 1
        }"#))
        .unwrap();
    let res = tower::ServiceExt::oneshot(app.clone(), req_stop).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 5. SyncTime
    let req_sync = Request::builder()
        .uri("/command")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(r#"{
            "command": "PatternV2",
            "type": "SyncTime",
            "apiVer": 1
        }"#))
        .unwrap();
    let res = tower::ServiceExt::oneshot(app.clone(), req_sync).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_preset_command() {
    let client = Arc::new(ButtplugClient::new("TestClient"));
    let state = AppState::new(client);
    let app = create_router(state);

    let request = Request::builder()
        .uri("/command")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(r#"{
            "command": "Preset",
            "name": "pulse",
            "timeSec": 9,
            "toy": "ff922f7fd345",
            "apiVer": 1
        }"#))
        .unwrap();

    let response = tower::ServiceExt::oneshot(app, request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["code"], 200);
    assert_eq!(json["type"], "ok");
}

#[tokio::test]
async fn test_lan_v2_command_url() {
    let client = Arc::new(ButtplugClient::new("TestClient"));
    let state = AppState::new(client);
    let app = create_router(state);

    let request = Request::builder()
        .uri("/api/lan/v2/command")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(r#"{
            "token": "testtoken",
            "uid": "123",
            "command": "Function",
            "action": "Vibrate:10",
            "timeSec": 5,
            "apiVer": 1
        }"#))
        .unwrap();

    let response = tower::ServiceExt::oneshot(app, request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_unknown_command() {
    let client = Arc::new(ButtplugClient::new("TestClient"));
    let state = AppState::new(client);
    let app = create_router(state);

    let request = Request::builder()
        .uri("/command")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"command": "NonExistentCommand"}"#))
        .unwrap();

    let response = tower::ServiceExt::oneshot(app, request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["code"], 400);
    assert!(json["message"].is_string());
}

#[tokio::test]
async fn test_toy_info_exact_schema() {
    let mut toys = HashMap::new();
    toys.insert(
        "ca771e9552be".to_string(),
        ToyInfo {
            id: "ca771e9552be".to_string(),
            name: "ferri".to_string(),
            status: "1".to_string(),
            battery: 86,
            nick_name: "".to_string(),
            version: "".to_string(),
            short_function_names: vec!["v".to_string()],
            full_function_names: vec!["Vibrate".to_string()],
        },
    );
    toys.insert(
        "286847ae60ca".to_string(),
        ToyInfo {
            id: "286847ae60ca".to_string(),
            name: "lush".to_string(),
            status: "1".to_string(),
            battery: 100,
            nick_name: "".to_string(),
            version: "4".to_string(),
            short_function_names: vec!["v".to_string()],
            full_function_names: vec!["Vibrate".to_string()],
        },
    );

    let toys_json = serde_json::to_string(&toys).unwrap();
    let response = LovenseResponse::ok(GetToysData {
        toys: toys_json,
        platform: Some("ios".to_string()),
        app_type: Some("remote".to_string()),
    });
    let serialized = serde_json::to_string(&response).unwrap();
    let val: Value = serde_json::from_str(&serialized).unwrap();

    assert_eq!(val["code"], 200);
    assert_eq!(val["type"], "OK");
    assert_eq!(val["data"]["platform"], "ios");
    assert_eq!(val["data"]["appType"], "remote");

    let inner_toys: HashMap<String, ToyInfo> =
        serde_json::from_str(val["data"]["toys"].as_str().unwrap()).unwrap();
    let ferri = &inner_toys["ca771e9552be"];
    assert_eq!(ferri.id, "ca771e9552be");
    assert_eq!(ferri.name, "ferri");
    assert_eq!(ferri.status, "1");
    assert_eq!(ferri.battery, 86);
    assert_eq!(ferri.nick_name, "");
    assert_eq!(ferri.version, "");
    assert_eq!(ferri.short_function_names, vec!["v"]);
    assert_eq!(ferri.full_function_names, vec!["Vibrate"]);

    let lush = &inner_toys["286847ae60ca"];
    assert_eq!(lush.id, "286847ae60ca");
    assert_eq!(lush.name, "lush");
    assert_eq!(lush.status, "1");
    assert_eq!(lush.battery, 100);
    assert_eq!(lush.nick_name, "");
    assert_eq!(lush.version, "4");
    assert_eq!(lush.short_function_names, vec!["v"]);
    assert_eq!(lush.full_function_names, vec!["Vibrate"]);
}

#[tokio::test]
async fn test_tls_config_generation() {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let subject_alt_names = vec![
        "localhost".to_string(),
        "127.0.0.1".to_string(),
        "0.0.0.0".to_string(),
    ];
    let cert = rcgen::generate_simple_self_signed(subject_alt_names).unwrap();
    let cert_pem = cert.cert.pem();
    let key_pem = cert.key_pair.serialize_pem();

    let rustls_config = axum_server::tls_rustls::RustlsConfig::from_pem(
        cert_pem.into_bytes(),
        key_pem.into_bytes(),
    )
    .await;

    assert!(rustls_config.is_ok());
}

#[tokio::test]
async fn test_http_connection_tracking() {
    use lovense_intiface_bridge::server::ConnectionTrackerMakeService;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let client = Arc::new(ButtplugClient::new("TestClient"));
    let state = AppState::new(client);
    let app = create_router(state);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let local_addr = listener.local_addr().unwrap();

    let make_service = ConnectionTrackerMakeService::new(app.into_make_service(), "HTTP");

    let server_handle = tokio::spawn(async move {
        axum::serve(listener, make_service).await.unwrap();
    });

    let mut stream = tokio::net::TcpStream::connect(local_addr).await.unwrap();
    let request_data = b"POST /command HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: 22\r\n\r\n{\"command\": \"GetToys\"}";
    stream.write_all(request_data).await.unwrap();

    let mut response_buf = vec![0u8; 1024];
    let n = stream.read(&mut response_buf).await.unwrap();
    assert!(n > 0);
    let response_str = String::from_utf8_lossy(&response_buf[..n]);
    assert!(response_str.contains("200 OK"));

    // Drop stream to close connection
    drop(stream);
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    server_handle.abort();
}

#[tokio::test]
async fn test_https_connection_tracking() {
    use lovense_intiface_bridge::server::ConnectionTrackerMakeService;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let _ = rustls::crypto::ring::default_provider().install_default();

    let client = Arc::new(ButtplugClient::new("TestClient"));
    let state = AppState::new(client);
    let app = create_router(state);

    let subject_alt_names = vec![
        "localhost".to_string(),
        "127.0.0.1".to_string(),
    ];
    let cert = rcgen::generate_simple_self_signed(subject_alt_names).unwrap();
    let cert_pem = cert.cert.pem();
    let key_pem = cert.key_pair.serialize_pem();

    let rustls_config = axum_server::tls_rustls::RustlsConfig::from_pem(
        cert_pem.into_bytes(),
        key_pem.into_bytes(),
    )
    .await
    .unwrap();

    let https_addr = std::net::SocketAddr::from(([127, 0, 0, 1], 0));
    let server = axum_server::bind_rustls(https_addr, rustls_config);
    let handle = axum_server::Handle::new();
    let handle_clone = handle.clone();

    let make_service = ConnectionTrackerMakeService::new(app.into_make_service(), "HTTPS");

    let server_handle = tokio::spawn(async move {
        server.handle(handle_clone).serve(make_service).await.unwrap();
    });

    let local_addr = handle.listening().await.unwrap();

    let mut root_cert_store = rustls::RootCertStore::empty();
    root_cert_store.add(cert.cert.der().clone()).unwrap();

    let client_config = rustls::ClientConfig::builder()
        .with_root_certificates(root_cert_store)
        .with_no_client_auth();

    let connector = tokio_rustls::TlsConnector::from(Arc::new(client_config));
    let tcp_stream = tokio::net::TcpStream::connect(local_addr).await.unwrap();
    let domain = rustls::pki_types::ServerName::try_from("localhost".to_string()).unwrap();
    let mut tls_stream = connector.connect(domain, tcp_stream).await.unwrap();

    let request_data = b"POST /command HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: 22\r\n\r\n{\"command\": \"GetToys\"}";
    tls_stream.write_all(request_data).await.unwrap();

    let mut response_buf = vec![0u8; 1024];
    let n = tls_stream.read(&mut response_buf).await.unwrap();
    assert!(n > 0);
    let response_str = String::from_utf8_lossy(&response_buf[..n]);
    assert!(response_str.contains("200 OK"));

    drop(tls_stream);
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    server_handle.abort();
}
