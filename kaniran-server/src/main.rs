//! `kaniran-server` binary: read config, load the rkyv-backed context, and
//! serve the HTTP API until a shutdown signal arrives.

use anyhow::Context;
use envconfig::Envconfig;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_server::{build_router, AppState, Config};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::init_from_env().context("reading configuration from the environment")?;
    init_tracing(&config.log);

    tracing::info!(database_url = %config.database_url, "loading snapshot, this may take a moment…");
    let ctx = KaniranContext::from_url(&config.database_url)
        .map_err(|err| anyhow::anyhow!("loading context from `{}`: {err}", config.database_url))?;

    let state = AppState {
        ctx,
        default_limit: config.default_limit,
    };
    let app = build_router(state);

    let addr = config.socket_addr();
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("binding {addr}"))?;
    tracing::info!(%addr, "kaniran-server listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("serving HTTP")?;
    Ok(())
}

/// `RUST_LOG` wins; otherwise fall back to the configured default filter.
fn init_tracing(default_filter: &str) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter));
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}

/// Resolve on Ctrl-C or `SIGTERM` so axum can drain in-flight requests.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("install Ctrl-C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    tracing::info!("shutdown signal received");
}
