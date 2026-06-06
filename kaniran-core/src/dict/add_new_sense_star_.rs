//! Port of `ichiran/dict:add-new-sense*` (`dict-errata.lisp:155`).
//!
//! Convenience wrapper around [`super::add_new_sense::add_new_sense`]:
//! wraps the single `pos` in a 1-element positions list and forwards
//! `glosses`.

use super::add_new_sense::add_new_sense;
use crate::conn::kani_context::KaniranContext;

pub async fn add_new_sense_star_(
    ctx: &KaniranContext,
    seq: i32,
    pos: &str,
    glosses: &[String],
) -> Result<Option<(i32, i32)>, sqlx::Error> {
    // dict-errata.lisp:156 (add-new-sense seq (list pos) glosses)
    let positions = [pos.to_string()];
    add_new_sense(ctx, seq, &positions, glosses).await
}
