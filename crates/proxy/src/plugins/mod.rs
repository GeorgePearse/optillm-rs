//! Ports of upstream `optillm/plugins/*` (v0.4.0).

use anyhow::anyhow;
use serde_json::Value;

use crate::approaches::{ApproachResult, Ctx};

pub mod compact;
pub mod deepthink;
pub mod genselect;
pub mod majority_voting;

/// Run the plugin registered under `slug` (upstream `plugin_approaches[slug]`).
pub async fn run(
    slug: &str,
    system_prompt: &str,
    initial_query: &str,
    ctx: &Ctx,
) -> ApproachResult {
    match slug {
        "majority_voting" => majority_voting::run(system_prompt, initial_query, ctx).await,
        "compact" => compact::run(system_prompt, initial_query, ctx).await,
        "genselect" => genselect::run(system_prompt, initial_query, ctx).await,
        "deepthink" => deepthink::run(system_prompt, initial_query, ctx).await,
        other => Err(anyhow!(
            "Plugin '{}' is not yet implemented in optillm-rs",
            other
        )),
    }
}

/// Model listing for `--approach proxy` (filled in with the proxy plugin).
pub async fn proxy_models() -> Option<Value> {
    None
}
