//! Port of `ichiran/conn:get-ichiran-connection-env` (`conn.lisp:154-166`).
//!
//! Read the database URL from a layered [`config::Config`]: an
//! optional `kaniran.toml` (or other `config`-supported extension) in
//! the current working directory, overlaid by env vars. Returns
//! [`None`] when neither source supplies a value or the value is
//! empty; surfaces a malformed read as [`Error::Config`].
//!
//! Divergences from Lisp:
//! - Returns `Option<String>` (a Postgres URL) rather than a parsed
//!   Lisp connection list. Format change is documented on
//!   [`super::_star_connection_env_var_star_::DATABASE_URL`].
//! - Layered config-file → env-var read instead of a single env-var
//!   parse, so checked-in defaults can ride alongside per-shell
//!   overrides without code changes.
//! - Upstream silently warns and returns nil on parse failure; Rust
//!   bubbles parse failure as an error and reserves [`None`] for the
//!   "unset / empty" case alone.

use crate::conn::_star_connection_env_var_star_::DATABASE_URL;
use crate::conn::kani_context::Error;
use config::{Config, ConfigError, Environment, File};

pub fn get_ichiran_connection_env() -> Result<Option<String>, Error> {
    let cfg = Config::builder()
        .add_source(File::with_name("kaniran").required(false))
        .add_source(Environment::default())
        .build()?;
    connection_url_from_config(&cfg)
}

/// The resolved-value decision: empty / whitespace → [`None`], a
/// present non-empty value → [`Some`], a missing key → [`None`], any
/// other read error → [`Error`]. Factored out so it can be exercised
/// against an explicitly-built [`Config`] without mutating the
/// process-global environment.
fn connection_url_from_config(cfg: &Config) -> Result<Option<String>, Error> {
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

    // Build a Config carrying an explicit `database_url` override —
    // exercises the same `get_string(&key)` → match path the env
    // overlay feeds, without touching the process-global environment
    // (which `from_env` callers across the crate read concurrently).
    fn config_with(database_url: Option<&str>) -> Config {
        let builder = Config::builder();
        let builder = match database_url {
            Some(v) => builder
                .set_override(DATABASE_URL.to_ascii_lowercase(), v)
                .unwrap(),
            None => builder,
        };
        builder.build().unwrap()
    }

    #[test]
    fn resolves_override_value() {
        // (database_url override, expected). Empty / whitespace-only and
        // a missing key all collapse to None; a present non-empty value
        // passes through unchanged.
        let cases: &[(Option<&str>, Option<&str>)] = &[
            (Some(""), None),
            (Some("   "), None),
            (None, None),
            (
                Some("postgres://postgres@localhost/jmdict?sslmode=disable"),
                Some("postgres://postgres@localhost/jmdict?sslmode=disable"),
            ),
        ];
        for (override_value, expected) in cases {
            let got = connection_url_from_config(&config_with(*override_value)).unwrap();
            assert_eq!(got.as_deref(), *expected, "override={override_value:?}");
        }
    }

    #[test]
    fn config_file_value_passes_through() {
        // The `kaniran.toml` File source alone (no env overlay) is read
        // and passed through the same logic — the fallback path when
        // DATABASE_URL is unset in the environment.
        let cfg = Config::builder()
            .add_source(File::with_name("kaniran").required(false))
            .build()
            .unwrap();
        let from_file = cfg
            .get_string(&DATABASE_URL.to_ascii_lowercase())
            .ok()
            .filter(|s| !s.trim().is_empty());
        assert_eq!(connection_url_from_config(&cfg).unwrap(), from_file);
    }
}
