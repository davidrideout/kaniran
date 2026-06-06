//! Port of `ichiran/dict:suffix-sou` (`dict-grammar.lisp:454`).
//!
//! Handles ～そう (appearance), delegating to the shared
//! `suffix-sou-base` body; the score depends on the root
//! (から→40, い→0, 出来→100, else 70).

use crate::conn::kani_context::KaniranContext;
use crate::dict::compound_text_class::{CompoundText, ScoreMod};
use crate::dict::def_simple_suffix_macro::{def_simple_suffix_body, DefSimpleSuffixOpts};
use crate::dict::kana_text_dao::KanaText;
use crate::dict::suffix_sou_base_macro::suffix_sou_base_body;

pub async fn suffix_sou(
    ctx: &KaniranContext,
    root: &str,
    suf: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:455-458 (constantly (cond …)) — resolved once over `root`.
    let score_val: i64 = if root == "から" {
        40
    } else if root == "い" {
        0
    } else if root == "出来" {
        100
    } else {
        70
    };

    // dict-grammar.lisp:461 (suffix-sou-base root patch)
    let (primary_words, patch) = suffix_sou_base_body(ctx, root).await?;

    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Constant(score_val),
        connector: "",
        patch,
    };
    def_simple_suffix_body(ctx, primary_words, root, suf, kf, &opts).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::simple_text_class::SimpleText;

    /// `:sou` suffix-cache `kf` for "そう", REPL pinned via
    /// `(gethash "そう" *suffix-cache*)`: id=876, seq=1006610, text="そう",
    /// ord=0, common=0, common_tags="[ichi1]", conjugate_p=T,
    /// nokanji=NIL, best_kanji=:NULL.
    fn kf_sou() -> KanaText {
        KanaText {
            id: 876,
            seq: 1006610,
            text: "そう".into(),
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

    /// REPL: `(suffix-sou "美味し" "そう" kf-sou)` → 1 COMPOUND
    /// text="美味しそう" kana="おいしそう" score-mod=(constantly 70)
    /// primary=KANJI-TEXT (美味し id=1433173 seq=10597564), patch=nil,
    /// words=(primary, kf). Exercises the catch-all `(t 70)` arm.
    #[tokio::test]
    async fn sou1_adj_stem_kanji_score70() {
        let ctx = ctx().await;
        let kf = kf_sou();
        let result = suffix_sou(&ctx, "美味し", "そう", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "美味しそう");
        assert_eq!(c.kana, "おいしそう");
        assert!(matches!(c.score_mod, ScoreMod::Constant(70)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 1433173);
                assert_eq!(k.seq, 10597564);
                assert_eq!(k.text, "美味し");
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
    }

    /// REPL: `(suffix-sou "出来" "そう" kf-sou)` → 1 COMPOUND
    /// text="出来そう" kana="できそう" score-mod=(constantly 100)
    /// primary=KANJI-TEXT (出来 id=689432 seq=10230657). Pins the
    /// `((equal root "出来") 100)` arm.
    #[tokio::test]
    async fn sou2_dekiru_arm_score100() {
        let ctx = ctx().await;
        let kf = kf_sou();
        let result = suffix_sou(&ctx, "出来", "そう", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "出来そう");
        assert_eq!(c.kana, "できそう");
        assert!(matches!(c.score_mod, ScoreMod::Constant(100)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => assert_eq!(k.seq, 10230657),
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL: `(suffix-sou "から" "そう" kf-sou)` → 2 COMPOUNDs
    /// (text="からそう" each), both with score-mod=(constantly 40)
    /// (the `((equal root "から") 40)` arm). Primary seqs 2858914 / 10419670.
    #[tokio::test]
    async fn sou3_kara_arm_score40() {
        let ctx = ctx().await;
        let kf = kf_sou();
        let result = suffix_sou(&ctx, "から", "そう", &kf).await.unwrap();
        assert_eq!(result.len(), 2);
        for c in &result {
            assert_eq!(c.text, "からそう");
            assert!(matches!(c.score_mod, ScoreMod::Constant(40)));
            assert!(c.score_base.is_none());
            assert_eq!(c.words.len(), 2);
        }
        let seqs: Vec<i32> = result
            .iter()
            .map(|c| match &*c.primary {
                KaniWordDispatchEnum::Kana(k) => k.seq,
                _ => -1,
            })
            .collect();
        assert!(seqs.contains(&2858914));
        assert!(seqs.contains(&10419670));
    }

    /// REPL: `(suffix-sou "い" "そう" kf-sou)` → 6 COMPOUNDs
    /// text="いそう" each, score-mod=(constantly 0). Hits the
    /// `((equal root "い") 0)` arm and finds 6 い-rooted conj-stem rows.
    #[tokio::test]
    async fn sou4_i_arm_score0() {
        let ctx = ctx().await;
        let kf = kf_sou();
        let result = suffix_sou(&ctx, "い", "そう", &kf).await.unwrap();
        assert_eq!(result.len(), 6);
        for c in &result {
            assert_eq!(c.text, "いそう");
            assert_eq!(c.kana, "いそう");
            assert!(matches!(c.score_mod, ScoreMod::Constant(0)));
            assert!(c.score_base.is_none());
            assert_eq!(c.words.len(), 2);
        }
    }

    /// REPL: `(suffix-sou "な" "そう" kf-sou)` → NIL — `root` is in the
    /// `'("な" "よ" "よさ" "に" "き")` exclusion list AND doesn't end
    /// with "なさ", so suffix-sou-base's cond falls through to nil.
    #[tokio::test]
    async fn sou5_excluded_root_returns_empty() {
        let ctx = ctx().await;
        let kf = kf_sou();
        for r in ["な", "よ", "よさ", "に", "き"] {
            let result = suffix_sou(&ctx, r, "そう", &kf).await.unwrap();
            assert!(result.is_empty(), "expected NIL for root={:?}", r);
        }
    }

    /// REPL: `(suffix-sou "つまらなさ" "そう" kf-sou)` → 1 COMPOUND
    /// text="つまらなさそう" kana="つまらなさそう" smod=(constantly 70)
    /// primary=KANA-TEXT (つまらない id=1082 seq=1008190). Exercises the
    /// "なさ"-tail branch: patch=("い","さ") rewrites root to "つまらない",
    /// find-word-with-conj-prop with conj-neg filter returns 1 row, and
    /// the kana branch uses `destem(k, length("い")=1) + "さ" + suf`.
    #[tokio::test]
    async fn sou6_nasa_branch_kana() {
        let ctx = ctx().await;
        let kf = kf_sou();
        let result = suffix_sou(&ctx, "つまらなさ", "そう", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "つまらなさそう");
        assert_eq!(c.kana, "つまらなさそう");
        assert!(matches!(c.score_mod, ScoreMod::Constant(70)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.id, 1082);
                assert_eq!(k.seq, 1008190);
                assert_eq!(k.text, "つまらない");
            }
            other => panic!("expected Kana primary, got {:?}", other),
        }
    }

    /// REPL: `(suffix-sou "食べなさ" "そう" kf-sou)` → 1 COMPOUND
    /// text="食べなさそう" kana="たべなさそう" primary=KANJI-TEXT
    /// (食べない id=411231 seq=10092227). Pins the "なさ" branch on a
    /// kanji-text result.
    #[tokio::test]
    async fn sou7_nasa_branch_kanji() {
        let ctx = ctx().await;
        let kf = kf_sou();
        let result = suffix_sou(&ctx, "食べなさ", "そう", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べなさそう");
        assert_eq!(c.kana, "たべなさそう");
        assert!(matches!(c.score_mod, ScoreMod::Constant(70)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 411231);
                assert_eq!(k.seq, 10092227);
                assert_eq!(k.text, "食べない");
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }
}
