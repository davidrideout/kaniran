//! Port of `ichiran/dict:suffix-tai` (`dict-grammar.lisp:370`).
//!
//! ```lisp
//! (def-simple-suffix suffix-tai :tai (:connector "" :score 5) (root)
//!   (unless (member root '("い") :test 'equal)
//!     (find-word-with-conj-type root 13)))
//! ```
//!
//! Mapcar tail delegated to [`def_simple_suffix_body`] per CONVENTIONS
//! §4.6 case (c).
//!
//! Divergences from `(root sv suf)`:
//! - `suf` typed `&KanaText` (the `:tai` cache rows are loaded by
//!   `(load-conjs :tai 2017560)` / `(load-kf :tai (get-kana-form 900000
//!   "たそう") …)` — both materialize kana-texts).

use crate::conn::kani_context::KaniranContext;
use crate::dict::compound_text_class::{CompoundText, ScoreMod};
use crate::dict::def_simple_suffix_macro::{
    def_simple_suffix_body, DefSimpleSuffixOpts, PrimaryWord,
};
use crate::dict::find_word_with_conj_type::find_word_with_conj_type;
use crate::dict::kana_text_dao::KanaText;

pub async fn suffix_tai(
    ctx: &KaniranContext,
    root: &str,
    suffix: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:371 — (unless (member root '("い") :test 'equal) …)
    let primary_words: Vec<PrimaryWord> = if root == "い" {
        Vec::new()
    } else {
        // dict-grammar.lisp:372 — (find-word-with-conj-type root 13)
        find_word_with_conj_type(ctx, root, &[13])
            .await?
            .into_iter()
            .map(PrimaryWord::from)
            .collect()
    };

    // dict-grammar.lisp:370 — (:connector "" :score 5), :stem 0 default.
    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(5),
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

    /// `:tai` suffix-cache `kf`, REPL pinned: `(get-kana-form 2017560
    /// "たい")` → id=109172, seq=2017560, text="たい", common=0,
    /// common_tags="[spec1]", conjugate_p=T, nokanji=nil,
    /// best_kanji=:NULL, hintedp=nil.
    fn kf_tai() -> KanaText {
        KanaText {
            id: 109172,
            seq: 2017560,
            text: "たい".into(),
            ord: 0,
            common: Some(0),
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

    /// REPL TAI1: `(suffix-tai "食べ" "たい" kf-tai)` → 1 COMPOUND
    /// text="食べたい" kana="たべたい" score-mod=5 primary=KANJI-TEXT
    /// (食べ seq 10092273), words=(primary kf), score-base=NIL.
    #[tokio::test]
    async fn tai1_ichidan_ren_youkei_kanji() {
        let ctx = ctx().await;
        let kf = kf_tai();
        let result = suffix_tai(&ctx, "食べ", "たい", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べたい");
        assert_eq!(c.kana, "たべたい");
        assert!(matches!(c.score_mod, ScoreMod::Single(5)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "食べ");
                assert_eq!(k.seq, 10092273);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        assert_eq!(c.words.len(), 2);
        // dict.lisp:644 — (:words (list word1 word2)) — word2 is kf wrapped.
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }

    /// REPL TAI2: `(suffix-tai "い" "たい" kf-tai)` → NIL. The
    /// `(member root '("い") :test 'equal)` guard excludes bare い.
    #[tokio::test]
    async fn tai2_i_excluded() {
        let ctx = ctx().await;
        let kf = kf_tai();
        let result = suffix_tai(&ctx, "い", "たい", &kf).await.unwrap();
        assert!(result.is_empty());
    }

    /// REPL TAI3: `(suffix-tai "無理" "たい" kf-tai)` → NIL. 無理 is
    /// not a verb stem; find-word-with-conj-type returns 0 rows.
    #[tokio::test]
    async fn tai3_non_verb_root() {
        let ctx = ctx().await;
        let kf = kf_tai();
        let result = suffix_tai(&ctx, "無理", "たい", &kf).await.unwrap();
        assert!(result.is_empty());
    }

    /// REPL TAI4: `(suffix-tai "飲み" "たい" kf-tai)` → 1 COMPOUND
    /// text="飲みたい" kana="のみたい" score-mod=5 score-base=NIL
    /// primary=KANJI-TEXT (飲み seq 10665871), words=(primary kf).
    /// Exercises a godan ren'youkei stem.
    #[tokio::test]
    async fn tai4_godan_ren_youkei_kanji() {
        let ctx = ctx().await;
        let kf = kf_tai();
        let result = suffix_tai(&ctx, "飲み", "たい", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "飲みたい");
        assert_eq!(c.kana, "のみたい");
        assert!(matches!(c.score_mod, ScoreMod::Single(5)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "飲み");
                assert_eq!(k.seq, 10665871);
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

    /// REPL TAI5: `(suffix-tai "のみ" "たい" kf-tai)` → 3 COMPOUNDs
    /// (KANA-TEXT arm of find-word-with-conj-type — three distinct
    /// kana_text rows of のみ as ren'youkei stem). Each compound has
    /// text="のみたい" kana="のみたい", a KANA-TEXT primary with
    /// text="のみ" / get-kana="のみ", and words=(primary kf). The
    /// three seqs are 10433818, 10577483, 10665871.
    #[tokio::test]
    async fn tai5_kana_ren_youkei_polysemy_three() {
        let ctx = ctx().await;
        let kf = kf_tai();
        let result = suffix_tai(&ctx, "のみ", "たい", &kf).await.unwrap();
        assert_eq!(result.len(), 3);
        for c in &result {
            assert_eq!(c.text, "のみたい");
            assert_eq!(c.kana, "のみたい");
            assert!(matches!(c.score_mod, ScoreMod::Single(5)));
            assert!(c.score_base.is_none());
            match &*c.primary {
                KaniWordDispatchEnum::Kana(k) => assert_eq!(k.text, "のみ"),
                other => panic!("expected Kana primary, got {:?}", other),
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
        let mut seqs: Vec<i32> = result
            .iter()
            .map(|c| match &*c.primary {
                KaniWordDispatchEnum::Kana(k) => k.seq,
                _ => unreachable!(),
            })
            .collect();
        seqs.sort();
        assert_eq!(seqs, vec![10433818, 10577483, 10665871]);
    }
}
