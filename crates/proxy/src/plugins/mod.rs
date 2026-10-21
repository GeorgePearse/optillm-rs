//! Ports of upstream `optillm/plugins/*`.

use serde_json::Value;

/// Model listing for `--approach proxy` (filled in with the proxy plugin).
pub async fn proxy_models() -> Option<Value> {
    None
}
