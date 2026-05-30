//! Port of `ichiran/dict:*penalty-list*` (`dict-grammar.lisp:964`).
//!
//! Registry of penalty fn pointers iterated by [`get_penalties`]. In
//! Lisp this is `(defparameter *penalty-list* nil)` populated at load
//! time by `pushnew` inside the `defpenalty` macro
//! (`dict-grammar.lisp:966`); the order below mirrors the captured
//! value of the global — `pushnew` prepends, so the last `defpenalty`
//! site appears first.
//!
//! Divergences from Lisp:
//! - Encoded as a fixed `&[Penalty]` instead of a mutable
//!   `defparameter`. Per CONVENTIONS §4.6 the `defpenalty` macro is
//!   marked `skip` (DSL definer); its only effect was the
//!   accumulation captured here.
//! - Each entry is a Rust `fn` pointer with the shared signature
//!   `fn(&SegmentList, &SegmentList) -> Option<Synergy>`.
//!
//! [`get_penalties`]: super::get_penalties::get_penalties

use super::kani::KaniLiteSegmentList;
use super::penalty_semi_final::penalty_semi_final;
use super::penalty_short::penalty_short;
use super::synergy_struct::Synergy;

pub type Penalty = fn(&KaniLiteSegmentList, &KaniLiteSegmentList) -> Option<Synergy>;

pub static PENALTY_LIST: &[Penalty] = &[penalty_semi_final, penalty_short];
