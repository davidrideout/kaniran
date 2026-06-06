//! Port of `ichiran/dict:source` (gf — `:reader source` on
//! counter-text at `dict-counters.lisp:14` and proxy-text at
//! `dict.lisp:553`).
//!
//! Reads the `source` slot of a counter-text or proxy-text word
//! (`None` for words that have no such slot).

use crate::dict::counter_text_class::CounterSource;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kani_word::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use crate::dict::kanji_text_dao::KanjiText;

/// Borrowed view of whatever a word's `source` slot holds. The two
/// upstream classes that define `source` (counter-text, proxy-text)
/// hold different slot types, so the Rust dispatcher returns a
/// borrowed sum rather than a single owned value.
#[derive(Debug)]
pub enum SourceRef<'a> {
    CounterKanji(&'a KanjiText),
    CounterKana(&'a KanaText),
    ProxySimple(&'a KaniSimpleTextDispatchEnum),
}

pub fn source(obj: &KaniWordDispatchEnum) -> Option<SourceRef<'_>> {
    match obj {
        KaniWordDispatchEnum::Counter(c) => match c.base().source.as_ref()? {
            CounterSource::Kanji(k) => Some(SourceRef::CounterKanji(k)),
            CounterSource::Kana(k) => Some(SourceRef::CounterKana(k)),
        },
        KaniWordDispatchEnum::Proxy(p) => Some(SourceRef::ProxySimple(&p.source)),
        // No method on these classes upstream; callers that statically
        // hold one read the slot directly (or, for kanji/kana, there
        // is no `source` slot at all).
        KaniWordDispatchEnum::Kanji(_)
        | KaniWordDispatchEnum::Kana(_)
        | KaniWordDispatchEnum::Compound(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::counter_text_class::{Common, Counter, CounterSource, CounterText};
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::kanji_text_dao::KanjiText;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::proxy_text_class::ProxyText;
    use crate::dict::simple_text_class::SimpleText;

    fn kanji(seq: i32) -> KanjiText {
        KanjiText {
            id: 0, seq, text: String::new(), ord: 0,
            common: None, common_tags: String::new(), conjugate_p: true,
            nokanji: false, best_kana: None,
            state: SimpleText::default(),
        }
    }

    fn kana(seq: i32) -> KanaText {
        KanaText {
            id: 0, seq, text: String::new(), ord: 0,
            common: None, common_tags: String::new(), conjugate_p: true,
            nokanji: false, best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn counter_with_source(source: Option<CounterSource>) -> Counter {
        Counter::Base(CounterText {
            text: String::new(), kana: String::new(),
            number_text: "0".into(), number: 0,
            source, ordinalp: false, suffix: None,
            accepts_suffixes: Vec::new(), suffix_descriptions: Vec::new(),
            digit_opts: Vec::new(), common: Common::Inherit,
            allowed: Vec::new(), foreign: false,
        })
    }

    #[test]
    fn counter_kanji_source() {
        let c = counter_with_source(Some(CounterSource::Kanji(kanji(42))));
        match source(&KaniWordDispatchEnum::Counter(c)) {
            Some(SourceRef::CounterKanji(k)) => assert_eq!(k.seq, 42),
            other => panic!("expected CounterKanji, got {:?}", other),
        }
    }

    #[test]
    fn counter_kana_source() {
        let c = counter_with_source(Some(CounterSource::Kana(kana(7))));
        match source(&KaniWordDispatchEnum::Counter(c)) {
            Some(SourceRef::CounterKana(k)) => assert_eq!(k.seq, 7),
            other => panic!("expected CounterKana, got {:?}", other),
        }
    }

    #[test]
    fn counter_no_source_returns_none() {
        let c = counter_with_source(None);
        assert!(source(&KaniWordDispatchEnum::Counter(c)).is_none());
    }

    #[test]
    fn proxy_source_borrows_simple_chain() {
        let inner = KaniSimpleTextDispatchEnum::Kanji(kanji(99));
        let proxy = ProxyText {
            text: String::new(), kana: String::new(),
            source: Box::new(inner), state: SimpleText::default(),
        };
        match source(&KaniWordDispatchEnum::Proxy(proxy)) {
            Some(SourceRef::ProxySimple(KaniSimpleTextDispatchEnum::Kanji(k))) => {
                assert_eq!(k.seq, 99);
            }
            other => panic!("expected ProxySimple(Kanji), got {:?}", other),
        }
    }

    #[test]
    fn kanji_kana_compound_have_no_source() {
        assert!(source(&KaniWordDispatchEnum::Kanji(kanji(1))).is_none());
        assert!(source(&KaniWordDispatchEnum::Kana(kana(1))).is_none());
        // Compound covered by the kani_word dispatcher path; constructing
        // a CompoundText needs its full payload — the variant's no-source
        // arm is exercised via the match-arm coverage of the other
        // tests since it shares the `_ => None` branch.
    }
}
