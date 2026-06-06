//! Port of `ichiran/dict:nokanji` (gf — `dict-counters.lisp:0`).
//!
//! Per-reading "kanji-blocked" flag — true when a kana reading should
//! never be paired with a kanji form. A slot on `kanji-text`/`kana-text`;
//! `counter-text` and `proxy-text` recurse via `source`; `compound-text`
//! has no method (returns `None`).

use crate::dict::counter_text_class::{Counter, CounterSource};
use crate::dict::kani_word::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};

pub fn nokanji(obj: &KaniWordDispatchEnum) -> Option<bool> {
    match obj {
        KaniWordDispatchEnum::Kanji(k) => Some(k.nokanji),
        KaniWordDispatchEnum::Kana(k) => Some(k.nokanji),
        KaniWordDispatchEnum::Proxy(p) => Some(nokanji_simple(&p.source)),
        KaniWordDispatchEnum::Counter(c) => Some(nokanji_counter(c)),
        KaniWordDispatchEnum::Compound(_) => None,
    }
}

fn nokanji_simple(obj: &KaniSimpleTextDispatchEnum) -> bool {
    match obj {
        KaniSimpleTextDispatchEnum::Kanji(k) => k.nokanji,
        KaniSimpleTextDispatchEnum::Kana(k) => k.nokanji,
        KaniSimpleTextDispatchEnum::Proxy(p) => nokanji_simple(&p.source),
    }
}

fn nokanji_counter(c: &Counter) -> bool {
    // dict-counters.lisp:89-90 — `(and (source obj) (nokanji (source obj)))`
    match c.base().source.as_ref() {
        None => false,
        Some(CounterSource::Kanji(k)) => k.nokanji,
        Some(CounterSource::Kana(k)) => k.nokanji,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::counter_text_class::{Common, Counter, CounterSource, CounterText};
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::kanji_text_dao::KanjiText;
    use crate::dict::proxy_text_class::ProxyText;
    use crate::dict::simple_text_class::SimpleText;

    fn kanji_with(nokanji: bool) -> KanjiText {
        KanjiText {
            id: 0, seq: 0, text: String::new(), ord: 0,
            common: None, common_tags: String::new(), conjugate_p: true,
            nokanji, best_kana: None, state: SimpleText::default(),
        }
    }

    fn kana_with(nokanji: bool) -> KanaText {
        KanaText {
            id: 0, seq: 0, text: String::new(), ord: 0,
            common: None, common_tags: String::new(), conjugate_p: true,
            nokanji, best_kanji: None, state: SimpleText::default(),
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
    fn proxy_recurses_through_source_chain() {
        let leaf = KaniSimpleTextDispatchEnum::Kana(kana_with(true));
        let inner = KaniSimpleTextDispatchEnum::Proxy(ProxyText {
            text: String::new(), kana: String::new(),
            source: Box::new(leaf), state: SimpleText::default(),
        });
        let outer = ProxyText {
            text: String::new(), kana: String::new(),
            source: Box::new(inner), state: SimpleText::default(),
        };
        assert_eq!(
            nokanji(&KaniWordDispatchEnum::Proxy(outer)),
            Some(true),
        );
    }

    #[test]
    fn counter_without_source_is_false() {
        let c = counter_with_source(None);
        assert_eq!(nokanji(&KaniWordDispatchEnum::Counter(c)), Some(false));
    }

    #[test]
    fn counter_with_kana_source_propagates_flag() {
        let c = counter_with_source(Some(CounterSource::Kana(kana_with(true))));
        assert_eq!(nokanji(&KaniWordDispatchEnum::Counter(c)), Some(true));
    }

    #[test]
    fn counter_with_kanji_source_propagates_flag() {
        let c = counter_with_source(Some(CounterSource::Kanji(kanji_with(false))));
        assert_eq!(nokanji(&KaniWordDispatchEnum::Counter(c)), Some(false));
    }
}
