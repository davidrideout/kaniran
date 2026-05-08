//! Port of `ichiran/dict:score-base` (`dict.lisp:669-672`).
//!
//! Returns the word whose identity drives `compound-text` scoring —
//! the `score-base` slot when set, otherwise the head reading in
//! `primary`. Consumed by `calc-score`'s compound-text branch
//! (`dict.lisp:782`) to drive the recursive scoring call.
//!
//! The Lisp generic has a single method on `compound-text`; with no
//! dispatch needed the Rust port is a free function taking
//! `&CompoundText` rather than a CONVENTIONS §4.7 family-dispatcher.

use super::compound_text_class::CompoundText;
use super::kani_word::KaniWordDispatchEnum;

pub fn score_base(word: &CompoundText) -> &KaniWordDispatchEnum {
    word.score_base.as_deref().unwrap_or(&*word.primary)
}
