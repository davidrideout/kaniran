//! Port of `ichiran/maintenance:add-errata` (`ichiran.lisp:154`).
//!
//! Public-facing wrapper around [`crate::dict::errata::add_errata`].

use crate::conn::kani_context::KaniranContext;
use crate::custom::load::LoadCustomDataError;

pub async fn add_errata(ctx: &KaniranContext) -> Result<(), LoadCustomDataError> {
    crate::dict::errata::add_errata(ctx).await
}
