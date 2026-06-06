//! Port of `ichiran/dict:def-generic-penalty` (`dict-grammar.lisp:972-984`).
//!
//! Shared body for the generic penalty definers: when the two adjacent
//! segment-lists pass their left/right tests (and abut, if `serial`),
//! emit a [`Synergy`] carrying the penalty's description, connector,
//! and score.

use super::kani_lite_segment_list::KaniLiteSegmentList;
use super::synergy_struct::Synergy;

pub struct DefGenericPenaltyOpts<'a> {
    pub serial: bool,
    pub description: &'a str,
    pub score: i32,
    pub connector: &'a str,
}

pub fn def_generic_penalty_body(
    segment_list_left: &KaniLiteSegmentList,
    segment_list_right: &KaniLiteSegmentList,
    test_left: impl Fn(&KaniLiteSegmentList) -> bool,
    test_right: impl Fn(&KaniLiteSegmentList) -> bool,
    opts: &DefGenericPenaltyOpts<'_>,
) -> Option<Synergy> {
    let start = segment_list_left.end;
    let end = segment_list_right.start;
    // dict-grammar.lisp:978-980 (and (if serial (= start end) t) (funcall test-left ...) (funcall test-right ...))
    if (!opts.serial || start == end)
        && test_left(segment_list_left)
        && test_right(segment_list_right)
    {
        // dict-grammar.lisp:981-984 (make-synergy :start :end :description :connector :score)
        Some(Synergy {
            description: Some(opts.description.to_string()),
            connector: Some(opts.connector.to_string()),
            score: opts.score,
            start,
            end,
        })
    } else {
        None
    }
}
