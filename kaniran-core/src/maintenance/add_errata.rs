//! Port of `ichiran/maintenance:add-errata` (`ichiran.lisp:154`).
//!
//! Public-facing wrapper around [`crate::dict::add_errata::add_errata`].
//! Upstream takes an `&optional conn` and wraps the call in `with-db
//! conn` to swap the dynamic `*connection*` for the duration; the Rust
//! port replaces that with explicit `&KaniranContext` injection per
//! [`crate::conn::kani_context`], so the wrapper is a one-line
//! re-export — callers pass whichever ctx points at the target DB.

use crate::conn::kani_context::KaniranContext;
use crate::custom::load_custom_data::LoadCustomDataError;

pub async fn add_errata(ctx: &KaniranContext) -> Result<(), LoadCustomDataError> {
    crate::dict::add_errata::add_errata(ctx).await
}
