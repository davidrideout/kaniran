//! Port of `ichiran/conn:*connection-env-var*` (`conn.lisp:13`).
//!
//! Name of the environment variable that supplies the database URL.
//! Read as a [`config`]-crate key (lowercased to `database_url`) when
//! loaded through [`config::Environment`].
//!
//! Divergences from Lisp:
//! - The upstream value is consumed by `read-from-string`, expecting a
//!   Lisp list spec like `("jmdict" "postgres" "" "localhost" :use-ssl
//!   :yes)`. The Rust port expects a Postgres URL string instead, e.g.
//!   `postgres://postgres@localhost/jmdict?sslmode=disable`.
//! - The env-var name itself moves from `ICHIRAN_CONNECTION` to
//!   `DATABASE_URL` to match the Rust ecosystem's standard convention
//!   (sqlx, diesel, and the `psql` / `pg_dump` family of tools all
//!   default to `DATABASE_URL`).

pub const DATABASE_URL: &str = "DATABASE_URL";
