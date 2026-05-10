//! Port of `ichiran/dict:get-split` (`dict-split.lisp:77`).
//!
//! Safety wrapper around [`super::get_split_star_::get_split_star_`]:
//! returns the parts list and score only when a split function ran
//! AND every part resolved to a non-`nil` word object. Mirrors
//! `(when (and split (every 'identity split)) (values split score))`.
//!
//! Diverges from the upstream lambda list `(reading &optional conj-of)`
//! identically to [`super::get_split_star_::get_split_star_`] — see that
//! file's doc-comment for the ctx-injection rationale.

use crate::conn::kani_context::KaniranContext;
use crate::dict::get_split_star_::get_split_star_;
use crate::dict::kani_split_part::SplitPart;
use crate::dict::kani_word::KaniSimpleTextDispatchEnum;

pub async fn get_split(
    ctx: &KaniranContext,
    reading: &KaniSimpleTextDispatchEnum,
    conj_of: &[i32],
) -> Result<Option<(Vec<SplitPart>, i32)>, sqlx::Error> {
    let Some((parts, score)) = get_split_star_(ctx, reading, conj_of).await? else {
        return Ok(None);
    };
    // dict-split.lisp:80 — `(every 'identity split)`; on empty list
    // `(every ...)` returns T, but the outer `(and split ...)` rejects
    // empty (nil-as-list is falsy). Mirror both conditions.
    if parts.is_empty() {
        return Ok(None);
    }
    let mut filtered: Vec<SplitPart> = Vec::with_capacity(parts.len());
    for p in parts {
        match p {
            Some(part) => filtered.push(part),
            None => return Ok(None),
        }
    }
    Ok(Some((filtered, score)))
}
