//! Port of `ichiran/dict:proxy-text` (`dict.lisp:550`).
//!
//! In-memory `simple-text` subclass that wraps a real reading row
//! (`kanji-text`, `kana-text`, or recursively another `proxy-text`)
//! while presenting altered surface forms. Built by
//! `find-word-as-hiragana` (`dict.lisp:592`) when an input string
//! that's not strictly hiragana matches the hiragana reading of an
//! entry — the proxy carries the user's literal text/kana while
//! delegating identity-bearing accessors (`seq`, `common`, `ord`,
//! `nokanji`, `true-text`, `true-kana`, `true-kanji`, `word-type`,
//! `word-conjugations`, `get-original-text`) through to the wrapped
//! source.
//!
//! Slot shape (faithful to `dict.lisp:550-553` plus the inherited
//! `simple-text` runtime state):
//! - `text`, `kana` — the proxy's own surface forms (typically the
//!   user's input string).
//! - `source` — the underlying reading. Lisp slot type is unrestricted
//!   but every callsite passes a `simple-text` (kanji-text, kana-text,
//!   or proxy-text), so the port narrows to
//!   [`Box<KaniSimpleTextDispatchEnum>`].
//! - `state` — inherited from `simple-text`; runtime-only
//!   `conjugations` / `hintedp` slots, mirroring the
//!   [`super::simple_text_class::SimpleText`] composition pattern used
//!   by [`super::kanji_text_dao::KanjiText`] and
//!   [`super::kana_text_dao::KanaText`].
//!
//! No methods are ported here — the cross-family generics
//! (`true-text`, `true-kana`, `true-kanji`, `word-conjugations`,
//! `seq`, `common`, `ord`, `nokanji`, `word-type`,
//! `get-original-text`) all dispatch on `simple-text` / `proxy-text`
//! and land alongside the wave-158 SCC port.

use crate::dict::kani::KaniSimpleTextDispatchEnum;
use crate::dict::simple_text_class::SimpleText;

#[derive(Debug, Clone)]
pub struct ProxyText {
    pub text: String,
    pub kana: String,
    pub source: Box<KaniSimpleTextDispatchEnum>,
    pub state: SimpleText,
}
