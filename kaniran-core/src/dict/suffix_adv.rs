//! Port of `ichiran/dict:suffix-adv` (`dict-grammar.lisp:464`).
//!
//! ```lisp
//! (def-simple-suffix suffix-adv :adv (:connector "" :score 1) (root)
//!   (find-word-with-conj-type root +conj-adverbial+))
//! ```
//!
//! `+conj-adverbial+` is `50` (`dict-errata.lisp:1236`); the `&rest`
//! conj-type set is `(50)`. Mapcar tail delegated to
//! [`def_simple_suffix_body`].
//!
//! Divergences from `(root sv suf)`:
//! - `suf` typed `&KanaText` (the `:adv` cache rows are loaded by
//!   `(load-conjs :adv 1375610 :naru)` — all 211 conjugations of なる
//!   materialize as kana-texts).
//! - `+conj-adverbial+` is inlined as the literal `50` since the
//!   constant has no Rust port file (precedent: `suffix_neg.rs` inlines
//!   `52` for `+conj-negative-stem+`).

use crate::conn::kani_context::KaniranContext;
use crate::dict::compound_text_class::{CompoundText, ScoreMod};
use crate::dict::def_simple_suffix_macro::{
    def_simple_suffix_body, DefSimpleSuffixOpts, PrimaryWord,
};
use crate::dict::find_word_with_conj_type::find_word_with_conj_type;
use crate::dict::kana_text_dao::KanaText;

pub async fn suffix_adv(
    ctx: &KaniranContext,
    root: &str,
    suffix: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:465 — (find-word-with-conj-type root +conj-adverbial+)
    // dict-errata.lisp:1236 — (defconstant +conj-adverbial+ 50)
    let primary_words: Vec<PrimaryWord> = find_word_with_conj_type(ctx, root, &[50])
        .await?
        .into_iter()
        .map(PrimaryWord::from)
        .collect();

    // dict-grammar.lisp:464 — (:connector "" :score 1), :stem 0 default.
    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(1),
        connector: "",
        patch: None,
    };
    def_simple_suffix_body(ctx, primary_words, root, suffix, kf, &opts).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::simple_text_class::SimpleText;

    /// `:adv` suffix-cache `kf` for "なる" — the root kana-text of seq
    /// 1375610. REPL pinned: id=44705, seq=1375610, text="なる",
    /// common=34, common_tags="[ichi1][news2][nf34]", conjugate_p=T,
    /// nokanji=nil, best_kanji="成る". The `(load-conjs :adv 1375610
    /// :naru)` loader walks every kana-form of 1375610 (211 rows); we
    /// pick the root row as a representative.
    fn kf_adv_naru() -> KanaText {
        KanaText {
            id: 44705,
            seq: 1375610,
            text: "なる".into(),
            ord: 0,
            common: Some(34),
            common_tags: "[ichi1][news2][nf34]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: Some("成る".into()),
            state: SimpleText::default(),
        }
    }

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL ADV1: `(suffix-adv "正しく" "なる" kf-adv-naru)` → 1
    /// COMPOUND text="正しくなる" kana="ただしくなる" score-mod=1
    /// score-base=NIL primary=KANJI-TEXT (正しく seq 2827272),
    /// words=(primary kf).
    #[tokio::test]
    async fn adv1_tadashiku_naru() {
        let ctx = ctx().await;
        let kf = kf_adv_naru();
        let result = suffix_adv(&ctx, "正しく", "なる", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "正しくなる");
        assert_eq!(c.kana, "ただしくなる");
        assert!(matches!(c.score_mod, ScoreMod::Single(1)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "正しく");
                assert_eq!(k.seq, 2827272);
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

    /// REPL ADV2: `(suffix-adv "大きく" "なる" kf-adv-naru)` → 1
    /// COMPOUND text="大きくなる" kana="おおきくなる" score-mod=1
    /// primary=KANJI-TEXT (大きく seq 10563301).
    #[tokio::test]
    async fn adv2_ookiku_naru() {
        let ctx = ctx().await;
        let kf = kf_adv_naru();
        let result = suffix_adv(&ctx, "大きく", "なる", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "大きくなる");
        assert_eq!(c.kana, "おおきくなる");
        assert!(matches!(c.score_mod, ScoreMod::Single(1)));
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "大きく");
                assert_eq!(k.seq, 10563301);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL ADV3: `(suffix-adv "無理" "なる" kf-adv-naru)` → NIL.
    /// 無理 has no conj-type-50 row.
    #[tokio::test]
    async fn adv3_non_adverbial_root() {
        let ctx = ctx().await;
        let kf = kf_adv_naru();
        let result = suffix_adv(&ctx, "無理", "なる", &kf).await.unwrap();
        assert!(result.is_empty());
    }

    /// REPL ADV4: `(suffix-adv "ジャバスクリプト" "なる" kf-adv-naru)` →
    /// NIL. Word with no conjugations.
    #[tokio::test]
    async fn adv4_no_conjugation_root() {
        let ctx = ctx().await;
        let kf = kf_adv_naru();
        let result = suffix_adv(&ctx, "ジャバスクリプト", "なる", &kf)
            .await
            .unwrap();
        assert!(result.is_empty());
    }
}
