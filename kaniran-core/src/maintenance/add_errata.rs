//! Port of `ichiran/maintenance:add-errata` (`ichiran.lisp:154`).
//!
//! Public-facing wrapper around [`crate::dict::add_errata::add_errata`].

use crate::conn::kani_context::KaniranContext;
use crate::custom::load::LoadCustomDataError;

pub async fn add_errata(ctx: &KaniranContext) -> Result<(), LoadCustomDataError> {
    crate::dict::add_errata::add_errata(ctx).await
}
