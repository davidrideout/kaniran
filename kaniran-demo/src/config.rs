//! Environment configuration.
//!
//! The dictionary itself is *not* configured here: [`KaniranContext::from_env`]
//! resolves `DATABASE_URL` (with `kaniran.toml` layered underneath), same as
//! the CLI.
//!
//! [`KaniranContext::from_env`]: kaniran_core::conn::kani_context::KaniranContext::from_env

use envconfig::Envconfig;

/// All `KANI_DEMO_*` knobs, each with a workable default.
#[derive(Debug, Envconfig)]
pub struct Config {
    /// Bind interface.
    #[envconfig(from = "KANI_DEMO_HOST", default = "127.0.0.1")]
    pub host: String,
    /// Bind port.
    #[envconfig(from = "KANI_DEMO_PORT", default = "3000")]
    pub port: u16,
    /// Segmentation beam width.
    #[envconfig(from = "KANI_DEMO_LIMIT", default = "5")]
    pub limit: usize,
    /// `tracing` filter used when `RUST_LOG` is unset.
    #[envconfig(from = "KANI_DEMO_LOG", default = "info")]
    pub log: String,
    /// Static-assets directory; defaults to the crate's `static/`.
    #[envconfig(from = "KANI_DEMO_STATIC")]
    pub static_dir: Option<String>,
    /// Template directory; defaults to the crate's `templates/`.
    #[envconfig(from = "KANI_DEMO_TEMPLATES")]
    pub template_dir: Option<String>,
}

impl Config {
    /// `host:port` for the TCP listener.
    pub fn socket_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Static-assets directory, falling back to the in-repo `static/`.
    pub fn static_dir(&self) -> String {
        self.static_dir
            .clone()
            .unwrap_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/static").to_owned())
    }

    /// Template directory, falling back to the in-repo `templates/`.
    pub fn template_dir(&self) -> String {
        self.template_dir
            .clone()
            .unwrap_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/templates").to_owned())
    }
}
