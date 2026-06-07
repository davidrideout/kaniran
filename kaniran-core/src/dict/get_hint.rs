//! Port of `ichiran/dict:get-hint` (`dict-split.lisp:938-945`).
//!
//! Look up a hint function for `reading` in `*hint-map*`, applying the
//! first one that matches: first a direct lookup on the reading's own
//! seq, else walk every `from`-seq in the reading's conjugation data
//! and return the first hint that fires. `None` when neither path
//! produces a hint.

use crate::conn::kani_context::KaniranContext;
use crate::dict::_star_hint_map_star_::{hint_map_dispatch, HintDispatch};
use crate::dict::conj_data_from::conj_data_from;
use crate::dict::kani_word::KaniWordDispatchEnum;
use crate::dict::counters::methods::seq as word_seq;
use crate::dict::word_conj_data::word_conj_data;
use crate::dict::word_info_class::WordInfoSeq;

pub async fn get_hint(
    ctx: &KaniranContext,
    reading: &KaniWordDispatchEnum,
) -> Result<Option<String>, sqlx::Error> {
    // dict-split.lisp:939 — (gethash (seq reading) *hint-map*)
    let primary_seq = match word_seq(reading) {
        Some(WordInfoSeq::Single(s)) => Some(s),
        // compound-text returns a list; get-hint's hashtable keys are
        // single ints. Upstream `(gethash <list> *hint-map*)` always
        // misses (lists don't hash to integers), so treat as no
        // primary lookup. (Get-kana :around only fires for simple-text
        // anyway — this branch is defensive.)
        Some(WordInfoSeq::Multi(_)) | None => None,
    };
    // dict-split.lisp:941-942 — `(if hint-fn (funcall hint-fn reading) ...)`.
    // When the primary seq IS registered, return its body's result
    // directly — even if the body returned nil (a `:test` clause
    // failed). The conj-of walk only fires for an UNREGISTERED
    // primary, not for a registered-but-nil-returning one.
    if let Some(s) = primary_seq {
        match hint_map_dispatch(ctx, s, reading).await? {
            HintDispatch::Registered(result) => return Ok(result),
            HintDispatch::Unregistered => { /* fall through to conj-of walk */ }
        }
    }

    // dict-split.lisp:943-945 — walk conj-of seqs. The upstream
    // `when hint-fn do (return (funcall hint-fn reading))` returns
    // the funcall result on the FIRST registered seq, whatever the
    // body returned (Some or None). Subsequent conj-of seqs are
    // never tried after the first hit, even if its body returned nil.
    let conj_data = word_conj_data(ctx, reading).await?;
    for cd in &conj_data {
        if let Some(from_seq) = conj_data_from(cd) {
            match hint_map_dispatch(ctx, from_seq, reading).await? {
                HintDispatch::Registered(result) => return Ok(result),
                HintDispatch::Unregistered => continue,
            }
        }
    }
    Ok(None)
}
