//! Port of the `word-conjugations` accessor on `simple-text`
//! (`dict.lisp:70`), with proxy/counter/compound overrides.
//!
//! Returns a reading's `word-conjugations` slot — recursing through
//! proxy chains and into a compound's last word, nil for counters.

use crate::dict::kani_word::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use crate::dict::proxy_text_class::ProxyText;
use crate::dict::simple_text_class::WordConjugations;

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
mod tests {
    use super::*;
    use crate::dict::compound_text_class::{CompoundText, ScoreMod};
    use crate::dict::find_word::{find_word, FindWordRows};
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::kanji_text_dao::KanjiText;
    use crate::dict::simple_text_class::SimpleText;
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
        use crate::dict::counter_text_class::{Common, Counter, CounterText};
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
        use crate::dict::proxy_text_class::ProxyText;
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
