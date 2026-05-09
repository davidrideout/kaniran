//! Port of `ichiran/dict:seq` (gf — `dict-counters.lisp:0`).
//!
//! Returns the JMdict sequence id for a word. Most upstream classes
//! define `seq` as an auto-generated `:reader seq` slot accessor;
//! three classes override with non-trivial bodies:
//!
//! - **counter-text** (`dict-counters.lisp:79`): `(and (source obj)
//!   (seq (source obj)))` — defers to the underlying jmdict row when
//!   the counter was synthesized from one, `nil` otherwise.
//! - **proxy-text** (`dict.lisp:574`): `(seq (source obj))` —
//!   wraps the proxied row's seq directly. Source is required upstream.
//! - **compound-text** (`dict.lisp:617`): `(mapcar #'seq (words obj))`
//!   — list of seqs, one per child reading.
//!
//! Other dispatch targets (`entry`, `kana-text`, `kanji-text`, `sense`,
//! `sense-prop`, `restricted-readings`, `conjugation`,
//! `conj-source-reading`) are pure slot readers; their callsites in
//! Rust hold a statically-known DAO and read `.seq` directly without
//! routing through this dispatcher.
//!
//! ## Return type
//!
//! Single integer for the simple cases, list for compound-text. Models
//! as `Option<WordInfoSeq>` reusing the already-existing
//! [`WordInfoSeq`] enum from
//! [`super::word_info_class`] — the downstream consumer
//! ([`super::word_info_class::WordInfo::seq`]) accepts that exact
//! shape, so the dispatcher's output flows through without conversion.
//!
//! ## Compound flattening — deviation from upstream
//!
//! Lisp's `(mapcar #'seq words)` does not flatten: a compound-text
//! containing another compound-text would produce a nested list, and
//! a compound-text whose word is a sourceless counter-text would emit
//! a `nil` element. Both cases are absent in the upstream
//! `adjoin-word` callsites (`dict.lisp:632-651`) — compounds are
//! built from simple-texts only — so the Rust dispatcher one-level
//! flattens and elides `None`. If a future call sites adds nested
//! compounds the `Multi` branch would need to lift to a recursive
//! enum, but no current consumer would observe the nested shape
//! through [`WordInfoSeq`] anyway.

use crate::dict::counter_text_class::{Counter, CounterSource};
use crate::dict::compound_text_class::CompoundText;
use crate::dict::kani_word::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
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

fn seq_compound(c: &CompoundText) -> Vec<i32> {
    c.words
        .iter()
        .flat_map(|w| match seq(w) {
            Some(WordInfoSeq::Single(i)) => vec![i],
            Some(WordInfoSeq::Multi(v)) => v,
            None => Vec::new(),
        })
        .collect()
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
            Some(WordInfoSeq::Multi(vec![1, 2, 3])),
        );
    }

    #[test]
    fn compound_elides_sourceless_counter_words() {
        // counter-text without source contributes nothing — Lisp would
        // emit a `nil` element, but the Rust port flattens those out
        // to keep `WordInfoSeq::Multi(Vec<i32>)` flat (see module doc).
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
            Some(WordInfoSeq::Multi(vec![10, 20])),
        );
    }
}
