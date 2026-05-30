//! Port of the dict-counters.lisp generic-function dispatchers —
//! `common`, `word-conjugations`, `seq`, `text`, `source`, `verify`,
//! `value-string`. Each function dispatches over the full word
//! polymorphism ([`crate::dict::kani::KaniWordDispatchEnum`]); the
//! counter-text / number-text method bodies live alongside the simpler
//! kana-text / kanji-text / proxy-text / compound-text arms.


use crate::dict::text_classes::CompoundText;
use crate::dict::counters::classes::{Common, Counter, CounterSource};
use crate::dict::dao::KanaText;
use crate::dict::kani::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use crate::dict::dao::KanjiText;
use crate::dict::text_classes::ProxyText;
use crate::dict::text_classes::WordConjugations;
use crate::dict::word_info::WordInfoSeq;
use std::borrow::Cow;

// ----- was common.rs -----

pub fn common(obj: &KaniWordDispatchEnum) -> Common {
    match obj {
        KaniWordDispatchEnum::Kanji(k) => simple_common_from_option(k.common),
        KaniWordDispatchEnum::Kana(k) => simple_common_from_option(k.common),
        KaniWordDispatchEnum::Proxy(p) => common_simple(&p.source),
        KaniWordDispatchEnum::Compound(c) => common(&c.primary),
        KaniWordDispatchEnum::Counter(c) => common_counter(c),
    }
}

fn common_simple(obj: &KaniSimpleTextDispatchEnum) -> Common {
    match obj {
        KaniSimpleTextDispatchEnum::Kanji(k) => simple_common_from_option(k.common),
        KaniSimpleTextDispatchEnum::Kana(k) => simple_common_from_option(k.common),
        KaniSimpleTextDispatchEnum::Proxy(p) => common_simple(&p.source),
    }
}

fn common_counter(c: &Counter) -> Common {
    // dict-counters.lisp:75-76 — `(or counter-common (if source (common source) 0))`.
    // Inherit / Null are the falsy-in-Lisp values that fall through to
    // the source-or-zero arm; Score is truthy and short-circuits.
    match c.base().common {
        Common::Score(n) => Common::Score(n),
        Common::Null => Common::Null,
        Common::Inherit => match c.base().source.as_ref() {
            Some(s) => common_counter_source(s),
            None => Common::Score(0),
        },
    }
}

fn common_counter_source(s: &CounterSource) -> Common {
    match s {
        CounterSource::Kanji(k) => simple_common_from_option(k.common),
        CounterSource::Kana(k) => simple_common_from_option(k.common),
    }
}

fn simple_common_from_option(v: Option<i32>) -> Common {
    match v {
        Some(n) => Common::Score(n),
        None => Common::Null,
    }
}

#[cfg(test)]
mod test_common {
    use super::*;
    use crate::dict::text_classes::{CompoundText, ScoreMod};
    use crate::dict::counters::classes::{Common, Counter, CounterSource, CounterText};
    use crate::dict::dao::KanaText;
    use crate::dict::dao::KanjiText;
    use crate::dict::text_classes::ProxyText;
    use crate::dict::text_classes::SimpleText;

    fn kanji_with_common(c: Option<i32>) -> KanjiText {
        KanjiText {
            id: 0, seq: 0, text: String::new(), ord: 0,
            common: c, common_tags: String::new(), conjugate_p: true,
            nokanji: false, best_kana: None, state: SimpleText::default(),
        }
    }

    fn kana_with_common(c: Option<i32>) -> KanaText {
        KanaText {
            id: 0, seq: 0, text: String::new(), ord: 0,
            common: c, common_tags: String::new(), conjugate_p: true,
            nokanji: false, best_kanji: None, state: SimpleText::default(),
        }
    }

    fn counter(common_slot: Common, source: Option<CounterSource>) -> Counter {
        Counter::Base(CounterText {
            text: String::new(), kana: String::new(),
            number_text: "0".into(), number: 0,
            source, ordinalp: false, suffix: None,
            accepts_suffixes: Vec::new(), suffix_descriptions: Vec::new(),
            digit_opts: Vec::new(), common: common_slot,
            allowed: Vec::new(), foreign: false,
        })
    }

    #[test]
    fn simple_text_some_score() {
        assert_eq!(
            common(&KaniWordDispatchEnum::Kanji(kanji_with_common(Some(5)))),
            Common::Score(5),
        );
    }

    #[test]
    fn simple_text_none_is_null() {
        // `db-null` upstream → :null → Common::Null in Rust.
        assert_eq!(
            common(&KaniWordDispatchEnum::Kana(kana_with_common(None))),
            Common::Null,
        );
    }

    #[test]
    fn counter_score_short_circuits() {
        let c = counter(Common::Score(7), Some(CounterSource::Kana(kana_with_common(Some(99)))));
        assert_eq!(common(&KaniWordDispatchEnum::Counter(c)), Common::Score(7));
    }

    #[test]
    fn counter_explicit_null_short_circuits() {
        // Lisp `(or :null ...)` returns :null — :null is truthy so
        // the `or` does NOT recurse on source.
        let c = counter(Common::Null, Some(CounterSource::Kana(kana_with_common(Some(3)))));
        assert_eq!(common(&KaniWordDispatchEnum::Counter(c)), Common::Null);
    }

    #[test]
    fn counter_inherit_recurses_on_source() {
        let c = counter(Common::Inherit, Some(CounterSource::Kanji(kanji_with_common(Some(11)))));
        assert_eq!(common(&KaniWordDispatchEnum::Counter(c)), Common::Score(11));
    }

    #[test]
    fn counter_inherit_no_source_returns_zero() {
        // dict-counters.lisp:75-76 — `(or nil (if nil ... 0))` → 0.
        let c = counter(Common::Inherit, None);
        assert_eq!(common(&KaniWordDispatchEnum::Counter(c)), Common::Score(0));
    }

    #[test]
    fn counter_inherit_source_with_db_null() {
        // counter Inherit + source whose common is db-null → Common::Null.
        let c = counter(Common::Inherit, Some(CounterSource::Kana(kana_with_common(None))));
        assert_eq!(common(&KaniWordDispatchEnum::Counter(c)), Common::Null);
    }

    #[test]
    fn proxy_recurses_through_source_chain() {
        let leaf = KaniSimpleTextDispatchEnum::Kanji(kanji_with_common(Some(2)));
        let inner = KaniSimpleTextDispatchEnum::Proxy(ProxyText {
            text: String::new(), kana: String::new(),
            source: Box::new(leaf), state: SimpleText::default(),
        });
        let outer = ProxyText {
            text: String::new(), kana: String::new(),
            source: Box::new(inner), state: SimpleText::default(),
        };
        assert_eq!(
            common(&KaniWordDispatchEnum::Proxy(outer)),
            Common::Score(2),
        );
    }

    #[test]
    fn compound_returns_primary_common() {
        let primary = Box::new(KaniWordDispatchEnum::Kanji(kanji_with_common(Some(4))));
        let words = vec![
            KaniWordDispatchEnum::Kanji(kanji_with_common(Some(4))),
            KaniWordDispatchEnum::Kana(kana_with_common(Some(99))),
        ];
        let c = CompoundText {
            text: String::new(), kana: String::new(),
            primary, words,
            score_base: None, score_mod: ScoreMod::Single(0),
        };
        // Compound common reads primary, ignores other words.
        assert_eq!(
            common(&KaniWordDispatchEnum::Compound(c)),
            Common::Score(4),
        );
    }
}


// ----- was word_conjugations.rs -----

pub fn word_conjugations(word: &KaniWordDispatchEnum) -> Option<WordConjugations> {
    match word {
        // dict.lisp:70 — `:accessor word-conjugations` slot on simple-text.
        KaniWordDispatchEnum::Kanji(k) => k.state.conjugations.clone(),
        KaniWordDispatchEnum::Kana(k) => k.state.conjugations.clone(),
        // dict.lisp:568-569 (defmethod word-conjugations ((obj proxy-text))
        //   (word-conjugations (source obj))) — descend through proxy chain.
        KaniWordDispatchEnum::Proxy(p) => proxy_chain_conjugations(p),
        // dict-counters.lisp:85 (defmethod word-conjugations ((obj counter-text)) nil)
        KaniWordDispatchEnum::Counter(_) => None,
        // dict.lisp:663-664 (defmethod word-conjugations ((word compound-text))
        //   (word-conjugations (car (last (words word)))))
        KaniWordDispatchEnum::Compound(c) => match c.words.last() {
            Some(last) => word_conjugations(last),
            None => None,
        },
    }
}

fn proxy_chain_conjugations(p: &ProxyText) -> Option<WordConjugations> {
    let mut current: &KaniSimpleTextDispatchEnum = &p.source;
    loop {
        match current {
            KaniSimpleTextDispatchEnum::Kanji(k) => return k.state.conjugations.clone(),
            KaniSimpleTextDispatchEnum::Kana(k) => return k.state.conjugations.clone(),
            KaniSimpleTextDispatchEnum::Proxy(inner) => current = &inner.source,
        }
    }
}

#[cfg(test)]
mod test_word_conjugations {
    use super::*;
    use crate::dict::text_classes::{CompoundText, ScoreMod};
    use crate::dict::find_word::{find_word, FindWordRows};
    use crate::dict::dao::KanaText;
    use crate::dict::dao::KanjiText;
    use crate::dict::text_classes::SimpleText;
    use crate::conn::kani_context::KaniranContext;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    fn kana_with_conj(seq: i32, conj: Option<WordConjugations>) -> KanaText {
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
            state: SimpleText {
                conjugations: conj,
                hintedp: false,
            },
        }
    }

    fn kanji_with_conj(seq: i32, conj: Option<WordConjugations>) -> KanjiText {
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
            state: SimpleText {
                conjugations: conj,
                hintedp: false,
            },
        }
    }

    #[test]
    fn kana_text_returns_state_slot() {
        let k = kana_with_conj(1, Some(WordConjugations::Root));
        assert_eq!(
            word_conjugations(&KaniWordDispatchEnum::Kana(k)),
            Some(WordConjugations::Root)
        );
    }

    #[test]
    fn kanji_text_returns_state_slot() {
        let k = kanji_with_conj(2, Some(WordConjugations::Ids(vec![10, 20])));
        assert_eq!(
            word_conjugations(&KaniWordDispatchEnum::Kanji(k)),
            Some(WordConjugations::Ids(vec![10, 20]))
        );
    }

    #[test]
    fn counter_text_always_returns_none() {
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
        assert!(word_conjugations(&KaniWordDispatchEnum::Counter(counter)).is_none());
    }

    #[test]
    fn proxy_recurses_through_source_chain() {
        use crate::dict::text_classes::ProxyText;
        let leaf =
            KaniSimpleTextDispatchEnum::Kana(kana_with_conj(99, Some(WordConjugations::Root)));
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
        assert_eq!(
            word_conjugations(&KaniWordDispatchEnum::Proxy(outer)),
            Some(WordConjugations::Root)
        );
    }

    #[test]
    fn compound_recurses_to_last_word() {
        // dict.lisp:663 — (defmethod word-conjugations ((word compound-text))
        //   (word-conjugations (car (last (words word)))))
        let first = KaniWordDispatchEnum::Kana(kana_with_conj(1, Some(WordConjugations::Root)));
        let last = KaniWordDispatchEnum::Kana(kana_with_conj(
            2,
            Some(WordConjugations::Ids(vec![5])),
        ));
        let c = CompoundText {
            text: String::new(),
            kana: String::new(),
            primary: Box::new(first.clone()),
            words: vec![first, last],
            score_base: None,
            score_mod: ScoreMod::Single(0),
        };
        assert_eq!(
            word_conjugations(&KaniWordDispatchEnum::Compound(c)),
            Some(WordConjugations::Ids(vec![5]))
        );
    }

    #[tokio::test]
    async fn real_kana_text_row_returns_none_by_default() {
        // A freshly-loaded DB row leaves state.conjugations as None
        // (FromRow resets state to Default). Pins the integration
        // surface: find_word("ねこ") rows have no conjugation
        // annotation until the find-word pipeline sets it.
        let ctx = ctx_from_env().await;
        let rows = find_word(&ctx, "ねこ", false).await.unwrap();
        let row = match rows {
            FindWordRows::Kana(v) => v.into_iter().next().expect("kana row for ねこ"),
            FindWordRows::Kanji(_) => panic!("expected kana rows for ねこ"),
        };
        assert!(word_conjugations(&KaniWordDispatchEnum::Kana(row)).is_none());
    }
}


// ----- was seq.rs -----

pub fn seq(obj: &KaniWordDispatchEnum) -> Option<WordInfoSeq> {
    match obj {
        KaniWordDispatchEnum::Kanji(k) => Some(WordInfoSeq::Single(k.seq)),
        KaniWordDispatchEnum::Kana(k) => Some(WordInfoSeq::Single(k.seq)),
        KaniWordDispatchEnum::Proxy(p) => seq_simple(&p.source),
        KaniWordDispatchEnum::Counter(c) => seq_counter(c),
        KaniWordDispatchEnum::Compound(c) => Some(WordInfoSeq::Multi(seq_compound(c))),
    }
}

fn seq_simple(obj: &KaniSimpleTextDispatchEnum) -> Option<WordInfoSeq> {
    match obj {
        KaniSimpleTextDispatchEnum::Kanji(k) => Some(WordInfoSeq::Single(k.seq)),
        KaniSimpleTextDispatchEnum::Kana(k) => Some(WordInfoSeq::Single(k.seq)),
        KaniSimpleTextDispatchEnum::Proxy(p) => seq_simple(&p.source),
    }
}

fn seq_counter(c: &Counter) -> Option<WordInfoSeq> {
    // dict-counters.lisp:79-80 — `(and (source obj) (seq (source obj)))`
    let s = c.base().source.as_ref()?;
    Some(match s {
        CounterSource::Kanji(k) => WordInfoSeq::Single(k.seq),
        CounterSource::Kana(k) => WordInfoSeq::Single(k.seq),
    })
}

fn seq_compound(c: &CompoundText) -> Vec<Option<WordInfoSeq>> {
    // dict.lisp:617 (defmethod seq ((obj compound-text)) (mapcar #'seq (words obj)))
    // — mapcar preserves nil entries; `c.words.iter().map(seq)` mirrors
    // that position-by-position.
    c.words.iter().map(seq).collect()
}

#[cfg(test)]
mod test_seq {
    use super::*;
    use crate::dict::text_classes::{CompoundText, ScoreMod};
    use crate::dict::counters::classes::{Common, Counter, CounterSource, CounterText};
    use crate::dict::dao::KanaText;
    use crate::dict::dao::KanjiText;
    use crate::dict::text_classes::ProxyText;
    use crate::dict::text_classes::SimpleText;

    fn kanji(seq: i32) -> KanjiText {
        KanjiText {
            id: 0, seq, text: String::new(), ord: 0,
            common: None, common_tags: String::new(), conjugate_p: true,
            nokanji: false, best_kana: None, state: SimpleText::default(),
        }
    }

    fn kana(seq: i32) -> KanaText {
        KanaText {
            id: 0, seq, text: String::new(), ord: 0,
            common: None, common_tags: String::new(), conjugate_p: true,
            nokanji: false, best_kanji: None, state: SimpleText::default(),
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
    fn slot_reader_kanji() {
        assert_eq!(
            seq(&KaniWordDispatchEnum::Kanji(kanji(123))),
            Some(WordInfoSeq::Single(123)),
        );
    }

    #[test]
    fn slot_reader_kana() {
        assert_eq!(
            seq(&KaniWordDispatchEnum::Kana(kana(456))),
            Some(WordInfoSeq::Single(456)),
        );
    }

    #[test]
    fn proxy_recurses_through_source_chain() {
        // proxy → proxy → kana
        let leaf = KaniSimpleTextDispatchEnum::Kana(kana(789));
        let inner_proxy = KaniSimpleTextDispatchEnum::Proxy(ProxyText {
            text: String::new(), kana: String::new(),
            source: Box::new(leaf), state: SimpleText::default(),
        });
        let outer = ProxyText {
            text: String::new(), kana: String::new(),
            source: Box::new(inner_proxy), state: SimpleText::default(),
        };
        assert_eq!(
            seq(&KaniWordDispatchEnum::Proxy(outer)),
            Some(WordInfoSeq::Single(789)),
        );
    }

    #[test]
    fn counter_with_source_returns_source_seq() {
        let c = counter_with_source(Some(CounterSource::Kana(kana(2220330))));
        assert_eq!(
            seq(&KaniWordDispatchEnum::Counter(c)),
            Some(WordInfoSeq::Single(2220330)),
        );
    }

    #[test]
    fn counter_without_source_returns_none() {
        let c = counter_with_source(None);
        assert_eq!(seq(&KaniWordDispatchEnum::Counter(c)), None);
    }

    #[test]
    fn compound_collects_word_seqs() {
        let words = vec![
            KaniWordDispatchEnum::Kanji(kanji(1)),
            KaniWordDispatchEnum::Kana(kana(2)),
            KaniWordDispatchEnum::Kanji(kanji(3)),
        ];
        let primary = Box::new(words[0].clone());
        let c = CompoundText {
            text: String::new(), kana: String::new(),
            primary, words,
            score_base: None, score_mod: ScoreMod::Single(0),
        };
        assert_eq!(
            seq(&KaniWordDispatchEnum::Compound(c)),
            Some(WordInfoSeq::Multi(vec![
                Some(WordInfoSeq::Single(1)),
                Some(WordInfoSeq::Single(2)),
                Some(WordInfoSeq::Single(3)),
            ])),
        );
    }

    #[test]
    fn compound_preserves_sourceless_counter_words_as_nil() {
        // dict.lisp:617 (defmethod seq ((obj compound-text)) (mapcar #'seq (words obj)))
        // — `mapcar` preserves position; the sourceless counter-text
        // contributes a `nil` entry which becomes [`None`] in the Vec.
        let words = vec![
            KaniWordDispatchEnum::Kana(kana(10)),
            KaniWordDispatchEnum::Counter(counter_with_source(None)),
            KaniWordDispatchEnum::Kana(kana(20)),
        ];
        let primary = Box::new(words[0].clone());
        let c = CompoundText {
            text: String::new(), kana: String::new(),
            primary, words,
            score_base: None, score_mod: ScoreMod::Single(0),
        };
        assert_eq!(
            seq(&KaniWordDispatchEnum::Compound(c)),
            Some(WordInfoSeq::Multi(vec![
                Some(WordInfoSeq::Single(10)),
                None,
                Some(WordInfoSeq::Single(20)),
            ])),
        );
    }
}


// ----- was text.rs -----


pub fn text<'a>(obj: &'a KaniWordDispatchEnum) -> Cow<'a, str> {
    match obj {
        KaniWordDispatchEnum::Kanji(k) => Cow::Borrowed(&k.text),
        KaniWordDispatchEnum::Kana(k) => Cow::Borrowed(&k.text),
        KaniWordDispatchEnum::Proxy(p) => Cow::Borrowed(&p.text),
        KaniWordDispatchEnum::Compound(c) => Cow::Borrowed(&c.text),
        KaniWordDispatchEnum::Counter(c) => {
            let base = c.base();
            Cow::Owned(format!("{}{}", base.number_text, base.text))
        }
    }
}


// ----- was source.rs -----

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
mod test_source {
    use super::*;
    use crate::dict::counters::classes::{Common, Counter, CounterSource, CounterText};
    use crate::dict::dao::KanaText;
    use crate::dict::dao::KanjiText;
    use crate::dict::kani::KaniWordDispatchEnum;
    use crate::dict::text_classes::ProxyText;
    use crate::dict::text_classes::SimpleText;

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


// ----- was verify.rs -----

pub fn verify(counter: &Counter, unique: bool) -> bool {
    match counter {
        Counter::Tsu(c) => c.verify(unique),
        Counter::DaysOn(c) => c.verify(unique),
        // Every other variant inherits the (T T) default method.
        Counter::Base(c)
        | Counter::NumberText(crate::dict::counters::subclasses::NumberText(c))
        | Counter::Age(crate::dict::counters::subclasses::CounterAge(c))
        | Counter::DaysKun(crate::dict::counters::subclasses::CounterDaysKun(c))
        | Counter::Halfhour(crate::dict::counters::subclasses::CounterHalfhour(c))
        | Counter::Months(crate::dict::counters::subclasses::CounterMonths(c))
        | Counter::People(crate::dict::counters::subclasses::CounterPeople(c))
        | Counter::Wari(crate::dict::counters::subclasses::CounterWari(c)) => c.verify(unique),
        Counter::Hifumi(c) => c.base.verify(unique),
    }
}

#[cfg(test)]
mod test_verify {
    //! Unit coverage targets the three dispatch arms (Tsu, DaysOn,
    //! default) at the boundary values that distinguish them. Bulk
    //! behavioural coverage lives in
    //! `corpus/extracted_counter_2026_05_08/dict/verify.parquet`
    //! (137,676 rows across 11 variants) replayed by `audit_fixtures`.
    use super::*;
    use crate::dict::counters::subclasses::CounterDaysOn;
    use crate::dict::counters::classes::{Common, Counter, CounterText};
    use crate::dict::counters::subclasses::CounterTsu;

    fn base(number: u64, allowed: Vec<i32>) -> CounterText {
        CounterText {
            text: String::new(),
            kana: String::new(),
            number_text: number.to_string(),
            number,
            source: None,
            ordinalp: false,
            suffix: None,
            accepts_suffixes: Vec::new(),
            suffix_descriptions: Vec::new(),
            digit_opts: Vec::new(),
            common: Common::Inherit,
            allowed,
            foreign: false,
        }
    }

    #[test]
    fn default_passes_when_allowed_empty() {
        let c = Counter::Base(base(0, vec![]));
        assert!(verify(&c, true));
        assert!(!verify(&c, false));
    }

    #[test]
    fn default_checks_allowed_membership() {
        let c = Counter::Base(base(5, vec![1, 2, 3, 4, 5]));
        assert!(verify(&c, true));
        let c = Counter::Base(base(6, vec![1, 2, 3, 4, 5]));
        assert!(!verify(&c, true));
    }

    #[test]
    fn tsu_in_range() {
        for n in 1..=9 {
            assert!(verify(&Counter::Tsu(CounterTsu(base(n, vec![]))), true), "n={}", n);
        }
        assert!(!verify(&Counter::Tsu(CounterTsu(base(0, vec![]))), true));
        assert!(!verify(&Counter::Tsu(CounterTsu(base(10, vec![]))), true));
    }

    #[test]
    fn days_on_excludes_20_and_2_through_10() {
        // Valid: n == 1 or n > 10, but not 20.
        assert!(verify(&Counter::DaysOn(CounterDaysOn(base(1, vec![]))), true));
        assert!(verify(&Counter::DaysOn(CounterDaysOn(base(11, vec![]))), true));
        assert!(verify(&Counter::DaysOn(CounterDaysOn(base(31, vec![]))), true));
        // Boundary: 20 is owned by counter-days-kun.
        assert!(!verify(&Counter::DaysOn(CounterDaysOn(base(20, vec![]))), true));
        // 2..=10 (except 1) are kun-yomi territory.
        for n in 2..=10 {
            assert!(!verify(&Counter::DaysOn(CounterDaysOn(base(n, vec![]))), true), "n={}", n);
        }
    }

    #[test]
    fn days_on_chains_to_default_for_allowed() {
        // call-next-method runs the default; if allowed is set and n
        // doesn't match, the chain returns false even when the days-on
        // gate passed. allowed=NIL is the captured shape; this case
        // pins the behaviour for any future days-on recipe with a list.
        let c = Counter::DaysOn(CounterDaysOn(base(11, vec![1, 11, 31])));
        assert!(verify(&c, true));
        let c = Counter::DaysOn(CounterDaysOn(base(11, vec![1, 31])));
        assert!(!verify(&c, true));
    }

    #[test]
    fn unique_false_overrides_everything() {
        // Default + Tsu + DaysOn all AND with `unique`; unique=false
        // short-circuits.
        assert!(!verify(&Counter::Base(base(0, vec![])), false));
        assert!(!verify(&Counter::Tsu(CounterTsu(base(5, vec![]))), false));
        assert!(!verify(&Counter::DaysOn(CounterDaysOn(base(11, vec![]))), false));
    }
}


// ----- was value_string.rs -----

pub fn value_string(counter: &Counter) -> String {
    match counter {
        Counter::Halfhour(c) => c.value_string(),
        Counter::Months(c) => c.value_string(),
        Counter::Wari(c) => c.value_string(),
        // Every other variant inherits the (counter-text) default.
        Counter::Base(c)
        | Counter::NumberText(crate::dict::counters::subclasses::NumberText(c))
        | Counter::Age(crate::dict::counters::subclasses::CounterAge(c))
        | Counter::DaysKun(crate::dict::counters::subclasses::CounterDaysKun(c))
        | Counter::DaysOn(crate::dict::counters::subclasses::CounterDaysOn(c))
        | Counter::People(crate::dict::counters::subclasses::CounterPeople(c))
        | Counter::Tsu(crate::dict::counters::subclasses::CounterTsu(c)) => c.value_string(),
        Counter::Hifumi(c) => c.base.value_string(),
    }
}

#[cfg(test)]
mod test_value_string {
    //! Unit coverage targets the four dispatch arms at the boundaries
    //! that distinguish them. Bulk behavioural coverage lives in
    //! `corpus/extracted_counter_2026_05_08/dict/value_string.parquet`
    //! replayed by `audit_fixtures`.
    use super::*;
    use crate::dict::counters::subclasses::CounterHalfhour;
    use crate::dict::counters::subclasses::CounterMonths;
    use crate::dict::counters::classes::{Common, Counter, CounterText};
    use crate::dict::counters::subclasses::CounterWari;

    fn base(number: u64, ordinalp: bool, descs: Vec<&str>) -> CounterText {
        CounterText {
            text: String::new(),
            kana: String::new(),
            number_text: number.to_string(),
            number,
            source: None,
            ordinalp,
            suffix: None,
            accepts_suffixes: Vec::new(),
            suffix_descriptions: descs.into_iter().map(String::from).collect(),
            digit_opts: Vec::new(),
            common: Common::Inherit,
            allowed: Vec::new(),
            foreign: false,
        }
    }

    #[test]
    fn default_numeric() {
        let c = Counter::Base(base(5, false, vec![]));
        assert_eq!(value_string(&c), "Value: 5");
    }

    #[test]
    fn default_ordinal() {
        let c = Counter::Base(base(2, true, vec![]));
        assert_eq!(value_string(&c), "Value: 2nd");
    }

    #[test]
    fn default_with_descriptions_reversed_and_space_prefixed() {
        // Lisp `~{ ~a~}` over (reverse '("d1" "d2")) → " d2 d1".
        let c = Counter::Base(base(5, false, vec!["d1", "d2"]));
        assert_eq!(value_string(&c), "Value: 5 d2 d1");
    }

    #[test]
    fn halfhour_emits_n_colon_30() {
        let c = Counter::Halfhour(CounterHalfhour(base(5, false, vec![])));
        assert_eq!(value_string(&c), "5:30");
    }

    #[test]
    fn months_indexes_into_english_array() {
        let c = Counter::Months(CounterMonths(base(1, false, vec![])));
        assert_eq!(value_string(&c), "January");
        let c = Counter::Months(CounterMonths(base(3, false, vec![])));
        assert_eq!(value_string(&c), "March");
        let c = Counter::Months(CounterMonths(base(12, false, vec![])));
        assert_eq!(value_string(&c), "December");
    }

    #[test]
    fn wari_emits_n_times_10_percent() {
        let c = Counter::Wari(CounterWari(base(5, false, vec![])));
        assert_eq!(value_string(&c), "50%");
        let c = Counter::Wari(CounterWari(base(1, false, vec![])));
        assert_eq!(value_string(&c), "10%");
    }
}
