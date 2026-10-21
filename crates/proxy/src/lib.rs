//! `optillm-proxy`: an OpenAI-compatible optimizing inference proxy that aims
//! for behavioural parity with [codelion/optillm](https://github.com/codelion/optillm)
//! v0.4.0 (commit e4ad19906578c9c472a3f8fd04d218db89a31c8c).
//!
//! Prompts in `approaches/` and `plugins/` are copied verbatim from upstream
//! (Apache-2.0, see NOTICE).

pub mod approaches;
pub mod batching;
pub mod client;
pub mod config;
pub mod conversation_logger;
pub mod parse;
pub mod plugins;
pub mod py;
pub mod server;

use std::sync::Arc;

pub use config::ServerConfig;

/// Upstream release this crate tracks.
pub const UPSTREAM_VERSION: &str = "0.4.0";
/// Upstream commit this crate tracks.
pub const UPSTREAM_COMMIT: &str = "e4ad19906578c9c472a3f8fd04d218db89a31c8c";

/// Build the server state and router for a config (upstream `main()` minus `app.run`).
pub fn build_app(config: ServerConfig) -> (Arc<server::AppState>, axum::Router) {
    let (default_client, api_key) = server::build_default_client(&config);
    let logger = Arc::new(conversation_logger::ConversationLogger::new(
        config.conversation_log_dir.clone().into(),
        config.log_conversations,
    ));
    let batcher = if config.batch_mode {
        Some(Arc::new(batching::Batcher::new(
            config.batch_size,
            config.batch_wait_ms,
        )))
    } else {
        None
    };
    let state = Arc::new(server::AppState {
        config: Arc::new(config),
        default_client,
        api_key,
        logger,
        batcher: batcher.clone(),
    });
    if let Some(b) = &batcher {
        b.set_state(&state);
    }
    let router = server::router(state.clone());
    (state, router)
}

/// Run the proxy until the process is stopped.
pub async fn serve(config: ServerConfig) -> anyhow::Result<()> {
    let addr = format!("{}:{}", config.host, config.port);
    if config.launch_gui {
        tracing::error!(
            "--launch-gui (Gradio) is not supported by optillm-rs; serving the API only"
        );
    }
    tracing::info!("Starting server with approach: {}", config.approach);
    let (_state, router) = build_app(config);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Listening on http://{}", addr);
    axum::serve(listener, router).await?;
    Ok(())
}
