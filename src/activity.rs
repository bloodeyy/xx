// ============================================
// Activity Helpers
// ============================================

use serde_json::Value;

#[allow(dead_code)]
pub fn extract_str(config: &Value, key: &str) -> Option<String> {
    config
        .get(key)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

#[allow(dead_code)]
pub fn extract_bool(config: &Value, key: &str) -> bool {
    config.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
}