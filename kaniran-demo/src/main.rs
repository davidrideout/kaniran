//! `kaniran-demo` binary: read config, load the rkyv-backed context and the
//! embedded difficulty tables, and serve the reader until a shutdown signal
//! arrives.

use std::sync::Arc;

use anyhow::Context;
use envconfig::Envconfig;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_demo::{build_router, AppState, Config, Levels, Templates};

// mimalloc over the system allocator — same allocation-bound segmentation
// pipeline the CLI runs, where it measured a large end-to-end win.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::init_from_env().context("reading configuration from the environment")?;
    init_tracing(&config.log);

    tracing::info!("loading dictionary snapshot…");
    // The dictionary archive is selected by DATABASE_URL / kaniran.toml
    // (a `memory://<archive>.rkyv` URL), same as the CLI.
    let ctx = KaniranContext::from_env().map_err(|err| {
        anyhow::anyhow!(
            "opening the dictionary: {err} — set DATABASE_URL to a \
             memory://<archive>.rkyv path (or add kaniran.toml)"
        )
    })?;
    let levels = Arc::new(Levels::load());
    tracing::info!("dictionary and difficulty tables loaded");

    let state = AppState {
        ctx,
        levels,
        templates: Arc::new(Templates::new(config.template_dir())),
        limit: config.limit,
    };
    let app = build_router(state, &config.static_dir());

    let addr = config.socket_addr();
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("binding {addr}"))?;
    tracing::info!(%addr, "kaniran-demo listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("serving HTTP")?;
    Ok(())
}

/// `RUST_LOG` wins; otherwise fall back to the configured default filter.
fn init_tracing(default_filter: &str) {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter));
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
