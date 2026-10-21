//! Ports of upstream `optillm/plugins/*` (v0.4.0).

use anyhow::anyhow;
use serde_json::Value;

use crate::approaches::{ApproachResult, Ctx};

pub mod coc;
pub mod compact;
pub mod deep_research;
pub mod deepthink;
pub mod executecode;
pub mod genselect;
pub mod json;
pub mod majority_voting;
pub mod mcp;
pub mod memory;
pub mod privacy;
pub mod proxy;
pub mod pyhelper;
pub mod readurls;
pub mod router;
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
        "router" => router::run(system_prompt, initial_query, ctx).await,
        "proxy" => proxy::run(system_prompt, initial_query, ctx).await,
        "executecode" => executecode::run(system_prompt, initial_query, ctx).await,
        "mcp" => mcp::run(system_prompt, initial_query, ctx).await,
        "deep_research" => deep_research::run(system_prompt, initial_query, ctx).await,
        // Upstream v0.4.0 loads longcepo/main.py as a top-level module, so its
        // relative imports fail before any work: every request errors.
        "longcepo" => Err(anyhow!(
            "attempted relative import with no known parent package"
        )),
        other => Err(anyhow!(
            "Plugin '{}' is not yet implemented in optillm-rs",
            other
        )),
    }
}

/// Model listing for `--approach proxy`: the providers' models (and
/// model_map aliases) when the proxy config has providers.
pub async fn proxy_models() -> Option<Value> {
    let config = proxy::load_config();
    if config["providers"]
        .as_array()
        .map(|a| a.is_empty())
        .unwrap_or(true)
    {
        return None;
    }
    Some(serde_json::json!({"object": "list", "data": proxy::list_provider_models(&config).await}))
}
