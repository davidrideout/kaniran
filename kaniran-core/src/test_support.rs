//! Shared fixtures for the in-crate unit tests.
//!
//! Building a context memory-maps the ~1.5 GB rkyv archive and populates
//! every lookup cache, so it is expensive. Each test used to call
//! `KaniranContext::from_env` on its own; under the rkyv backend the test
//! harness ran those builds once per parallel thread and exhausted memory.
//! The whole unit suite links into one test binary, so a single
//! process-wide `OnceLock` lets every test share one context — the first
//! caller builds it, the rest clone the `Arc`.

use std::sync::{Arc, OnceLock};

use crate::conn::kani_context::KaniranContext;

/// The process-wide shared context, built on first use from `DATABASE_URL`
/// / `kaniran.toml`. Concurrent first callers block on the single build,
/// then all share the same `Arc`.
pub(crate) fn shared_ctx() -> Arc<KaniranContext> {
    static CTX: OnceLock<Arc<KaniranContext>> = OnceLock::new();
    CTX.get_or_init(|| {
        KaniranContext::from_env().expect("DATABASE_URL / kaniran.toml required")
    })
    .clone()
}
