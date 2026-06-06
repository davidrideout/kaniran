//! Port of `ichiran/dict:*penalty-list*` (`dict-grammar.lisp:964`).
//!
//! Registry of penalty functions applied to adjacent segments during
//! scoring.

use super::kani_lite_segment_list::KaniLiteSegmentList;
use super::penalty_semi_final::penalty_semi_final;
use super::penalty_short::penalty_short;
use super::synergy_struct::Synergy;

pub type Penalty = fn(&KaniLiteSegmentList, &KaniLiteSegmentList) -> Option<Synergy>;

pub static PENALTY_LIST: &[Penalty] = &[penalty_semi_final, penalty_short];
