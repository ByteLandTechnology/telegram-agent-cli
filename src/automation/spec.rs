use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioSpec {
    pub name: String,
    #[serde(default)]
    pub accounts: BTreeMap<String, String>,
    #[serde(default)]
    pub targets: BTreeMap<String, String>,
    pub steps: Vec<ScenarioStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ScenarioStep {
    Send { send: SendStep },
    Wait { wait: WaitStep },
    Cleanup { cleanup: CleanupStep },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendStep {
    #[serde(rename = "as")]
    pub as_ref: String,
    pub to: String,
    pub text: String,
    #[serde(default)]
    pub save_as: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitStep {
    #[serde(rename = "as")]
    pub as_ref: String,
    pub chat: String,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub text_contains: Option<String>,
    #[serde(default = "default_timeout")]
    pub timeout: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupStep {
    #[serde(rename = "as")]
    pub as_ref: String,
    pub chat: String,
    pub saved_message: String,
}

fn default_timeout() -> String {
    "20s".into()
}
