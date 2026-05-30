//! Port of `(setf word-conjugations)` — setter half of the
//! `:accessor word-conjugations` slot option on `simple-text`
//! (`dict.lisp:70`), plus two explicit overrides:
//!
//! - **simple-text** (`dict.lisp:70`): write `conjugations` slot.
//! - **proxy-text** (`dict.lisp:571-572`): recurse to source.
//! - **compound-text** (`dict.lisp:666-667`): recurse to last word.
//! - **counter-text**: no method; panic mirrors `no-applicable-method`.
//!
//! Companion getter: [`super::word_conjugations`].

use crate::dict::kani::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use crate::dict::simple_text_class::WordConjugations;

pub fn set_word_conjugations(
    word: &mut KaniWordDispatchEnum,
    value: Option<WordConjugations>,
) {
    match word {
        KaniWordDispatchEnum::Kanji(k) => k.state.conjugations = value,
        KaniWordDispatchEnum::Kana(k) => k.state.conjugations = value,
        KaniWordDispatchEnum::Proxy(p) => set_simple_text(&mut p.source, value),
        KaniWordDispatchEnum::Compound(c) => match c.words.last_mut() {
            Some(last) => set_word_conjugations(last, value),
            // (car (last nil)) → nil; (setf (word-conjugations nil) v)
            // → no-applicable-method on nil upstream.
            None => panic!(
                "(setf word-conjugations) on compound-text with empty words: no applicable method"
            ),
        },
        KaniWordDispatchEnum::Counter(_) => panic!(
            "(setf word-conjugations) on counter-text: no applicable method"
        ),
    }
}

fn set_simple_text(
    simple: &mut KaniSimpleTextDispatchEnum,
    value: Option<WordConjugations>,
) {
    match simple {
        KaniSimpleTextDispatchEnum::Kanji(k) => k.state.conjugations = value,
        KaniSimpleTextDispatchEnum::Kana(k) => k.state.conjugations = value,
        KaniSimpleTextDispatchEnum::Proxy(p) => set_simple_text(&mut p.source, value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::compound_text_class::{CompoundText, ScoreMod};
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::kanji_text_dao::KanjiText;
    use crate::dict::proxy_text_class::ProxyText;
    use crate::dict::simple_text_class::SimpleText;

    fn kana(seq: i32) -> KanaText {
        KanaText {
            id: 0,
            seq,
            text: String::new(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn kanji(seq: i32) -> KanjiText {
        KanjiText {
            id: 0,
            seq,
            text: String::new(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: true,
            nokanji: false,
            best_kana: None,
            state: SimpleText::default(),
        }
    }

    #[test]
    fn kana_text_writes_state_slot() {
        let mut w = KaniWordDispatchEnum::Kana(kana(1));
        set_word_conjugations(&mut w, Some(WordConjugations::Root));
        match &w {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.state.conjugations, Some(WordConjugations::Root));
            }
            _ => panic!("expected kana variant"),
        }
    }

    #[test]
    fn kanji_text_writes_state_slot() {
        let mut w = KaniWordDispatchEnum::Kanji(kanji(2));
        set_word_conjugations(&mut w, Some(WordConjugations::Ids(vec![10, 20])));
        match &w {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(
                    k.state.conjugations,
                    Some(WordConjugations::Ids(vec![10, 20]))
                );
            }
            _ => panic!("expected kanji variant"),
        }
    }

    #[test]
    fn kana_text_writes_none() {
        let mut k = kana(3);
        k.state.conjugations = Some(WordConjugations::Root);
        let mut w = KaniWordDispatchEnum::Kana(k);
        set_word_conjugations(&mut w, None);
        match &w {
            KaniWordDispatchEnum::Kana(k) => assert!(k.state.conjugations.is_none()),
            _ => panic!("expected kana variant"),
        }
    }

    #[test]
    fn kanji_text_writes_none() {
        let mut k = kanji(4);
        k.state.conjugations = Some(WordConjugations::Ids(vec![1, 2]));
        let mut w = KaniWordDispatchEnum::Kanji(k);
        set_word_conjugations(&mut w, None);
        match &w {
            KaniWordDispatchEnum::Kanji(k) => assert!(k.state.conjugations.is_none()),
            _ => panic!("expected kanji variant"),
        }
    }

    #[test]
    fn proxy_writes_through_source_chain() {
        // Two-level proxy chain — verify the setter descends to the leaf.
        let leaf = KaniSimpleTextDispatchEnum::Kana(kana(99));
        let inner = ProxyText {
            text: String::new(),
            kana: String::new(),
            source: Box::new(leaf),
            state: SimpleText::default(),
        };
        let outer = ProxyText {
            text: String::new(),
            kana: String::new(),
            source: Box::new(KaniSimpleTextDispatchEnum::Proxy(inner)),
            state: SimpleText::default(),
        };
        let mut w = KaniWordDispatchEnum::Proxy(outer);
        set_word_conjugations(&mut w, Some(WordConjugations::Root));

        // Drill down and verify the leaf kana was mutated, not the proxy wrapper.
        match &w {
            KaniWordDispatchEnum::Proxy(outer) => {
                assert!(outer.state.conjugations.is_none(), "outer state untouched");
                match &*outer.source {
                    KaniSimpleTextDispatchEnum::Proxy(inner) => {
                        assert!(inner.state.conjugations.is_none(), "inner state untouched");
                        match &*inner.source {
                            KaniSimpleTextDispatchEnum::Kana(k) => assert_eq!(
                                k.state.conjugations,
                                Some(WordConjugations::Root)
                            ),
                            _ => panic!("expected kana leaf"),
                        }
                    }
                    _ => panic!("expected proxy inner"),
                }
            }
            _ => panic!("expected proxy outer"),
        }
    }

    #[test]
    fn compound_writes_to_last_word() {
        // dict.lisp:666-667 — setter descends into (car (last (words word))).
        let first = KaniWordDispatchEnum::Kana(kana(1));
        let last = KaniWordDispatchEnum::Kana(kana(2));
        let primary = first.clone();
        let c = CompoundText {
            text: String::new(),
            kana: String::new(),
            primary: Box::new(primary),
            words: vec![first, last],
            score_base: None,
            score_mod: ScoreMod::Single(0),
        };
        let mut w = KaniWordDispatchEnum::Compound(c);
        set_word_conjugations(&mut w, Some(WordConjugations::Root));

        match &w {
            KaniWordDispatchEnum::Compound(c) => {
                // First word untouched.
                match &c.words[0] {
                    KaniWordDispatchEnum::Kana(k) => assert!(k.state.conjugations.is_none()),
                    _ => panic!("expected kana first"),
                }
                // Last word mutated.
                match &c.words[1] {
                    KaniWordDispatchEnum::Kana(k) => assert_eq!(
                        k.state.conjugations,
                        Some(WordConjugations::Root)
                    ),
                    _ => panic!("expected kana last"),
                }
            }
            _ => panic!("expected compound variant"),
        }
    }

    #[test]
    fn compound_writes_ids_value() {
        // Ids variant through compound recursion — distinct from Root.
        let first = KaniWordDispatchEnum::Kana(kana(1));
        let last = KaniWordDispatchEnum::Kanji(kanji(2));
        let primary = first.clone();
        let c = CompoundText {
            text: String::new(),
            kana: String::new(),
            primary: Box::new(primary),
            words: vec![first, last],
            score_base: None,
            score_mod: ScoreMod::Single(0),
        };
        let mut w = KaniWordDispatchEnum::Compound(c);
        set_word_conjugations(&mut w, Some(WordConjugations::Ids(vec![7, 11])));

        match &w {
            KaniWordDispatchEnum::Compound(c) => match &c.words[1] {
                KaniWordDispatchEnum::Kanji(k) => assert_eq!(
                    k.state.conjugations,
                    Some(WordConjugations::Ids(vec![7, 11]))
                ),
                _ => panic!("expected kanji last"),
            },
            _ => panic!("expected compound variant"),
        }
    }

    #[test]
    fn nested_compound_recurses_through_inner_last() {
        // Outer compound whose last element is itself a compound. The setter
        // should descend recursively (last → last) rather than stopping at
        // the inner compound — catches a regression that treated compound-as-
        // last as a terminal write target.
        let a = KaniWordDispatchEnum::Kana(kana(1));
        let b = KaniWordDispatchEnum::Kana(kana(2));
        let c = KaniWordDispatchEnum::Kana(kana(3));
        let inner = CompoundText {
            text: String::new(),
            kana: String::new(),
            primary: Box::new(b.clone()),
            words: vec![b, c],
            score_base: None,
            score_mod: ScoreMod::Single(0),
        };
        let inner_word = KaniWordDispatchEnum::Compound(inner);
        let outer = CompoundText {
            text: String::new(),
            kana: String::new(),
            primary: Box::new(a.clone()),
            words: vec![a, inner_word],
            score_base: None,
            score_mod: ScoreMod::Single(0),
        };
        let mut w = KaniWordDispatchEnum::Compound(outer);
        set_word_conjugations(&mut w, Some(WordConjugations::Root));

        match &w {
            KaniWordDispatchEnum::Compound(outer) => {
                // Outer[0] untouched.
                match &outer.words[0] {
                    KaniWordDispatchEnum::Kana(k) => assert!(k.state.conjugations.is_none()),
                    _ => panic!("expected kana outer[0]"),
                }
                // Outer[1] is the inner compound; its [0] untouched, [1] mutated.
                match &outer.words[1] {
                    KaniWordDispatchEnum::Compound(inner) => {
                        match &inner.words[0] {
                            KaniWordDispatchEnum::Kana(k) => {
                                assert!(k.state.conjugations.is_none(), "inner[0] untouched");
                            }
                            _ => panic!("expected kana inner[0]"),
                        }
                        match &inner.words[1] {
                            KaniWordDispatchEnum::Kana(k) => assert_eq!(
                                k.state.conjugations,
                                Some(WordConjugations::Root),
                                "inner[1] is the leaf — must be mutated"
                            ),
                            _ => panic!("expected kana inner[1]"),
                        }
                    }
                    _ => panic!("expected inner compound at outer[1]"),
                }
            }
            _ => panic!("expected outer compound"),
        }
    }

    #[test]
    fn compound_with_proxy_last_descends_through_source() {
        // Compound whose last element is a Proxy — verify the recursion
        // continues into the proxy's source chain rather than stopping at
        // the proxy wrapper.
        let leaf = KaniSimpleTextDispatchEnum::Kana(kana(50));
        let proxy = ProxyText {
            text: String::new(),
            kana: String::new(),
            source: Box::new(leaf),
            state: SimpleText::default(),
        };
        let first = KaniWordDispatchEnum::Kana(kana(1));
        let c = CompoundText {
            text: String::new(),
            kana: String::new(),
            primary: Box::new(first.clone()),
            words: vec![first, KaniWordDispatchEnum::Proxy(proxy)],
            score_base: None,
            score_mod: ScoreMod::Single(0),
        };
        let mut w = KaniWordDispatchEnum::Compound(c);
        set_word_conjugations(&mut w, Some(WordConjugations::Root));

        match &w {
            KaniWordDispatchEnum::Compound(c) => match &c.words[1] {
                KaniWordDispatchEnum::Proxy(p) => {
                    assert!(p.state.conjugations.is_none(), "proxy wrapper untouched");
                    match &*p.source {
                        KaniSimpleTextDispatchEnum::Kana(k) => assert_eq!(
                            k.state.conjugations,
                            Some(WordConjugations::Root),
                            "proxy source (leaf) must be mutated"
                        ),
                        _ => panic!("expected kana leaf under proxy"),
                    }
                }
                _ => panic!("expected proxy at compound[1]"),
            },
            _ => panic!("expected compound variant"),
        }
    }

    #[test]
    #[should_panic(expected = "(setf word-conjugations) on compound-text with empty words")]
    fn empty_compound_panics() {
        // `(car (last nil))` = nil; `(setf (word-conjugations nil) v)` raises
        // no-applicable-method upstream. Mirror with a panic so a silent
        // no-op can't mask a future caller that constructs a compound mid-build.
        let c = CompoundText {
            text: String::new(),
            kana: String::new(),
            primary: Box::new(KaniWordDispatchEnum::Kana(kana(1))),
            words: Vec::new(),
            score_base: None,
            score_mod: ScoreMod::Single(0),
        };
        let mut w = KaniWordDispatchEnum::Compound(c);
        set_word_conjugations(&mut w, Some(WordConjugations::Root));
    }

    #[test]
    #[should_panic(expected = "(setf word-conjugations) on counter-text")]
    fn counter_text_panics() {
        use crate::dict::counters::classes::{Common, Counter, CounterText};
        let counter = Counter::Base(CounterText {
            text: String::new(),
            kana: String::new(),
            number_text: "1".into(),
            number: 1,
            source: None,
            ordinalp: false,
            suffix: None,
            accepts_suffixes: Vec::new(),
            suffix_descriptions: Vec::new(),
            digit_opts: Vec::new(),
            common: Common::Inherit,
            allowed: Vec::new(),
            foreign: false,
        });
        let mut w = KaniWordDispatchEnum::Counter(counter);
        set_word_conjugations(&mut w, Some(WordConjugations::Root));
    }
}
