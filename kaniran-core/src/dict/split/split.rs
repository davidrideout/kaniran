use crate::conn::kani_context::KaniranContext;
use crate::dict::kani_word::KaniSimpleTextDispatchEnum;
use crate::dict::split::kani_split_part::SplitPart;
use crate::dict::split::split_map::split_map_dispatch;

/// Port of `ichiran/dict:get-split*` (`dict-split.lisp:69`).
///
/// Look up a split function in [`crate::dict::_star_split_map_star_`]
/// by `(seq reading)`; if absent, walk `conj-of` left-to-right and
/// return the first registered split. Returns `None` when no entry
/// matches on either path.
pub fn get_split_star_(
    ctx: &KaniranContext,
    reading: &KaniSimpleTextDispatchEnum,
    conj_of: &[i32],
) -> Result<Option<(Vec<Option<SplitPart>>, i32)>, crate::conn::KaniDbError> {
    if let Some(res) = split_map_dispatch(reading.seq(), ctx, reading) {
        return res.map(Some);
    }
    for &seq in conj_of {
        if let Some(res) = split_map_dispatch(seq, ctx, reading) {
            return res.map(Some);
        }
    }
    Ok(None)
}

/// Port of `ichiran/dict:get-split` (`dict-split.lisp:77`).
///
/// Wrapper around [`crate::dict::split::split::get_split_star_`]: returns
/// the parts list and score only when a split function ran AND every
/// part resolved to a non-`nil` word object.
pub fn get_split(
    ctx: &KaniranContext,
    reading: &KaniSimpleTextDispatchEnum,
    conj_of: &[i32],
) -> Result<Option<(Vec<SplitPart>, i32)>, crate::conn::KaniDbError> {
    let Some((parts, score)) = get_split_star_(ctx, reading, conj_of)? else {
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

/// Port of `ichiran/dict:optprefix` (`dict-split.lisp:523`).
///
/// Build a closure that prepends `prefix` to its argument when the
/// argument doesn't already start with `prefix`.
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
mod tests;
