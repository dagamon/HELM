pub mod output_logs;
pub mod run_logs;
pub mod scripts;
pub mod services;
pub mod stacks;

use anyhow::Result;
use serde_json::Value;

/// Serialize Option<Vec<...>> or Option<HashMap<...>> to JSON string for TEXT column.
pub fn ser_json<T: serde::Serialize>(v: &Option<T>) -> Result<Option<String>> {
    match v {
        Some(x) => Ok(Some(serde_json::to_string(x)?)),
        None => Ok(None),
    }
}

/// Parse Option<String> as JSON into target type. None → None.
pub fn de_json<T: for<'de> serde::Deserialize<'de>>(s: &Option<String>) -> Result<Option<T>> {
    match s {
        Some(raw) => Ok(Some(serde_json::from_str(raw)?)),
        None => Ok(None),
    }
}

/// Strip volatile/runtime fields and serialize a row's JSON columns out for export.
pub fn row_to_value(row: &impl serde::Serialize) -> Result<Value> {
    Ok(serde_json::to_value(row)?)
}
