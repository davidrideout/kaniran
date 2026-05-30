//! Rust-only sidecar: process-level configuration for kaniran.
//!
//! Layered load (`kaniran.toml` in CWD overlaid by env vars) via the
//! [`config`] crate. Diverges from upstream `*connection-env-var*` +
//! `get-ichiran-connection-env`: the env var is `DATABASE_URL` and
//! the value is a Postgres URL string, not a Lisp connection-spec
//! list parsed by `read-from-string`.

use crate::conn::kani_context::Error;
use config::{Config, ConfigError, Environment, File};

const DATABASE_URL_ENV: &str = "DATABASE_URL";

#[derive(Debug, Clone)]
pub struct KaniConfig {
    pub database_url: String,
}

impl KaniConfig {
    pub fn from_env() -> Result<Self, Error> {
        let cfg = Config::builder()
            .add_source(File::with_name("kaniran").required(false))
            .add_source(Environment::default())
            .build()?;
        Self::from_config(&cfg)
    }

    fn from_config(cfg: &Config) -> Result<Self, Error> {
        let key = DATABASE_URL_ENV.to_ascii_lowercase();
        match cfg.get_string(&key) {
            Ok(s) if !s.trim().is_empty() => Ok(Self { database_url: s }),
            Ok(_) | Err(ConfigError::NotFound(_)) => {
                Err(Error::MissingConnection(DATABASE_URL_ENV))
            }
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with(database_url: Option<&str>) -> Config {
        let builder = Config::builder();
        let builder = match database_url {
            Some(v) => builder
                .set_override(DATABASE_URL_ENV.to_ascii_lowercase(), v)
                .unwrap(),
            None => builder,
        };
        builder.build().unwrap()
    }

    #[test]
    fn unset_empty_and_whitespace_all_error_missing_connection() {
        for override_value in [None, Some(""), Some("   ")] {
            let cfg = config_with(override_value);
            let err = KaniConfig::from_config(&cfg).unwrap_err();
            assert!(
                matches!(err, Error::MissingConnection(name) if name == DATABASE_URL_ENV),
                "override={override_value:?}",
            );
        }
    }

    #[test]
    fn from_config_populates_database_url() {
        let url = "postgres://postgres@localhost/jmdict?sslmode=disable";
        let cfg = config_with(Some(url));
        let kc = KaniConfig::from_config(&cfg).unwrap();
        assert_eq!(kc.database_url, url);
    }
}
