pub mod error;
pub mod models;

pub fn entity_key(entity_type: &str, entity_id: i64) -> String {
    format!("{}_{}", entity_type, entity_id)
}
