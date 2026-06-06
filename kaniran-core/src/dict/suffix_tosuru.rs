//! Port of `ichiran/dict:suffix-tosuru` (`dict-grammar.lisp:537`).
//!
//! Handles ～とする: looks up the root as a volitional (conj-type 9, e.g.
//! 食べよう, 飲もう, 行こう) conjugation.

use crate::conn::kani_context::KaniranContext;
use crate::dict::compound_text_class::{CompoundText, ScoreMod};
use crate::dict::def_simple_suffix_macro::{
    def_simple_suffix_body, DefSimpleSuffixOpts, PrimaryWord,
};
use crate::dict::find_word_with_conj_type::find_word_with_conj_type;
use crate::dict::kana_text_dao::KanaText;

pub async fn suffix_tosuru(
    ctx: &KaniranContext,
    root: &str,
    suffix: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:538 — (find-word-with-conj-type root 9)
    let primary_words: Vec<PrimaryWord> = find_word_with_conj_type(ctx, root, &[9])
        .await?
        .into_iter()
        .map(PrimaryWord::from)
        .collect();

    // dict-grammar.lisp:537 — (:connector " " :score 3), :stem 0 default.
    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(3),
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

    /// `:tosuru` suffix-cache `kf` for "とする" — the root kana-text of
    /// seq 2136890. REPL pinned: id=122279, seq=2136890, text="とする",
    /// common=:NULL, common_tags="[spec1]", conjugate_p=T, nokanji=nil,
    /// best_kanji=:NULL. The `(load-conjs :tosuru 2136890)` loader walks
    /// every kana-form of 2136890; we pick the root row.
    fn kf_tosuru() -> KanaText {
        KanaText {
            id: 122279,
            seq: 2136890,
            text: "とする".into(),
            ord: 0,
            common: None,
            common_tags: "[spec1]".into(),
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

    /// REPL TOSURU1: `(suffix-tosuru "食べよう" "とする" kf-tosuru)` → 1
    /// COMPOUND text="食べようとする" kana="たべよう とする"
    /// score-mod=3 score-base=NIL primary=KANJI-TEXT (食べよう seq
    /// 10092257), words=(primary kf). Note the space in kana from
    /// connector=" ".
    #[tokio::test]
    async fn tosuru1_taberu_volitional_kanji() {
        let ctx = ctx().await;
        let kf = kf_tosuru();
        let result = suffix_tosuru(&ctx, "食べよう", "とする", &kf)
            .await
            .unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べようとする");
        assert_eq!(c.kana, "たべよう とする");
        assert!(matches!(c.score_mod, ScoreMod::Single(3)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "食べよう");
                assert_eq!(k.seq, 10092257);
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

    /// REPL TOSURU2: `(suffix-tosuru "行こう" "とする" kf-tosuru)` → 1
    /// COMPOUND text="行こうとする" kana="いこう とする" score-mod=3
    /// primary=KANJI-TEXT (行こう seq 10349426).
    #[tokio::test]
    async fn tosuru2_ikou() {
        let ctx = ctx().await;
        let kf = kf_tosuru();
        let result = suffix_tosuru(&ctx, "行こう", "とする", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "行こうとする");
        assert_eq!(c.kana, "いこう とする");
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "行こう");
                assert_eq!(k.seq, 10349426);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL TOSURU3: `(suffix-tosuru "なろう" "とする" kf-tosuru)` → 3
    /// COMPOUNDs (KANA-TEXT polysemy of なろう as volitional).
    /// Each compound has text="なろうとする" kana="なろう とする"
    /// score-mod=3 primary=KANA-TEXT (なろう). Pinned seqs:
    /// 10052616, 10374864, 10549414.
    #[tokio::test]
    async fn tosuru3_narou_polysemy_three() {
        let ctx = ctx().await;
        let kf = kf_tosuru();
        let result = suffix_tosuru(&ctx, "なろう", "とする", &kf).await.unwrap();
        assert_eq!(result.len(), 3);
        for c in &result {
            assert_eq!(c.text, "なろうとする");
            assert_eq!(c.kana, "なろう とする");
            assert!(matches!(c.score_mod, ScoreMod::Single(3)));
            match &*c.primary {
                KaniWordDispatchEnum::Kana(k) => assert_eq!(k.text, "なろう"),
                other => panic!("expected Kana primary, got {:?}", other),
            }
        }
        let mut seqs: Vec<i32> = result
            .iter()
            .map(|c| match &*c.primary {
                KaniWordDispatchEnum::Kana(k) => k.seq,
                _ => unreachable!(),
            })
            .collect();
        seqs.sort();
        assert_eq!(seqs, vec![10052616, 10374864, 10549414]);
    }

    /// REPL TOSURU4: `(suffix-tosuru "無理" "とする" kf-tosuru)` → NIL.
    /// 無理 has no conj-type-9 (volitional) row.
    #[tokio::test]
    async fn tosuru4_non_verb_root() {
        let ctx = ctx().await;
        let kf = kf_tosuru();
        let result = suffix_tosuru(&ctx, "無理", "とする", &kf).await.unwrap();
        assert!(result.is_empty());
    }
}
