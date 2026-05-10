//! Port of `ichiran/dict:get-split*` (`dict-split.lisp:69`).
//!
//! Look up a split function in [`crate::dict::_star_split_map_star_`]
//! by `(seq reading)`; if absent, walk `conj-of` left-to-right and
//! return the first registered split. Returns `None` when no entry
//! matches on either path.
//!
//! Diverges from the upstream lambda list `(reading &optional conj-of)`
//! by taking `&KaniranContext` for the database handle (replacing
//! Lisp's dynamic `*connection*` per [`crate::conn::kani_context`])
//! and threading it into the dispatched split function. The upstream
//! `&optional conj-of` becomes `conj_of: &[i32]` — empty slice for
//! the unspecified case.

use crate::conn::kani_context::KaniranContext;
use crate::dict::_star_split_map_star_::split_map_dispatch;
use crate::dict::kani_split_part::SplitPart;
use crate::dict::kani_word::KaniSimpleTextDispatchEnum;

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
