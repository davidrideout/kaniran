//! Top-level dispatcher enums for the `ichiran/dict` word polymorphism.
//!
//! Sidecar (no Lisp FQN). Lisp `word` is the ad-hoc union of the
//! reading and tokenization types that segmentation and scoring
//! generic functions (`get-kana`, `text`, `seq`, `common`, `ord`,
//! `word-type`, `word-conj-data`, ...) dispatch over. The
//! [`KaniWordDispatchEnum`] names that union;
//! [`KaniSimpleTextDispatchEnum`] names the `simple-text` sub-family
//! used by [`super::proxy_text_class::ProxyText::source`].
//!
//! No methods are defined here yet. The cross-family generics land
//! alongside their wave-158 SCC port (`get-kana`, `true-text`,
//! `word-conj-data`, `score-base`, `get-hint`); per CONVENTIONS §4.7
//! they will be `pub fn`s on these enums that match-and-delegate to
//! the variant's own method.
//!
//! Counter is wrapped through its existing family enum
//! [`super::counter_text_class::Counter`], which already dispatches
//! across the 11 counter-text subclasses.

use crate::dict::compound_text_class::CompoundText;
use crate::dict::counter_text_class::Counter;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kanji_text_dao::KanjiText;
use crate::dict::proxy_text_class::ProxyText;

#[derive(Debug, Clone)]
pub enum KaniSimpleTextDispatchEnum {
    Kanji(KanjiText),
    Kana(KanaText),
    Proxy(ProxyText),
}

#[derive(Debug, Clone)]
pub enum KaniWordDispatchEnum {
    Kanji(KanjiText),
    Kana(KanaText),
    Proxy(ProxyText),
    Compound(CompoundText),
    Counter(Counter),
}
