//! Port of `ichiran/conn:get-ichiran-connection-env` (`conn.lisp:154-166`).
//!
//! Read the database URL from the configured env var via the [`config`]
//! crate. Returns [`None`] when unset or empty; surfaces a malformed
//! read as [`Error::Config`].
//!
//! Divergences from Lisp:
//! - Returns `Option<String>` (a Postgres URL) rather than a parsed
//!   Lisp connection list. Format change is documented on
//!   [`super::_star_connection_env_var_star_::DATABASE_URL`].
//! - Read goes through [`config::Config`] / [`config::Environment`] so
//!   a future config-file source dropping the same key (`kaniran.toml`
//!   or similar) layers in without changing call sites.
//! - Upstream silently warns and returns nil on parse failure; Rust
//!   bubbles parse failure as an error and reserves [`None`] for the
//!   "unset / empty" case alone.

use crate::conn::_star_connection_env_var_star_::DATABASE_URL;
use crate::conn::kani_context::Error;
use config::{Config, ConfigError, Environment};

pub fn get_ichiran_connection_env() -> Result<Option<String>, Error> {
    let cfg = Config::builder()
        .add_source(Environment::default())
        .build()?;
    let key = DATABASE_URL.to_ascii_lowercase();
    match cfg.get_string(&key) {
        Ok(s) if s.trim().is_empty() => Ok(None),
        Ok(s) => Ok(Some(s)),
        Err(ConfigError::NotFound(_)) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Env-var mutation isn't thread-safe; tests serialize on this lock
    // so cargo's parallel test runner doesn't race them against each
    // other.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_env<F: FnOnce()>(value: Option<&str>, f: F) {
        let _guard = ENV_LOCK.lock().unwrap();
        let prior = std::env::var(DATABASE_URL).ok();
        match value {
            Some(v) => std::env::set_var(DATABASE_URL, v),
            None => std::env::remove_var(DATABASE_URL),
        }
        f();
        match prior {
            Some(v) => std::env::set_var(DATABASE_URL, v),
            None => std::env::remove_var(DATABASE_URL),
        }
    }

    #[test]
    fn unset_yields_none() {
        with_env(None, || {
            assert_eq!(get_ichiran_connection_env().unwrap(), None);
        });
    }

    #[test]
    fn empty_yields_none() {
        with_env(Some(""), || {
            assert_eq!(get_ichiran_connection_env().unwrap(), None);
        });
    }

    #[test]
    fn whitespace_only_yields_none() {
        with_env(Some("   "), || {
            assert_eq!(get_ichiran_connection_env().unwrap(), None);
        });
    }

    #[test]
    fn url_value_passes_through() {
        let url = "postgres://postgres@localhost/jmdict?sslmode=disable";
        with_env(Some(url), || {
            assert_eq!(get_ichiran_connection_env().unwrap().as_deref(), Some(url));
        });
    }
}
