//! Port of `ichiran/dict:filter-is-noun` (`dict-grammar.lisp:748`).
//!
//! Tests whether a segment is a noun: a kpcl-gated word with one of the
//! six noun parts-of-speech, or a counter-text with a non-empty seq-set.

use std::sync::Arc;

use super::kani_lite_segment::{KaniLiteSegment, KPCL_C, KPCL_K, KPCL_L, KPCL_P, POS_NOUN};

pub fn filter_is_noun(segment: &Arc<KaniLiteSegment>) -> bool {
    let kpcl = segment.kpcl;
    let kpcl_gate =
        (kpcl & (KPCL_L | KPCL_K)) != 0 || (kpcl & KPCL_P != 0 && kpcl & KPCL_C != 0);
    if kpcl_gate && (segment.pos & POS_NOUN) != 0 {
        return true;
    }
    segment.is_counter && !segment.seq_set.is_empty()
}
