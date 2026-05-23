//! Port of `ichiran/dict:suffix-teii` (`dict-grammar.lisp:414`).
//!
//! ```lisp
//! (def-simple-suffix suffix-teii :teii (:connector " " :score 1) (root)
//!   (and (find (char root (1- (length root))) "てで")
//!        (find-word-with-conj-type root 3)))
//! ```
//!
//! Body is not `te-check`: no `(not (equal root "で"))` guard, so root
//! "で" passes and yields the bare で kana-text (seq 2028980).
//! `suf` typed `&KanaText`: both `(load-kf :teii (get-kana-form 2820690
//! "いい") …)` and `(load-kf :teii (get-kana-form 900001 "もいい") …)`
//! materialize kana-texts.

use crate::conn::kani_context::KaniranContext;
use crate::dict::compound_text_class::{CompoundText, ScoreMod};
use crate::dict::def_simple_suffix_macro::{
    def_simple_suffix_body, DefSimpleSuffixOpts, PrimaryWord,
};
use crate::dict::find_word_with_conj_type::find_word_with_conj_type;
use crate::dict::kana_text_dao::KanaText;

pub async fn suffix_teii(
    ctx: &KaniranContext,
    root: &str,
    suffix: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:415 — (find (char root (1- (length root))) "てで")
    // Upstream `(char "" -1)` signals SIMPLE-TYPE-ERROR; the
    // surrounding find-word-suffix gates with (> offset 0) so root is
    // never empty.
    let last = root
        .chars()
        .last()
        .expect("suffix-teii: (char root (1- (length root))) on empty root signals upstream");
    let primary_words: Vec<PrimaryWord> = if last == 'て' || last == 'で' {
        // dict-grammar.lisp:416 — (find-word-with-conj-type root 3)
        find_word_with_conj_type(ctx, root, &[3])
            .await?
            .into_iter()
            .map(PrimaryWord::from)
            .collect()
    } else {
        Vec::new()
    };

    // dict-grammar.lisp:414 — (:connector " " :score 1), :stem 0 default.
    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(1),
        connector: " ",
        patch: None,
    };
    def_simple_suffix_body(ctx, primary_words, root, suffix, kf, &opts).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::simple_text_class::SimpleText;

    /// `:teii` suffix-cache `kf` for "いい", REPL pinned via
    /// `(get-kana-form 2820690 "いい")`: id=201742, seq=2820690,
    /// text="いい", common=0, common_tags="[ichi1]", conjugate_p=T,
    /// nokanji=nil, best_kanji=:NULL.
    fn kf_teii_ii() -> KanaText {
        KanaText {
            id: 201742,
            seq: 2820690,
            text: "いい".into(),
            ord: 0,
            common: Some(0),
            common_tags: "[ichi1]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL TEII1: `(suffix-teii "食べて" "いい" kf-teii-ii)` → 1
    /// COMPOUND text="食べていい" kana="たべて いい" score-mod=1
    /// score-base=NIL primary=KANJI-TEXT (食べて seq 10092233).
    #[tokio::test]
    async fn teii1_tabete_ii() {
        let ctx = ctx().await;
        let kf = kf_teii_ii();
        let result = suffix_teii(&ctx, "食べて", "いい", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べていい");
        assert_eq!(c.kana, "たべて いい");
        assert!(matches!(c.score_mod, ScoreMod::Single(1)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "食べて");
                assert_eq!(k.seq, 10092233);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }

    /// REPL TEII2: `(suffix-teii "で" "いい" kf-teii-ii)` → 1 COMPOUND
    /// text="でいい" kana="で いい" score-mod=1 score-base=NIL
    /// primary=KANA-TEXT (で seq 2028980). Unlike te-check-based
    /// suffixes, suffix-teii does NOT exclude bare "で" — last char で
    /// is in "てで" and `(find-word-with-conj-type "で" 3)` returns one
    /// row.
    #[tokio::test]
    async fn teii2_bare_de_not_excluded() {
        let ctx = ctx().await;
        let kf = kf_teii_ii();
        let result = suffix_teii(&ctx, "で", "いい", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "でいい");
        assert_eq!(c.kana, "で いい");
        assert!(matches!(c.score_mod, ScoreMod::Single(1)));
        match &*c.primary {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.text, "で");
                assert_eq!(k.seq, 2028980);
            }
            other => panic!("expected Kana primary, got {:?}", other),
        }
    }

    /// REPL TEII3: `(suffix-teii "食べる" "いい" kf-teii-ii)` → NIL.
    /// Last char る not in "てで", so the `(find …)` guard fails.
    #[tokio::test]
    async fn teii3_last_char_not_te_or_de() {
        let ctx = ctx().await;
        let kf = kf_teii_ii();
        let result = suffix_teii(&ctx, "食べる", "いい", &kf).await.unwrap();
        assert!(result.is_empty());
    }
}
