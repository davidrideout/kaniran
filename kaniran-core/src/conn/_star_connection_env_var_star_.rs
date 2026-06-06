//! Port of `ichiran/conn:*connection-env-var*` (`conn.lisp:13`).
//!
//! Name of the environment variable that supplies the database URL
//! (a Postgres URL string, read via the [`config`] crate).

pub const DATABASE_URL: &str = "DATABASE_URL";
