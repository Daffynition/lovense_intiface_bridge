use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use buttplug_client::device::ClientDeviceOutputCommand;
use buttplug_client::ButtplugClientDevice;
use buttplug_core::message::{InputType, OutputType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LovenseResponse<T: Serialize> {
    pub code: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub response_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl LovenseResponse<serde_json::Value> {
    pub fn ok_simple() -> Self {
        Self {
            code: 200,
            message: None,
            response_type: Some("ok".to_string()),
            data: None,
        }
    }

    pub fn error(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: Some(message.into()),
            response_type: None,
            data: None,
        }
    }
}

impl<T: Serialize> LovenseResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            code: 200,
            message: None,
            response_type: Some("OK".to_string()),
            data: Some(data),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GetToysData {
    pub toys: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ToyInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub battery: i32,
    pub nick_name: String,
    pub version: String,
    pub short_function_names: Vec<String>,
    pub full_function_names: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PatternAction {
    #[serde(default)]
    pub ts: u64,
    #[serde(default)]
    pub pos: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct LovenseCommandRequest {
    #[serde(alias = "Command", alias = "cmd")]
    pub command: Option<String>,
    #[serde(alias = "type", alias = "Type")]
    pub req_type: Option<String>,
    #[serde(alias = "action", alias = "Action")]
    pub action: Option<String>,
    #[serde(alias = "toy", alias = "Toy")]
    pub toy: Option<serde_json::Value>,
    #[serde(alias = "value", alias = "Value")]
    pub value: Option<serde_json::Value>,
    #[serde(alias = "name", alias = "Name")]
    pub name: Option<String>,
    #[serde(alias = "rule", alias = "Rule")]
    pub rule: Option<String>,
    #[serde(alias = "strength", alias = "Strength")]
    pub strength: Option<String>,
    #[serde(alias = "actions", alias = "Actions")]
    pub actions: Option<Vec<PatternAction>>,
    #[serde(alias = "sec", alias = "Sec", alias = "time", alias = "timeSec")]
    pub time_sec: Option<f64>,
    #[serde(alias = "loopRunningSec")]
    pub loop_running_sec: Option<f64>,
    #[serde(alias = "loopPauseSec")]
    pub loop_pause_sec: Option<f64>,
    #[serde(alias = "startTime")]
    pub start_time: Option<u64>,
    #[serde(alias = "offsetTime")]
    pub offset_time: Option<u64>,
    #[serde(alias = "timeMs")]
    pub time_ms: Option<f64>,
    #[serde(alias = "stopPrevious")]
    pub stop_previous: Option<i32>,
    #[serde(alias = "apiVer")]
    pub api_ver: Option<serde_json::Value>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl LovenseCommandRequest {
    pub fn get_target_toys(&self) -> Vec<String> {
        match &self.toy {
            Some(serde_json::Value::String(s)) => {
                if s.trim().is_empty() {
                    Vec::new()
                } else {
                    vec![s.trim().to_string()]
                }
            }
            Some(serde_json::Value::Array(arr)) => arr
                .iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            _ => Vec::new(),
        }
    }
}

/// Generates a deterministic 12-character hex toy ID for a Buttplug device.
pub fn generate_toy_id(device: &ButtplugClientDevice) -> String {
    let mut hasher = DefaultHasher::new();
    device.name().hash(&mut hasher);
    device.index().hash(&mut hasher);
    let hash = hasher.finish();
    format!("{:012x}", hash & 0xffffffffffff)
}

/// Extracts capability function names from a Buttplug device.
pub fn get_device_functions(device: &ButtplugClientDevice) -> (Vec<String>, Vec<String>) {
    let mut short_names = Vec::new();
    let mut full_names = Vec::new();

    if device.output_available(OutputType::Vibrate) {
        short_names.push("v".to_string());
        full_names.push("Vibrate".to_string());
    }
    if device.output_available(OutputType::Rotate) {
        short_names.push("r".to_string());
        full_names.push("Rotate".to_string());
    }
    if device.output_available(OutputType::Oscillate) {
        short_names.push("o".to_string());
        full_names.push("Oscillate".to_string());
    }
    if device.output_available(OutputType::Position) {
        short_names.push("p".to_string());
        full_names.push("Position".to_string());
    }

    if short_names.is_empty() {
        short_names.push("v".to_string());
        full_names.push("Vibrate".to_string());
    }

    (short_names, full_names)
}

/// Converts a ButtplugClientDevice to a ToyInfo struct.
pub async fn device_to_toy_info(device: &ButtplugClientDevice) -> ToyInfo {
    let id = generate_toy_id(device);
    let name = device.name().to_lowercase();
    let status = "1".to_string();

    let battery = if device.input_available(InputType::Battery) {
        match device.battery().await {
            Ok(b) => b as i32,
            Err(_) => 100,
        }
    } else {
        100
    }
    .clamp(0, 100);

    let (short_function_names, full_function_names) = get_device_functions(device);

    ToyInfo {
        id,
        name,
        status,
        battery,
        nick_name: String::new(),
        version: String::new(),
        short_function_names,
        full_function_names,
    }
}

/// Builds the GetToysData response from a collection of devices.
pub async fn build_get_toys_response(devices: &[Arc<ButtplugClientDevice>]) -> GetToysData {
    let mut toys = HashMap::new();
    for device in devices {
        let info = device_to_toy_info(device).await;
        toys.insert(info.id.clone(), info);
    }
    let toys_json = serde_json::to_string(&toys).unwrap_or_else(|_| "{}".to_string());
    GetToysData {
        toys: toys_json,
        platform: Some("ios".to_string()),
        app_type: Some("remote".to_string()),
    }
}

/// Helper to control device outputs
pub async fn apply_device_strength(device: &ButtplugClientDevice, feature: &str, strength_ratio: f64) {
    let ratio = strength_ratio.clamp(0.0, 1.0);
    match feature.to_lowercase().as_str() {
        "v" | "vibrate" => {
            if device.output_available(OutputType::Vibrate) {
                let _ = device.run_output(&ClientDeviceOutputCommand::Vibrate(ratio.into())).await;
            }
        }
        "r" | "rotate" => {
            if device.output_available(OutputType::Rotate) {
                let _ = device.run_output(&ClientDeviceOutputCommand::Rotate(ratio.into())).await;
            }
        }
        "o" | "oscillate" | "pump" | "thrusting" | "fingering" | "suction" => {
            if device.output_available(OutputType::Oscillate) {
                let _ = device.run_output(&ClientDeviceOutputCommand::Oscillate(ratio.into())).await;
            } else if device.output_available(OutputType::Vibrate) {
                let _ = device.run_output(&ClientDeviceOutputCommand::Vibrate(ratio.into())).await;
            }
        }
        "p" | "position" | "depth" | "stroke" => {
            if device.output_available(OutputType::Position) {
                let _ = device.run_output(&ClientDeviceOutputCommand::Position(ratio.into())).await;
            } else if device.output_available(OutputType::Oscillate) {
                let _ = device.run_output(&ClientDeviceOutputCommand::Oscillate(ratio.into())).await;
            } else if device.output_available(OutputType::Vibrate) {
                let _ = device.run_output(&ClientDeviceOutputCommand::Vibrate(ratio.into())).await;
            }
        }
        "all" => {
            if device.output_available(OutputType::Vibrate) {
                let _ = device.run_output(&ClientDeviceOutputCommand::Vibrate(ratio.into())).await;
            }
            if device.output_available(OutputType::Rotate) {
                let _ = device.run_output(&ClientDeviceOutputCommand::Rotate(ratio.into())).await;
            }
            if device.output_available(OutputType::Oscillate) {
                let _ = device.run_output(&ClientDeviceOutputCommand::Oscillate(ratio.into())).await;
            }
            if device.output_available(OutputType::Position) {
                let _ = device.run_output(&ClientDeviceOutputCommand::Position(ratio.into())).await;
            }
        }
        "stop" => {
            let _ = device.stop().await;
        }
        _ => {
            if device.output_available(OutputType::Vibrate) {
                let _ = device.run_output(&ClientDeviceOutputCommand::Vibrate(ratio.into())).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toy_info_serialization() {
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

        let toys_json = serde_json::to_string(&toys).unwrap();
        let response = LovenseResponse::ok(GetToysData {
            toys: toys_json,
            platform: Some("ios".to_string()),
            app_type: Some("remote".to_string()),
        });
        let json_value = serde_json::to_value(&response).unwrap();

        assert_eq!(json_value["code"], 200);
        assert_eq!(json_value["type"], "OK");
        assert_eq!(json_value["data"]["platform"], "ios");
        assert_eq!(json_value["data"]["appType"], "remote");

        let inner_toys: HashMap<String, ToyInfo> =
            serde_json::from_str(json_value["data"]["toys"].as_str().unwrap()).unwrap();
        let toy = &inner_toys["ca771e9552be"];
        assert_eq!(toy.id, "ca771e9552be");
        assert_eq!(toy.name, "ferri");
        assert_eq!(toy.status, "1");
        assert_eq!(toy.battery, 86);
        assert_eq!(toy.nick_name, "");
        assert_eq!(toy.version, "");
        assert_eq!(toy.short_function_names, vec!["v"]);
        assert_eq!(toy.full_function_names, vec!["Vibrate"]);
    }

    #[test]
    fn test_parse_command_request() {
        let body = r#"{"command": "GetToys"}"#;
        let req: LovenseCommandRequest = serde_json::from_str(body).unwrap();
        assert_eq!(req.command.as_deref(), Some("GetToys"));

        let body_single_quote = r#"{"command":"GetToys","apiVer":1}"#;
        let req2: LovenseCommandRequest = serde_json::from_str(body_single_quote).unwrap();
        assert_eq!(req2.command.as_deref(), Some("GetToys"));
    }
}
