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
//! Counter is wrapped through its existing family enum
//! [`super::counter_text_class::Counter`], which already dispatches
//! across the 11 counter-text subclasses.
//!
//! Inherent methods on [`KaniSimpleTextDispatchEnum`] narrow three
//! cross-family generics — `seq`, `true-text`, `word-type` — to the
//! simple-text + proxy-text subset. They mirror the upstream
//! `:method` bodies for those classes (slot read for kanji-text /
//! kana-text, recurse on `(source obj)` for proxy-text) and exist so
//! split-* implementations can borrow from a `&KaniSimpleTextDispatchEnum`
//! without cloning into [`KaniWordDispatchEnum`] for each of the three
//! reads. Equivalent to wrapping the enum in [`KaniWordDispatchEnum`]
//! and calling [`super::seq::seq`], [`super::true_text::true_text`],
//! or [`super::word_type::word_type`].

use crate::dict::compound_text_class::CompoundText;
use crate::dict::counter_text_class::Counter;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kanji_text_dao::KanjiText;
use crate::dict::proxy_text_class::ProxyText;
use crate::dict::word_type::WordType;

#[derive(Debug, Clone)]
pub enum KaniSimpleTextDispatchEnum {
    Kanji(KanjiText),
    Kana(KanaText),
    Proxy(ProxyText),
}

impl KaniSimpleTextDispatchEnum {
    pub fn seq(&self) -> i32 {
        let mut current = self;
        loop {
            match current {
                Self::Kanji(k) => return k.seq,
                Self::Kana(k) => return k.seq,
                Self::Proxy(p) => current = &p.source,
            }
        }
    }

    pub fn true_text(&self) -> &str {
        let mut current = self;
        loop {
            match current {
                Self::Kanji(k) => return &k.text,
                Self::Kana(k) => return &k.text,
                Self::Proxy(p) => current = &p.source,
            }
        }
    }

    pub fn word_type(&self) -> WordType {
        let mut current = self;
        loop {
            match current {
                Self::Kanji(_) => return WordType::Kanji,
                Self::Kana(_) => return WordType::Kana,
                Self::Proxy(p) => current = &p.source,
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum KaniWordDispatchEnum {
    Kanji(KanjiText),
    Kana(KanaText),
    Proxy(ProxyText),
    Compound(CompoundText),
    Counter(Counter),
}
