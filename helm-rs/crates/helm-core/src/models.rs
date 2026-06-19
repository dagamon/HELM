use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

fn default_health_check_interval() -> i64 {
    30
}
fn default_platform() -> String {
    "all".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceCreate {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub r#type: String,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub venv_path: Option<String>,
    #[serde(default)]
    pub args: Option<Vec<String>>,
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub health_check_url: Option<String>,
    #[serde(default = "default_health_check_interval")]
    pub health_check_interval: i64,
    #[serde(default)]
    pub auto_start: bool,
    #[serde(default)]
    pub restart_on_crash: bool,
    #[serde(default = "default_platform")]
    pub platform: String,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub depends_on: Option<Vec<i64>>,
    #[serde(default)]
    pub webhook_url: Option<String>,
    #[serde(default)]
    pub manifest_path: Option<String>,
    #[serde(default)]
    pub binary_path: Option<String>,
    #[serde(default)]
    pub cargo_profile: Option<String>,
    #[serde(default)]
    pub cargo_features: Option<String>,
    #[serde(default)]
    pub prebuild: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServiceUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub venv_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_check_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_check_interval: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_start: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restart_on_crash: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cargo_profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cargo_features: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prebuild: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceResponse {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub r#type: String,
    pub command: Option<String>,
    pub cwd: Option<String>,
    pub venv_path: Option<String>,
    pub args: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
    pub url: Option<String>,
    pub health_check_url: Option<String>,
    pub health_check_interval: i64,
    pub auto_start: bool,
    pub restart_on_crash: bool,
    pub platform: String,
    pub tags: Option<Vec<String>>,
    pub depends_on: Option<Vec<i64>>,
    pub webhook_url: Option<String>,
    pub manifest_path: Option<String>,
    pub binary_path: Option<String>,
    pub cargo_profile: Option<String>,
    pub cargo_features: Option<String>,
    pub prebuild: bool,
    pub created_at: String,
    pub updated_at: String,
    pub status: String,
    pub pid: Option<i64>,
}

// ---------------------------------------------------------------------------
// Script
// ---------------------------------------------------------------------------

fn default_run_mode() -> String {
    "exec".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptCreate {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub command: String,
    #[serde(default = "default_run_mode")]
    pub run_mode: String,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub args: Option<Vec<String>>,
    #[serde(default = "default_platform")]
    pub platform: String,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub cron_schedule: Option<String>,
    #[serde(default)]
    pub cron_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScriptUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cron_schedule: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cron_enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptResponse {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub command: String,
    pub run_mode: String,
    pub cwd: Option<String>,
    pub args: Option<Vec<String>>,
    pub platform: String,
    pub tags: Option<Vec<String>>,
    pub cron_schedule: Option<String>,
    pub cron_enabled: bool,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// Run log
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunLog {
    pub id: i64,
    pub entity_type: String,
    pub entity_id: i64,
    pub started_at: String,
    pub stopped_at: Option<String>,
    pub exit_code: Option<i64>,
    pub status: Option<String>,
    pub pid: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputLogEntry {
    pub stream: String,
    pub line: String,
    pub ts: String,
}

// ---------------------------------------------------------------------------
// FAQ
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaqArticle {
    pub slug: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaqArticleContent {
    pub slug: String,
    pub title: String,
    pub content: String,
}

// ---------------------------------------------------------------------------
// Export / Import
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportPayload {
    pub version: i64,
    pub services: Vec<serde_json::Value>,
    pub scripts: Vec<serde_json::Value>,
}
