//! `optillm-proxy`: drop-in replacement for upstream's `optillm` server entry point.

use optillm_proxy::ServerConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = ServerConfig::from_args();
    let level = match config.log.as_str() {
        "notset" | "debug" => "debug",
        "warning" => "warn",
        "error" | "critical" => "error",
        _ => "info",
    };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level)),
        )
        .with_writer(std::io::stderr)
        .init();
    optillm_proxy::serve(config).await
}
