//! Port of `ichiran/dict:score-base` (`dict.lisp:669-672`).
//!
//! Returns the word that drives `compound-text` scoring — the
//! `score-base` slot when set, otherwise the head reading in `primary`.

use super::compound_text_class::CompoundText;
use super::kani_word::KaniWordDispatchEnum;

pub fn score_base(word: &CompoundText) -> &KaniWordDispatchEnum {
    word.score_base.as_deref().unwrap_or(&*word.primary)
}
