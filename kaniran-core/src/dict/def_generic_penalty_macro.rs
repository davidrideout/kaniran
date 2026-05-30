//! Port of `ichiran/dict:def-generic-penalty` (`dict-grammar.lisp:972-984`).
//!
//! ```lisp
//! (defmacro def-generic-penalty (name (segment-list-left segment-list-right)
//!                                test-left test-right &key (serial t) description score (connector " "))
//!   (alexandria:with-gensyms (start end)
//!    `(defpenalty ,name (,segment-list-left ,segment-list-right)
//!       (let ((,start (segment-list-end ,segment-list-left))
//!             (,end (segment-list-start ,segment-list-right)))
//!         (when (and ,(if serial `(= ,start ,end) t)
//!                    (funcall ,test-left ,segment-list-left)
//!                    (funcall ,test-right ,segment-list-right))
//!           (make-synergy :start ,start :end ,end
//!                         :description ,description
//!                         :connector ,connector
//!                         :score ,score))))))
//! ```
//!
//! Divergences from Lisp:
//! - `&key (serial t)` → `bool`; `description` / `score` / `connector`
//!   keywords → [`DefGenericPenaltyOpts`] fields.
//! - The nil-or-`synergy` result is `Option<Synergy>`.
//! - The `defpenalty` `pushnew` registration lives in
//!   [`PENALTY_LIST`](super::_star_penalty_list_star_::PENALTY_LIST).

use super::kani::KaniLiteSegmentList;
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
