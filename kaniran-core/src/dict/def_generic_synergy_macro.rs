//! Port of `ichiran/dict:def-generic-synergy` (`dict-grammar.lisp:731-746`).
//!
//! ```lisp
//! (defmacro def-generic-synergy (name (segment-list-left segment-list-right)
//!                                filter-left filter-right &key description connector score)
//!   (alexandria:with-gensyms (start end left right)
//!    `(defsynergy ,name (,segment-list-left ,segment-list-right)
//!       (let ((,start (segment-list-end ,segment-list-left))
//!             (,end (segment-list-start ,segment-list-right)))
//!         (when (= ,start ,end)
//!           (let ((,left (remove-if-not ,filter-left (segment-list-segments ,segment-list-left)))
//!                 (,right (remove-if-not ,filter-right (segment-list-segments ,segment-list-right))))
//!             (when (and ,left ,right)
//!               (list (list (make-segment-list-from ,segment-list-right ,right)
//!                           (make-synergy :start ,start :end ,end
//!                                         :description ,description
//!                                         :connector ,connector
//!                                         :score ,score)
//!                           (make-segment-list-from ,segment-list-left ,left))))))))))
//! ```
//!
//! Divergences from Lisp:
//! - `&key description connector score` → [`DefGenericSynergyOpts`]
//!   fields; `description` is `Option<&str>` (`synergy-oki` omits it).
//! - `filter-left` / `filter-right` are passed as already-built
//!   segment predicates `Fn(&Arc<KaniLiteSegment>) -> bool`.
//! - The `(list (list ...))` nil-or-single result is a `Vec` holding
//!   zero or one `(right-list, synergy, left-list)` triple.
//! - The `defsynergy` `pushnew` registration lives in
//!   [`SYNERGY_LIST`](super::_star_synergy_list_star_::SYNERGY_LIST).

use std::sync::Arc;

use super::kani::KaniLiteSegment;
use super::kani::{make_kani_lite_segment_list_from, KaniLiteSegmentList};
use super::synergy_struct::Synergy;

pub struct DefGenericSynergyOpts<'a> {
    pub description: Option<&'a str>,
    pub connector: &'a str,
    pub score: i32,
}

pub fn def_generic_synergy_body(
    segment_list_left: &KaniLiteSegmentList,
    segment_list_right: &KaniLiteSegmentList,
    filter_left: impl Fn(&Arc<KaniLiteSegment>) -> bool,
    filter_right: impl Fn(&Arc<KaniLiteSegment>) -> bool,
    opts: &DefGenericSynergyOpts<'_>,
) -> Vec<(Arc<KaniLiteSegmentList>, Synergy, Arc<KaniLiteSegmentList>)> {
    let start = segment_list_left.end;
    let end = segment_list_right.start;
    // dict-grammar.lisp:737 (when (= start end))
    if start != end {
        return vec![];
    }
    // dict-grammar.lisp:738-739 (remove-if-not filter-left/right over segment-list-segments)
    let left: Vec<Arc<KaniLiteSegment>> = segment_list_left
        .segments
        .iter()
        .filter(|s| filter_left(s))
        .cloned()
        .collect();
    let right: Vec<Arc<KaniLiteSegment>> = segment_list_right
        .segments
        .iter()
        .filter(|s| filter_right(s))
        .cloned()
        .collect();
    // dict-grammar.lisp:740 (when (and left right))
    if left.is_empty() || right.is_empty() {
        return vec![];
    }
    // dict-grammar.lisp:741-746 (list (list (make-segment-list-from r right) (make-synergy ...) (make-segment-list-from l left)))
    let syn = Synergy {
        description: opts.description.map(|d| d.to_string()),
        connector: Some(opts.connector.to_string()),
        score: opts.score,
        start,
        end,
    };
    vec![(
        Arc::new(make_kani_lite_segment_list_from(segment_list_right, right)),
        syn,
        Arc::new(make_kani_lite_segment_list_from(segment_list_left, left)),
    )]
}
