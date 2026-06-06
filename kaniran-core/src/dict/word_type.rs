//! Port of `ichiran/dict:word-type` (gf — `dict.lisp:22-24`).
//!
//! Classifies a word's primary script as `:kanji`, `:kana`, or `:gap`
//! — counters resolve by whether their rendered text holds any kanji
//! char; proxies and compounds recurse on their source / primary word.

use crate::characters::char_class_type::CharClass;
use crate::characters::count_char_class::count_char_class;
use crate::dict::kani_word::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use crate::dict::text::text;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordType {
    Kanji,
    Kana,
    Gap,
}

pub fn word_type(obj: &KaniWordDispatchEnum) -> WordType {
    match obj {
        KaniWordDispatchEnum::Kanji(_) => WordType::Kanji,
        KaniWordDispatchEnum::Kana(_) => WordType::Kana,
        KaniWordDispatchEnum::Counter(_) => {
            let t = text(obj);
            if count_char_class(&t, CharClass::KanjiChar) > 0 {
                WordType::Kanji
            } else {
                WordType::Kana
            }
        }
        KaniWordDispatchEnum::Proxy(p) => word_type_simple(&p.source),
        KaniWordDispatchEnum::Compound(c) => word_type(&c.primary),
    }
}

fn word_type_simple(obj: &KaniSimpleTextDispatchEnum) -> WordType {
    match obj {
        KaniSimpleTextDispatchEnum::Kanji(_) => WordType::Kanji,
        KaniSimpleTextDispatchEnum::Kana(_) => WordType::Kana,
        KaniSimpleTextDispatchEnum::Proxy(p) => word_type_simple(&p.source),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::compound_text_class::{CompoundText, ScoreMod};
    use crate::dict::counter_text_class::{Common, Counter, CounterSource, CounterText};
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::kanji_text_dao::KanjiText;
    use crate::dict::proxy_text_class::ProxyText;
    use crate::dict::simple_text_class::SimpleText;

    fn kanji(seq: i32, text: &str) -> KanjiText {
        KanjiText {
            id: 0, seq, text: text.into(), ord: 0,
            common: None, common_tags: String::new(), conjugate_p: true,
            nokanji: false, best_kana: None, state: SimpleText::default(),
        }
    }

    fn kana(seq: i32, text: &str) -> KanaText {
        KanaText {
            id: 0, seq, text: text.into(), ord: 0,
            common: None, common_tags: String::new(), conjugate_p: true,
            nokanji: false, best_kanji: None, state: SimpleText::default(),
        }
    }

    fn counter(number_text: &str, text: &str, source: Option<CounterSource>) -> Counter {
        Counter::Base(CounterText {
            text: text.into(), kana: String::new(),
            number_text: number_text.into(), number: 0,
            source, ordinalp: false, suffix: None,
            accepts_suffixes: Vec::new(), suffix_descriptions: Vec::new(),
            digit_opts: Vec::new(), common: Common::Inherit,
            allowed: Vec::new(), foreign: false,
        })
    }

    #[test]
    fn kanji_variant_is_kanji() {
        assert_eq!(
            word_type(&KaniWordDispatchEnum::Kanji(kanji(1, "犬"))),
            WordType::Kanji,
        );
    }

    #[test]
    fn kana_variant_is_kana() {
        assert_eq!(
            word_type(&KaniWordDispatchEnum::Kana(kana(1, "いぬ"))),
            WordType::Kana,
        );
    }

    #[test]
    fn counter_with_kanji_in_concatenated_text() {
        // `text` gf concatenates "1" + "人" → "1人"; contains kanji.
        let c = counter("1", "人", None);
        assert_eq!(
            word_type(&KaniWordDispatchEnum::Counter(c)),
            WordType::Kanji,
        );
    }

    #[test]
    fn counter_with_only_kana_text() {
        // "1" + "つ" → "1つ"; no kanji.
        let c = counter("1", "つ", None);
        assert_eq!(
            word_type(&KaniWordDispatchEnum::Counter(c)),
            WordType::Kana,
        );
    }

    #[test]
    fn proxy_recurses_through_source_chain() {
        let leaf = KaniSimpleTextDispatchEnum::Kana(kana(1, "ねこ"));
        let inner = KaniSimpleTextDispatchEnum::Proxy(ProxyText {
            text: String::new(), kana: String::new(),
            source: Box::new(leaf), state: SimpleText::default(),
        });
        let outer = ProxyText {
            text: String::new(), kana: String::new(),
            source: Box::new(inner), state: SimpleText::default(),
        };
        assert_eq!(
            word_type(&KaniWordDispatchEnum::Proxy(outer)),
            WordType::Kana,
        );
    }

    #[test]
    fn compound_dispatches_on_primary() {
        let primary = Box::new(KaniWordDispatchEnum::Kanji(kanji(1, "犬")));
        let words = vec![
            KaniWordDispatchEnum::Kanji(kanji(1, "犬")),
            KaniWordDispatchEnum::Kana(kana(2, "ねこ")),
        ];
        let c = CompoundText {
            text: String::new(), kana: String::new(),
            primary, words,
            score_base: None, score_mod: ScoreMod::Single(0),
        };
        assert_eq!(
            word_type(&KaniWordDispatchEnum::Compound(c)),
            WordType::Kanji,
        );
    }
}
