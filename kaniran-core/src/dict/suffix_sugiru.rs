//! Port of `ichiran/dict:suffix-sugiru` (`dict-grammar.lisp:475`).
//!
//! ```lisp
//! (def-simple-suffix suffix-sugiru :sugiru (:stem 1 :connector "" :score 5) (root suf patch)
//!   (let ((root (cond ((equal root "い") nil)
//!                     ((or (alexandria:ends-with-subseq "なさ" root)
//!                          (alexandria:ends-with-subseq "無さ" root))
//!                      (setf patch '("い" . "さ"))
//!                      (apply-patch root patch))
//!                     (t (concatenate 'string root "い")))))
//!     (when root
//!       (cond
//!         ((and patch (> (length root) 2))
//!          (find-word-with-conj-prop root (lambda (cdata)
//!                                           (conj-neg (conj-data-prop cdata)))))
//!         (t (find-word-with-pos root "adj-i"))))))
//! ```
//!
//! `:stem 1` triggers the macro's `(let* ((*suffix-map-temp* nil)) …)`
//! rebind; the rebound ctx is threaded into the primary-words producer
//! and [`def_simple_suffix_body`].
//!
//! [`def_simple_suffix_body`]: super::def_simple_suffix_macro::def_simple_suffix_body

use crate::conn::kani_context::KaniranContext;
use crate::dict::apply_patch::apply_patch;
use crate::dict::compound_text_class::{CompoundText, ScoreMod};
use crate::dict::def_simple_suffix_macro::{
    def_simple_suffix_body, DefSimpleSuffixOpts, PrimaryWord,
};
use crate::dict::find_word_with_conj_prop::find_word_with_conj_prop;
use crate::dict::find_word_with_pos::{find_word_with_pos, WordWithPosRows};
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kani_word::KaniWordDispatchEnum;

pub async fn suffix_sugiru(
    ctx: &KaniranContext,
    root: &str,
    suf: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:475 (:stem 1) — outer rebind to nil.
    let ctx_rebound = ctx.with_suffix_map_temp(None);

    // dict-grammar.lisp:476-479 (let ((root (cond …))) …)
    let (new_root_opt, patch_set): (Option<String>, Option<(&'static str, &'static str)>) =
        if root == "い" {
            (None, None)
        } else if root.ends_with("なさ") || root.ends_with("無さ") {
            let patch = ("い", "さ");
            (Some(apply_patch(root, patch)), Some(patch))
        } else {
            (Some(format!("{}い", root)), None)
        };

    // dict-grammar.lisp:480 (when root …)
    let primary_words: Vec<PrimaryWord> = match new_root_opt {
        None => Vec::new(),
        Some(new_root) => {
            if patch_set.is_some() && new_root.chars().count() > 2 {
                // dict-grammar.lisp:482-484 (find-word-with-conj-prop root
                //   (lambda (cdata) (conj-neg (conj-data-prop cdata))))
                find_word_with_conj_prop(
                    &ctx_rebound,
                    &new_root,
                    |cd| cd.prop.as_ref().is_some_and(|p| p.neg != Some(false)),
                    false,
                )
                .await?
                .into_iter()
                .map(PrimaryWord::from)
                .collect()
            } else {
                // dict-grammar.lisp:485 (t (find-word-with-pos root "adj-i"))
                match find_word_with_pos(&ctx_rebound, &new_root, &["adj-i"]).await? {
                    WordWithPosRows::Kana(rows) => rows
                        .into_iter()
                        .map(|r| PrimaryWord::from(KaniWordDispatchEnum::Kana(r)))
                        .collect(),
                    WordWithPosRows::Kanji(rows) => rows
                        .into_iter()
                        .map(|r| PrimaryWord::from(KaniWordDispatchEnum::Kanji(r)))
                        .collect(),
                }
            }
        }
    };

    let opts = DefSimpleSuffixOpts {
        stem: 1,
        score: ScoreMod::Single(5),
        connector: "",
        patch: patch_set,
    };
    def_simple_suffix_body(&ctx_rebound, primary_words, root, suf, kf, &opts).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::simple_text_class::SimpleText;

    /// `:sugiru` cache `kf` for "すぎる", REPL pinned: id=26253,
    /// seq=1195970, text="すぎる", ord=0, common=34,
    /// common_tags="[ichi1][news2][nf34]", conjugate_p=T, nokanji=NIL,
    /// best_kanji=:NULL.
    fn kf_sugiru() -> KanaText {
        KanaText {
            id: 26253,
            seq: 1195970,
            text: "すぎる".into(),
            ord: 0,
            common: Some(34),
            common_tags: "[ichi1][news2][nf34]".into(),
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

    /// REPL: `(suffix-sugiru "高" "すぎる" kf)` → 1 COMPOUND
    /// text="高すぎる" kana="たかすぎる" score-mod=5 primary=KANJI-TEXT
    /// (高い id=18690 seq=1283190). Exercises the `(t (concatenate root "い"))`
    /// branch → find-word-with-pos "高い" "adj-i". kana="たかい",
    /// destem(kana,1)="たか", + "" + "すぎる" = "たかすぎる".
    #[tokio::test]
    async fn sugiru1_adj_i_short_root() {
        let ctx = ctx().await;
        let kf = kf_sugiru();
        let result = suffix_sugiru(&ctx, "高", "すぎる", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "高すぎる");
        assert_eq!(c.kana, "たかすぎる");
        assert!(matches!(c.score_mod, ScoreMod::Single(5)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 18690);
                assert_eq!(k.seq, 1283190);
                assert_eq!(k.text, "高い");
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL: `(suffix-sugiru "つまらな" "すぎる" kf)` → 1 COMPOUND
    /// text="つまらなすぎる" kana="つまらなすぎる" primary=KANA-TEXT
    /// (つまらない seq 1008190). Else-branch (no patch): new-root
    /// "つまらない", find-word-with-pos "adj-i".
    #[tokio::test]
    async fn sugiru2_adj_i_kana_root() {
        let ctx = ctx().await;
        let kf = kf_sugiru();
        let result = suffix_sugiru(&ctx, "つまらな", "すぎる", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "つまらなすぎる");
        assert_eq!(c.kana, "つまらなすぎる");
        assert!(matches!(c.score_mod, ScoreMod::Single(5)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.id, 1082);
                assert_eq!(k.seq, 1008190);
            }
            other => panic!("expected Kana primary, got {:?}", other),
        }
    }

    /// REPL: `(suffix-sugiru "つまらなさ" "すぎる" kf)` → 1 COMPOUND
    /// text="つまらなさすぎる" kana="つまらなさすぎる" primary=KANA-TEXT
    /// (つまらない seq 1008190). Patch branch (length new-root=5 > 2):
    /// patch=("い","さ"), new-root="つまらない", find-word-with-conj-prop
    /// conj-neg → 1 row. Kana=destem("つまらない",1)+"さ"+""+"すぎる" =
    /// "つまらな"+"さ"+"すぎる".
    #[tokio::test]
    async fn sugiru3_nasa_tail_long_conj_prop_branch() {
        let ctx = ctx().await;
        let kf = kf_sugiru();
        let result = suffix_sugiru(&ctx, "つまらなさ", "すぎる", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "つまらなさすぎる");
        assert_eq!(c.kana, "つまらなさすぎる");
        assert!(matches!(c.score_mod, ScoreMod::Single(5)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kana(k) => assert_eq!(k.seq, 1008190),
            other => panic!("expected Kana primary, got {:?}", other),
        }
    }

    /// REPL: `(suffix-sugiru "無さ" "すぎる" kf)` → 1 COMPOUND
    /// text="無さすぎる" kana="なさすぎる" primary=KANJI-TEXT
    /// (無い id=49726 seq=1529520). Patch branch falls through because
    /// length new-root=2 ≤ 2 → find-word-with-pos "無い" "adj-i".
    /// Kana=destem("ない",1)+"さ"+""+"すぎる"="な"+"さ"+"すぎる".
    #[tokio::test]
    async fn sugiru4_nasa_kanji_short_falls_to_pos() {
        let ctx = ctx().await;
        let kf = kf_sugiru();
        let result = suffix_sugiru(&ctx, "無さ", "すぎる", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "無さすぎる");
        assert_eq!(c.kana, "なさすぎる");
        assert!(matches!(c.score_mod, ScoreMod::Single(5)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 49726);
                assert_eq!(k.seq, 1529520);
                assert_eq!(k.text, "無い");
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL: `(suffix-sugiru "無" "すぎる" kf)` → 1 COMPOUND
    /// text="無すぎる" kana="なすぎる" primary=KANJI-TEXT (無い seq 1529520).
    /// Else-branch (no patch): new-root "無い".
    #[tokio::test]
    async fn sugiru5_kanji_short_else_branch() {
        let ctx = ctx().await;
        let kf = kf_sugiru();
        let result = suffix_sugiru(&ctx, "無", "すぎる", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "無すぎる");
        assert_eq!(c.kana, "なすぎる");
        assert!(matches!(c.score_mod, ScoreMod::Single(5)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => assert_eq!(k.seq, 1529520),
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL: `(suffix-sugiru "美味し" "すぎる" kf)` → 1 COMPOUND
    /// text="美味しすぎる" kana="おいしすぎる" primary=KANJI-TEXT
    /// (美味しい id=44494 seq=1486650).
    #[tokio::test]
    async fn sugiru6_oishii() {
        let ctx = ctx().await;
        let kf = kf_sugiru();
        let result = suffix_sugiru(&ctx, "美味し", "すぎる", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "美味しすぎる");
        assert_eq!(c.kana, "おいしすぎる");
        assert!(matches!(c.score_mod, ScoreMod::Single(5)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 44494);
                assert_eq!(k.seq, 1486650);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL: `(suffix-sugiru "い" "すぎる" kf)` → NIL — first-branch
    /// `((equal root "い") nil)` short-circuits the outer `when root`.
    #[tokio::test]
    async fn sugiru7_i_root_returns_nil() {
        let ctx = ctx().await;
        let kf = kf_sugiru();
        let result = suffix_sugiru(&ctx, "い", "すぎる", &kf).await.unwrap();
        assert!(result.is_empty());
    }

    /// REPL: `(suffix-sugiru "食べ" "すぎる" kf)` and `"やり"` and `"行か"`
    /// all → NIL — else-branch new-root ("食べい"/"やりい"/"行かい") is not
    /// an adj-i lemma.
    #[tokio::test]
    async fn sugiru8_non_adj_else_returns_empty() {
        let ctx = ctx().await;
        let kf = kf_sugiru();
        for r in ["食べ", "やり", "行か"] {
            let result = suffix_sugiru(&ctx, r, "すぎる", &kf).await.unwrap();
            assert!(result.is_empty(), "expected NIL for root={:?}", r);
        }
    }
}
