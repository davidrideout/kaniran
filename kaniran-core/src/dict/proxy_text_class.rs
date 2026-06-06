//! Port of `ichiran/dict:proxy-text` (`dict.lisp:550`).
//!
//! In-memory `simple-text` subclass that wraps a real reading row
//! (`kanji-text`, `kana-text`, or recursively another `proxy-text`)
//! while presenting altered surface forms, delegating identity-bearing
//! accessors through to the wrapped source.

use crate::dict::kani_word::KaniSimpleTextDispatchEnum;
use crate::dict::simple_text_class::SimpleText;

#[derive(Debug, Clone)]
pub struct ProxyText {
    pub text: String,
    pub kana: String,
    pub source: Box<KaniSimpleTextDispatchEnum>,
    pub state: SimpleText,
}
