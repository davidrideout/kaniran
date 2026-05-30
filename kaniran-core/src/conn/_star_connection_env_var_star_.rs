//! Port of `ichiran/conn:*connection-env-var*` (`conn.lisp:13`).
//!
//! Name of the env var supplying the database URL. The value is a
//! Postgres URL (e.g. `postgres://postgres@localhost/jmdict?sslmode=disable`);
//! upstream consumes a Lisp connection-spec list via `read-from-string`.

pub const DATABASE_URL: &str = "DATABASE_URL";
