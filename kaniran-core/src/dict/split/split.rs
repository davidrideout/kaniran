//! Port of the dict-split.lisp split-dispatch layer — the small
//! runtime that walks `*split-map*` (`get-split*`), wraps the result
//! to drop incomplete part lists (`get-split`), and the `optprefix`
//! closure factory used by some `def-simple-split` callsites. The
//! split data tables themselves live in [`super::split_map`] (the
//! 174 registered seqs) and [`super::segsplit`] (the 18-entry
//! `*segsplit-map*` overlay).
//!
//! Section order mirrors upstream's `dict-split.lisp:67-80` (get-split*
//! at line 69, get-split at line 77, optprefix at line 523).

use crate::conn::kani_context::KaniranContext;
use crate::dict::kani::{KaniSimpleTextDispatchEnum, SplitPart};
use crate::dict::split::split_map::split_map_dispatch;

// =========================================================================
// get-split* (dict-split.lisp:69)
// =========================================================================

/// Look up a split function in [`super::split_map`] by `(seq reading)`;
/// if absent, walk `conj-of` left-to-right and return the first
/// registered split. Returns `None` when no entry matches on either
/// path.
///
/// Diverges from the upstream lambda list `(reading &optional conj-of)`
/// by taking `&KaniranContext` for the database handle (replacing
/// Lisp's dynamic `*connection*` per [`crate::conn::kani_context`])
/// and threading it into the dispatched split function. The upstream
/// `&optional conj-of` becomes `conj_of: &[i32]` — empty slice for
/// the unspecified case.
pub async fn get_split_star_(
    ctx: &KaniranContext,
    reading: &KaniSimpleTextDispatchEnum,
    conj_of: &[i32],
) -> Result<Option<(Vec<Option<SplitPart>>, i32)>, sqlx::Error> {
    if let Some(res) = split_map_dispatch(reading.seq(), ctx, reading).await {
        return res.map(Some);
    }
    for &seq in conj_of {
        if let Some(res) = split_map_dispatch(seq, ctx, reading).await {
            return res.map(Some);
        }
    }
    Ok(None)
}

// =========================================================================
// get-split (dict-split.lisp:77)
// =========================================================================

/// Safety wrapper around [`get_split_star_`]: returns the parts list
/// and score only when a split function ran AND every part resolved
/// to a non-`nil` word object. Mirrors
/// `(when (and split (every 'identity split)) (values split score))`.
///
/// Diverges from the upstream lambda list `(reading &optional conj-of)`
/// identically to [`get_split_star_`] — see that fn's doc-comment for
/// the ctx-injection rationale.
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

// =========================================================================
// optprefix (dict-split.lisp:523)
// =========================================================================

/// Build a closure that prepends `prefix` to its argument when the
/// argument doesn't already start with `prefix`. Used by
/// `def-simple-split` entries (e.g. `split-1894260`) to optionally
/// reattach a leading kana to the matched continuation.
pub fn optprefix(prefix: &str) -> impl Fn(&str) -> String {
    let prefix = prefix.to_string();
    move |txt: &str| {
        if txt.starts_with(&prefix) {
            txt.to_string()
        } else {
            format!("{prefix}{txt}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optprefix_already_has_prefix() {
        assert_eq!(optprefix("い")("いう"), "いう");
    }

    #[test]
    fn optprefix_missing_prefix_prepended() {
        assert_eq!(optprefix("い")("う"), "いう");
    }

    #[test]
    fn optprefix_empty_input_takes_prefix() {
        assert_eq!(optprefix("い")(""), "い");
    }
}
