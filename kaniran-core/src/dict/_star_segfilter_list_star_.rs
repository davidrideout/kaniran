//! Port of `ichiran/dict:*segfilter-list*` (`dict-grammar.lisp:1024`).
//!
//! Registry of segfilter fn pointers iterated by [`apply_segfilters`].
//! In Lisp this is `(defparameter *segfilter-list* nil)` populated at
//! load time by `pushnew` inside the `defsegfilter` macro
//! (`dict-grammar.lisp:1026`); the order below mirrors the captured
//! value of the global — pushnew prepends, so the last `defsegfilter`
//! site appears first.
//!
//! Divergences from Lisp:
//! - Encoded as a fixed `&[SegFilter]` instead of a mutable
//!   `defparameter`. Per CONVENTIONS §4.6 the `defsegfilter` macro is
//!   marked `skip` (DSL definer); its only effect was the
//!   accumulation captured here.
//! - Each filter is a Rust `fn` pointer that threads
//!   `Arc<SegmentList>` so the pass-through paths inside
//!   [`apply_segfilters`] reduce to refcount bumps; the Lisp
//!   `(list seg-left seg-right)` is also pointer-shared. Modify
//!   paths allocate a fresh `Arc<SegmentList>`.
//!
//! [`apply_segfilters`]: super::apply_segfilters::apply_segfilters

use std::sync::Arc;

use super::segfilter_aux_verb::segfilter_aux_verb;
use super::segfilter_badend::segfilter_badend;
use super::segfilter_dashi::segfilter_dashi;
use super::segfilter_dekiru::segfilter_dekiru;
use super::segfilter_honorific::segfilter_honorific;
use super::segfilter_janai::segfilter_janai;
use super::segfilter_mononi::segfilter_mononi;
use super::segfilter_n::segfilter_n;
use super::segfilter_nohayamete::segfilter_nohayamete;
use super::segfilter_roku::segfilter_roku;
use super::segfilter_sae::segfilter_sae;
use super::segfilter_sukiyoki::segfilter_sukiyoki;
use super::segfilter_toomou::segfilter_toomou;
use super::segfilter_totte::segfilter_totte;
use super::segfilter_tsu_iru::segfilter_tsu_iru;
use super::segfilter_wokarasu::segfilter_wokarasu;
use super::kani::KaniLiteSegmentList;

pub type SegFilter = fn(
    Option<&Arc<KaniLiteSegmentList>>,
    &Arc<KaniLiteSegmentList>,
) -> Vec<(Option<Arc<KaniLiteSegmentList>>, Arc<KaniLiteSegmentList>)>;

pub static SEGFILTER_LIST: &[SegFilter] = &[
    segfilter_mononi,
    segfilter_honorific,
    segfilter_dekiru,
    segfilter_dashi,
    segfilter_totte,
    segfilter_toomou,
    segfilter_nohayamete,
    segfilter_janai,
    segfilter_sae,
    segfilter_roku,
    segfilter_sukiyoki,
    segfilter_badend,
    segfilter_wokarasu,
    segfilter_n,
    segfilter_tsu_iru,
    segfilter_aux_verb,
];
