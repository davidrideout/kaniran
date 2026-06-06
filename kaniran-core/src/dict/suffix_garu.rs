//! Port of `ichiran/dict:suffix-garu` (`dict-grammar.lisp:504`).
//!
//! Handles ～がる on adjectives: for a root other than な/い/よ, looks up
//! an adjective-stem conjugation, or for a root ending in そ patches it to
//! ～そう and retries via the :sou suffix.

use crate::conn::kani_context::KaniranContext;
use crate::dict::apply_patch::apply_patch;
use crate::dict::compound_text_class::{CompoundText, ScoreMod};
use crate::dict::def_simple_suffix_macro::{
    def_simple_suffix_body, DefSimpleSuffixOpts, PrimaryWord,
};
use crate::dict::find_word_with_conj_type::find_word_with_conj_type;
use crate::dict::find_word_with_suffix::find_word_with_suffix;
use crate::dict::kana_text_dao::KanaText;

pub async fn suffix_garu(
    ctx: &KaniranContext,
    root: &str,
    suf: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:505 (unless (member root '("な" "い" "よ") …))
    let (primary_words, patch_set): (Vec<PrimaryWord>, Option<(&'static str, &'static str)>) =
        if matches!(root, "な" | "い" | "よ") {
            (Vec::new(), None)
        } else {
            // dict-grammar.lisp:506 (or (find-word-with-conj-type root +conj-adjective-stem+) …)
            // dict-errata.lisp:1237 — (defconstant +conj-adjective-stem+ 51)
            let arm_a = find_word_with_conj_type(ctx, root, &[51]).await?;
            if !arm_a.is_empty() {
                (arm_a.into_iter().map(PrimaryWord::from).collect(), None)
            } else if root.ends_with("そ") {
                // dict-grammar.lisp:507-511 (when (ends-with "そ" root)
                //   (setf patch '("う" . "")) (let ((root (apply-patch root patch))
                //                                   (*suffix-map-temp* nil))
                //     (find-word-with-suffix root :sou)))
                let patch = ("う", "");
                let new_root = apply_patch(root, patch);
                let ctx_inner = ctx.with_suffix_map_temp(None);
                let words = find_word_with_suffix(&ctx_inner, &new_root, &["sou"]).await?;
                (
                    words.into_iter().map(PrimaryWord::from).collect(),
                    Some(patch),
                )
            } else {
                (Vec::new(), None)
            }
        };

    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(0),
        connector: "",
        patch: patch_set,
    };
    def_simple_suffix_body(ctx, primary_words, root, suf, kf, &opts).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::simple_text_class::SimpleText;

    /// `:garu` cache `kf` for "がる", REPL pinned: id=72111, seq=1631750,
    /// text="がる", ord=0, common=:NULL, common_tags="[spec1]",
    /// conjugate_p=T, nokanji=NIL, best_kanji=:NULL.
    fn kf_garu() -> KanaText {
        KanaText {
            id: 72111,
            seq: 1631750,
            text: "がる".into(),
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

    /// REPL: `(suffix-garu "寒" "がる" kf)` → 1 COMPOUND text="寒がる"
    /// kana="さむがる" score-mod=0 primary=KANJI-TEXT (寒 id=148342 seq=2453760).
    /// Hits the conj-adj-stem arm with a kanji root.
    #[tokio::test]
    async fn garu1_adj_stem_kanji() {
        let ctx = ctx().await;
        let kf = kf_garu();
        let result = suffix_garu(&ctx, "寒", "がる", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "寒がる");
        assert_eq!(c.kana, "さむがる");
        assert!(matches!(c.score_mod, ScoreMod::Single(0)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 148342);
                assert_eq!(k.seq, 2453760);
                assert_eq!(k.text, "寒");
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL: `(suffix-garu "怖" "がる" kf)` → 1 COMPOUND text="怖がる"
    /// kana="こわがる" primary=KANJI-TEXT (怖 seq 2259840).
    #[tokio::test]
    async fn garu2_kowa() {
        let ctx = ctx().await;
        let kf = kf_garu();
        let result = suffix_garu(&ctx, "怖", "がる", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "怖がる");
        assert_eq!(c.kana, "こわがる");
        assert!(matches!(c.score_mod, ScoreMod::Single(0)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => assert_eq!(k.seq, 2259840),
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL: `(suffix-garu "欲し" "がる" kf)` → 1 COMPOUND text="欲しがる"
    /// kana="ほしがる" primary=KANJI-TEXT (欲し seq 10139646). Pins
    /// adj-stem on a 2-char kanji root.
    #[tokio::test]
    async fn garu3_hoshi() {
        let ctx = ctx().await;
        let kf = kf_garu();
        let result = suffix_garu(&ctx, "欲し", "がる", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "欲しがる");
        assert_eq!(c.kana, "ほしがる");
        assert!(matches!(c.score_mod, ScoreMod::Single(0)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => assert_eq!(k.seq, 10139646),
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL: `(suffix-garu "広" "がる" kf)` → 1 COMPOUND text="広がる"
    /// kana="ひろがる" primary=KANJI-TEXT (広 seq 10420123).
    #[tokio::test]
    async fn garu4_hiro() {
        let ctx = ctx().await;
        let kf = kf_garu();
        let result = suffix_garu(&ctx, "広", "がる", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "広がる");
        assert_eq!(c.kana, "ひろがる");
        assert!(matches!(c.score_mod, ScoreMod::Single(0)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => assert_eq!(k.seq, 10420123),
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL: each of `"な" "い" "よ"` → NIL via the outer
    /// `(unless (member root …))` guard.
    #[tokio::test]
    async fn garu5_member_excludes() {
        let ctx = ctx().await;
        let kf = kf_garu();
        for r in ["な", "い", "よ"] {
            let result = suffix_garu(&ctx, r, "がる", &kf).await.unwrap();
            assert!(result.is_empty(), "expected NIL for root={:?}", r);
        }
    }

    /// REPL: `(suffix-garu "食べた" "がる" kf)` and `"行きた"` → NIL.
    /// "食べた" / "行きた" are conj-type-2 (past) stems, not adj-stems
    /// (conj-type 51), and don't end with "そ"; both arms yield NIL.
    #[tokio::test]
    async fn garu6_tai_stem_no_match() {
        let ctx = ctx().await;
        let kf = kf_garu();
        for r in ["食べた", "行きた"] {
            let result = suffix_garu(&ctx, r, "がる", &kf).await.unwrap();
            assert!(result.is_empty(), "expected NIL for root={:?}", r);
        }
    }

    /// REPL: `(suffix-garu "行きそ" "がる" kf)` → 1 COMPOUND text="行きそがる"
    /// kana="いきそがる" score-mod=(0 (constantly N)) primary=KANJI-TEXT
    /// (行き seq 10349442) nwords=3. Exercises the `(ends-with "そ" root)`
    /// patch branch: patch=("う",""), new-root="行きそう",
    /// find-word-with-suffix on `:sou` returns a compound (行き+そう);
    /// adjoin-word wraps that compound with kf-garu, building text from
    /// the outer root ("行きそ"+"がる") and kana via
    /// `destem(compound-kana, length("う")=1) + "" + "" + suf` =
    /// destem("いきそう",1)+"がる"="いきそ"+"がる". Score-mod stacks the
    /// inner suffix-sou's constantly behind the integer 0.
    #[tokio::test]
    async fn garu7_so_patch_branch_kanji() {
        let ctx = ctx().await;
        let kf = kf_garu();
        let result = suffix_garu(&ctx, "行きそ", "がる", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "行きそがる");
        assert_eq!(c.kana, "いきそがる");
        // dict.lisp:651 — :score-mod stacks (list new old) when the
        // pre-existing slot was a non-list closure.
        match &c.score_mod {
            ScoreMod::Stack(v) => {
                assert_eq!(v.len(), 2);
                assert!(matches!(v[0], ScoreMod::Single(0)));
                assert!(matches!(v[1], ScoreMod::Constant(_)));
            }
            other => panic!("expected Stack score_mod, got {:?}", other),
        }
        assert_eq!(c.words.len(), 3);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => assert_eq!(k.seq, 10349442),
            other => panic!("expected Kanji primary (inner suffix-sou primary), got {:?}", other),
        }
    }

    /// REPL: `(suffix-garu "そ" "がる" kf)` → NIL. Arm A: conj-type-51 on
    /// "そ" → 0. Arm B (so-tail): new-root="そう"; find-word-with-suffix
    /// "そう" :sou → 0 because the cache has no compound suffix-class
    /// :sou entry for "そう".
    #[tokio::test]
    async fn garu8_so_only_no_match() {
        let ctx = ctx().await;
        let kf = kf_garu();
        let result = suffix_garu(&ctx, "そ", "がる", &kf).await.unwrap();
        assert!(result.is_empty());
    }
}
