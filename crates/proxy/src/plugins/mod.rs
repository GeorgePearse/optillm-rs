//! Ports of upstream `optillm/plugins/*` (v0.4.0).

use anyhow::anyhow;
use serde_json::Value;

use crate::approaches::{ApproachResult, Ctx};

pub mod coc;
pub mod compact;
pub mod deepthink;
pub mod executecode;
pub mod genselect;
pub mod json;
pub mod majority_voting;
pub mod memory;
pub mod privacy;
pub mod pyhelper;
pub mod readurls;
pub mod spl;
pub mod web_search;

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
        "memory" => memory::run(system_prompt, initial_query, ctx).await,
        "spl" => spl::run(system_prompt, initial_query, ctx).await,
        "coc" => coc::run(system_prompt, initial_query, ctx).await,
        "readurls" => readurls::run(system_prompt, initial_query, ctx).await,
        "web_search" => web_search::run(system_prompt, initial_query, ctx).await,
        "json" => json::run(system_prompt, initial_query, ctx).await,
        "privacy" => privacy::run(system_prompt, initial_query, ctx).await,
        "executecode" => executecode::run(system_prompt, initial_query, ctx).await,
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
