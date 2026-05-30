//! Polymorphic dispatcher for the upstream `seq` gf over the
//! [`KaniWordDispatchEnum`] union. The four arms:
//!
//! - `kana-text` / `kanji-text` — slot reader on
//!   [`super::kana_text_dao::KanaText::seq`] /
//!   [`super::kanji_text_dao::KanjiText::seq`].
//! - `proxy-text` (`dict.lisp:574`): `(seq (source obj))`.
//! - `counter-text` (`dict-counters.lisp:79`):
//!   `(and (source obj) (seq (source obj)))`.
//! - `compound-text` (`dict.lisp:617`): `(mapcar #'seq (words obj))`.
//!
//! Wider gf surface (`entry`, `sense`, `sense-prop`,
//! `restricted-readings`, `conjugation`, `conj-source-reading`) is out
//! of scope here — those classes carry `seq` as a slot but are never
//! dispatched through [`KaniWordDispatchEnum`] upstream; callsites
//! hold a statically-known DAO and read `.seq` directly.
//!
//! Returns `Option<WordInfoSeq>` so the [`super::word_info_class::WordInfo::seq`]
//! consumer can flow the value through without conversion.
//! Compound-text returns `Some(Multi(Vec<Option<WordInfoSeq>>))` —
//! each child's `seq` is preserved at its position, including `None`
//! for sourceless counter-text children (mirroring `(mapcar #'seq …)`
//! which puts `nil` in the list at that position).

use crate::dict::counter_text_class::{Counter, CounterSource};
use crate::dict::compound_text_class::CompoundText;
use crate::dict::kani::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use crate::dict::word_info_class::WordInfoSeq;

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
mod tests {
    use super::*;
    use crate::dict::compound_text_class::{CompoundText, ScoreMod};
    use crate::dict::counter_text_class::{Common, Counter, CounterSource, CounterText};
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::kanji_text_dao::KanjiText;
    use crate::dict::proxy_text_class::ProxyText;
    use crate::dict::simple_text_class::SimpleText;

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
