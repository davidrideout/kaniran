//! Server configuration, read from the environment via [`envconfig`].
//!
//! Every value is overridable by an env var; only `DATABASE_URL` is
//! required. The struct is handed to the binary's startup path, which
//! passes [`Config::database_url`] into the kaniran context creator
//! ([`kaniran_core::conn::kani_context::KaniranContext::from_url`]).

use std::net::{IpAddr, SocketAddr};

use envconfig::Envconfig;

/// Runtime configuration sourced from environment variables.
#[derive(Envconfig, Debug, Clone)]
pub struct Config {
    /// rkyv snapshot URL handed to the context creator, e.g.
    /// `memory://corpus/kaniran_ichiran_latest_2026_06_10.rkyv`. Required.
    #[envconfig(from = "DATABASE_URL")]
    pub database_url: String,

    /// Interface to bind. Defaults to all interfaces, which is what a
    /// container wants; set `127.0.0.1` to keep it loopback-only.
    #[envconfig(from = "KANIRAN_HTTP_HOST", default = "0.0.0.0")]
    pub http_host: IpAddr,

    /// TCP port to bind. Defaults to 3000.
    #[envconfig(from = "KANIRAN_HTTP_PORT", default = "3000")]
    pub http_port: u16,

    /// Segmentation beam width applied when a request omits `limit`.
    /// Matches the `segmentation_limit` default; the beam width affects
    /// the JSON formats.
    #[envconfig(from = "KANIRAN_DEFAULT_LIMIT", default = "5")]
    pub default_limit: usize,

    /// `tracing` filter directive used only when `RUST_LOG` is unset.
    #[envconfig(from = "KANIRAN_LOG", default = "info")]
    pub log: String,
}

impl Config {
    /// The address the HTTP listener binds to.
    #[must_use]
    pub fn socket_addr(&self) -> SocketAddr {
        SocketAddr::new(self.http_host, self.http_port)
    }
}
