//! Port of `ichiran/dict:get-hint` (`dict-split.lisp:938-945`).
//!
//! Look up a hint function for `reading` in
//! [`super::_star_hint_map_star_::HINT_MAP`], applying the first one
//! that matches:
//!
//! 1. `(gethash (seq reading) *hint-map*)` — direct lookup on the
//!    reading's own seq. If a registered hint-fn exists, call it.
//! 2. Otherwise walk every `from`-seq in `(mapcar #'conj-data-from
//!    (word-conj-data reading))`, returning the first hint result
//!    that fires.
//!
//! Returns `None` when neither path produces a hint — meaning the
//! `get-kana :around` method on `simple-text` falls through to
//! `(call-next-method)`.
//!
//! ## Divergences
//!
//! Diverges from the upstream lambda list `(reading)` by:
//! - taking `&KaniranContext` for the database handle (transitively,
//!   via [`super::word_conj_data::word_conj_data`] and the hint-fn
//!   bodies), replacing the upstream dynamic `*connection*` per
//!   [`crate::conn::kani_context`]. The ctx also carries the
//!   `*disable-hints*` binding (see
//!   [`super::get_kana::get_kana`] for the rationale); callers
//!   invoke `get_hint` after rebinding via
//!   [`crate::conn::kani_context::KaniranContext::with_disable_hints`]`(true)`.
//! - returning `Result<Option<String>, sqlx::Error>` — hint-fn bodies
//!   reach the database (via [`super::true_kana::true_kana`] and
//!   [`super::true_kanji::true_kanji`] and
//!   [`crate::kanji::match_readings::match_readings`]), so errors
//!   propagate;
//! - the hint-fn invocation itself dispatches through
//!   [`super::_star_hint_map_star_::hint_map_dispatch`] (the static
//!   table replacing the upstream runtime hashtable per
//!   CONVENTIONS §5.4 + the `*split-map*` precedent).
//!
//! ## Upstream gf cross-dispatch
//!
//! Upstream `(seq reading)` is itself a generic function with five
//! method bodies (proxy / compound / counter-text / simple-text
//! slot reader / entry). Since `get-hint` is called from the
//! `simple-text :around` method on `get-kana` (`dict.lisp:80-84`),
//! the receiver here is always a `kanji-text`, `kana-text`, or
//! `proxy-text`. For proxy-text, `(seq proxy)` is
//! `(seq (source proxy))`, which the Rust dispatcher routes through
//! [`super::seq::seq`].

use crate::conn::kani_context::KaniranContext;
use crate::dict::_star_hint_map_star_::{hint_map_dispatch, HintDispatch};
use crate::dict::conj_data_from::conj_data_from;
use crate::dict::kani::KaniWordDispatchEnum;
use crate::dict::seq::seq as word_seq;
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
