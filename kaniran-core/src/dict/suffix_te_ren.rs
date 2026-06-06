//! Port of `ichiran/dict:suffix-te-ren` (`dict-grammar.lisp:407`).
//!
//! Handles a te-form continuative auxiliary: for a root other than で,
//! looks up conj-type 3 if it ends in て/で, else conj-type 13 (unless
//! the root is い).

use crate::conn::kani_context::KaniranContext;
use crate::dict::compound_text_class::{CompoundText, ScoreMod};
use crate::dict::def_simple_suffix_macro::{
    def_simple_suffix_body, DefSimpleSuffixOpts, PrimaryWord,
};
use crate::dict::find_word_with_conj_type::find_word_with_conj_type;
use crate::dict::kana_text_dao::KanaText;

pub async fn suffix_te_ren(
    ctx: &KaniranContext,
    root: &str,
    suffix: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:408 — (not (equal root "で"))
    let primary_words: Vec<PrimaryWord> = if root == "で" {
        Vec::new()
    } else {
        // dict-grammar.lisp:409 — (find (char root (1- (length root))) "てで")
        // Upstream `(char "" -1)` signals SIMPLE-TYPE-ERROR; the
        // surrounding find-word-suffix gates with (> offset 0) so the
        // root is never empty.
        let last = root
            .chars()
            .last()
            .expect("suffix-te-ren: (char root (1- (length root))) on empty root signals upstream");
        if last == 'て' || last == 'で' {
            // dict-grammar.lisp:410 — (find-word-with-conj-type root 3)
            find_word_with_conj_type(ctx, root, &[3])
                .await?
                .into_iter()
                .map(PrimaryWord::from)
                .collect()
        } else if root != "い" {
            // dict-grammar.lisp:411-412 — (not (member root '("い") :test 'equal))
            //                              (find-word-with-conj-type root 13)
            find_word_with_conj_type(ctx, root, &[13])
                .await?
                .into_iter()
                .map(PrimaryWord::from)
                .collect()
        } else {
            // Upstream `cond` falls through to nil when both arms fail.
            Vec::new()
        }
    };

    // dict-grammar.lisp:407 — (:connector "" :score 4), :stem 0 default.
    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(4),
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

    /// `:teren` suffix-cache `kf` for "やがって", REPL pinned via the
    /// `(load-conjs :teren 1012740 :yagaru)` populator: id=597027,
    /// seq=10285137, text="やがって", common=:NULL, common_tags="",
    /// conjugate_p=nil, nokanji=nil, best_kanji=:NULL.
    fn kf_te_ren_yagatte() -> KanaText {
        KanaText {
            id: 597027,
            seq: 10285137,
            text: "やがって".into(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
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

    /// REPL TEREN1: `(suffix-te-ren "食べて" "やがって" kf-yagatte)` →
    /// 1 COMPOUND text="食べてやがって" kana="たべてやがって"
    /// score-mod=4 score-base=NIL primary=KANJI-TEXT (食べて seq
    /// 10092233). Last char て → conj-type 3 arm.
    #[tokio::test]
    async fn teren1_tabete_te_arm_conj_3() {
        let ctx = ctx().await;
        let kf = kf_te_ren_yagatte();
        let result = suffix_te_ren(&ctx, "食べて", "やがって", &kf)
            .await
            .unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べてやがって");
        assert_eq!(c.kana, "たべてやがって");
        assert!(matches!(c.score_mod, ScoreMod::Single(4)));
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

    /// REPL TEREN2: `(suffix-te-ren "食べ" "やがって" kf-yagatte)` →
    /// 1 COMPOUND text="食べやがって" kana="たべやがって" score-mod=4
    /// score-base=NIL primary=KANJI-TEXT (食べ seq 10092273). Last
    /// char べ ≠ て/で, root ≠ "い" → conj-type 13 arm (ren'youkei
    /// stem).
    #[tokio::test]
    async fn teren2_tabe_stem_arm_conj_13() {
        let ctx = ctx().await;
        let kf = kf_te_ren_yagatte();
        let result = suffix_te_ren(&ctx, "食べ", "やがって", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べやがって");
        assert_eq!(c.kana, "たべやがって");
        assert!(matches!(c.score_mod, ScoreMod::Single(4)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "食べ");
                assert_eq!(k.seq, 10092273);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL TEREN3: `(suffix-te-ren "い" "やがって" kf-yagatte)` → NIL.
    /// Root "い" is excluded from the conj-type 13 arm by the
    /// `(not (member root '("い") :test 'equal))` guard, and last char
    /// い ≠ て/で so the conj-type 3 arm doesn't fire either.
    #[tokio::test]
    async fn teren3_i_excluded() {
        let ctx = ctx().await;
        let kf = kf_te_ren_yagatte();
        let result = suffix_te_ren(&ctx, "い", "やがって", &kf).await.unwrap();
        assert!(result.is_empty());
    }

    /// REPL TEREN4: `(suffix-te-ren "で" "やがって" kf-yagatte)` → NIL.
    /// Outer `(not (equal root "で"))` guard excludes bare で.
    #[tokio::test]
    async fn teren4_bare_de_excluded() {
        let ctx = ctx().await;
        let kf = kf_te_ren_yagatte();
        let result = suffix_te_ren(&ctx, "で", "やがって", &kf).await.unwrap();
        assert!(result.is_empty());
    }

    /// REPL TEREN5: `(suffix-te-ren "無理" "やがって" kf-yagatte)` →
    /// NIL. 無理 is not a verb stem; find-word-with-conj-type returns
    /// 0 rows for conj-type 13.
    #[tokio::test]
    async fn teren5_non_verb_root() {
        let ctx = ctx().await;
        let kf = kf_te_ren_yagatte();
        let result = suffix_te_ren(&ctx, "無理", "やがって", &kf)
            .await
            .unwrap();
        assert!(result.is_empty());
    }
}
