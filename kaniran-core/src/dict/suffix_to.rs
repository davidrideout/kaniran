//! Port of `ichiran/dict:suffix-to` (`dict-grammar.lisp:425`).
//!
//! ```lisp
//! (def-simple-suffix suffix-to :to (:stem 1 :score 0) (root suf)
//!   (let ((te (case (char suf 0)
//!               (#\HIRAGANA_LETTER_TO "て")
//!               (#\HIRAGANA_LETTER_DO "で"))))
//!     (when te
//!       (find-word-with-conj-type (concatenate 'string root te) 3))))
//! ```
//!
//! `:stem 1` triggers the macro's `(let* ((*suffix-map-temp* nil)) …)`
//! rebind; the rebound ctx is threaded into both the primary-words
//! producer and [`def_simple_suffix_body`].
//!
//! `suf` typed `&KanaText`: every `:to` cache entry under
//! `(load-conjs :to …)` populates a kana-text (とく id=119112 /
//! どく id=119113 / their conjugated forms).

use crate::conn::kani_context::KaniranContext;
use crate::dict::compound_text_class::{CompoundText, ScoreMod};
use crate::dict::def_simple_suffix_macro::{
    def_simple_suffix_body, DefSimpleSuffixOpts, PrimaryWord,
};
use crate::dict::find_word_with_conj_type::find_word_with_conj_type;
use crate::dict::kana_text_dao::KanaText;

pub async fn suffix_to(
    ctx: &KaniranContext,
    root: &str,
    suf: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:425 — (def-simple-suffix suffix-to :to (:stem 1 …))
    // macro emits (let* ((*suffix-map-temp* nil)) …) for stem != 0.
    let ctx_rebound = ctx.with_suffix_map_temp(None);

    // dict-grammar.lisp:426-428 — (case (char suf 0) (#\と "て") (#\ど "で"))
    let te = match suf.chars().next() {
        Some('と') => Some("て"),
        Some('ど') => Some("で"),
        _ => None,
    };

    // dict-grammar.lisp:429-430 — (when te (find-word-with-conj-type (concatenate root te) 3))
    let primary_words: Vec<PrimaryWord> = match te {
        Some(te) => {
            let word = format!("{}{}", root, te);
            find_word_with_conj_type(&ctx_rebound, &word, &[3])
                .await?
                .into_iter()
                .map(PrimaryWord::from)
                .collect()
        }
        None => Vec::new(),
    };

    // dict-grammar.lisp:425 — (:stem 1 :score 0), :connector "" default.
    let opts = DefSimpleSuffixOpts {
        stem: 1,
        score: ScoreMod::Single(0),
        connector: "",
        patch: None,
    };
    def_simple_suffix_body(&ctx_rebound, primary_words, root, suf, kf, &opts).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::kani::KaniWordDispatchEnum;
    use crate::dict::simple_text_class::SimpleText;

    /// `:to` cache kf for "とく", REPL pinned via
    /// `(postmodern:get-dao 'kana-text 119112)`: id=119112, seq=2108590,
    /// text="とく", ord=0, common=0, common_tags="[spec1]",
    /// conjugate_p=T, nokanji=NIL, best_kanji=:NULL.
    fn kf_toku() -> KanaText {
        KanaText {
            id: 119112,
            seq: 2108590,
            text: "とく".into(),
            ord: 0,
            common: Some(0),
            common_tags: "[spec1]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    /// `:to` cache kf for "どく", REPL pinned via
    /// `(postmodern:get-dao 'kana-text 119113)`: id=119113, seq=2108590,
    /// text="どく", ord=1, common=:NULL, common_tags="",
    /// conjugate_p=T, nokanji=NIL, best_kanji=:NULL.
    fn kf_doku() -> KanaText {
        KanaText {
            id: 119113,
            seq: 2108590,
            text: "どく".into(),
            ord: 1,
            common: None,
            common_tags: String::new(),
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

    /// REPL TO1: `(suffix-to "食べ" "とく" kf-toku)` → 1 COMPOUND
    /// text="食べとく" kana="たべとく" score-mod=0 score-base=NIL
    /// primary=KANJI-TEXT (食べて id=411243 seq=10092233),
    /// words=(primary, kf-toku). Exercises the と→て arm.
    #[tokio::test]
    async fn to1_to_arm_kanji() {
        let ctx = ctx().await;
        let kf = kf_toku();
        let result = suffix_to(&ctx, "食べ", "とく", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べとく");
        assert_eq!(c.kana, "たべとく");
        assert!(matches!(c.score_mod, ScoreMod::Single(0)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 411243);
                assert_eq!(k.seq, 10092233);
                assert_eq!(k.text, "食べて");
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

    /// REPL TO2: `(suffix-to "読ん" "どく" kf-doku)` → 1 COMPOUND
    /// text="読んどく" kana="よんどく" score-mod=0 score-base=NIL
    /// primary=KANJI-TEXT (読んで id=431719 seq=10102130),
    /// words=(primary, kf-doku). Exercises the ど→で arm.
    #[tokio::test]
    async fn to2_do_arm_kanji() {
        let ctx = ctx().await;
        let kf = kf_doku();
        let result = suffix_to(&ctx, "読ん", "どく", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "読んどく");
        assert_eq!(c.kana, "よんどく");
        assert!(matches!(c.score_mod, ScoreMod::Single(0)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 431719);
                assert_eq!(k.seq, 10102130);
                assert_eq!(k.text, "読んで");
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
        // adjoin_word puts word1 at words[0] (dict.lisp:644 — `(list word1 word2)`).
        assert_eq!(c.words.len(), 2);
        match &c.words[0] {
            KaniWordDispatchEnum::Kanji(k) => assert_eq!(k.id, 431719),
            other => panic!("expected Kanji words[0] (primary), got {:?}", other),
        }
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }

    /// REPL TO3: `(suffix-to "食べ" "あく" kf-toku)` → NIL. First char
    /// あ is neither と nor ど, so the `case` returns NIL.
    #[tokio::test]
    async fn to3_other_first_char() {
        let ctx = ctx().await;
        let kf = kf_toku();
        let result = suffix_to(&ctx, "食べ", "あく", &kf).await.unwrap();
        assert!(result.is_empty());
    }

    /// REPL TO4: `(suffix-to "のん" "どく" kf-doku)` → 3 COMPOUNDs
    /// (kana-text arm of find-word-with-conj-type — three distinct
    /// kana_text のんで rows). Each compound has text="のんどく"
    /// kana="のんどく", KANA-TEXT primary with text "のんで". Seqs:
    /// 10433774, 10577439, 10665827; ids: 773379, 945133, 1050587.
    #[tokio::test]
    async fn to4_polysemy_kana_three() {
        let ctx = ctx().await;
        let kf = kf_doku();
        let result = suffix_to(&ctx, "のん", "どく", &kf).await.unwrap();
        assert_eq!(result.len(), 3);
        for c in &result {
            assert_eq!(c.text, "のんどく");
            assert_eq!(c.kana, "のんどく");
            assert!(matches!(c.score_mod, ScoreMod::Single(0)));
            assert!(c.score_base.is_none());
            match &*c.primary {
                KaniWordDispatchEnum::Kana(k) => assert_eq!(k.text, "のんで"),
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
        let mut got: Vec<(i32, i32)> = result
            .iter()
            .map(|c| match &*c.primary {
                KaniWordDispatchEnum::Kana(k) => (k.id, k.seq),
                _ => unreachable!(),
            })
            .collect();
        got.sort();
        assert_eq!(
            got,
            vec![(773379, 10433774), (945133, 10577439), (1050587, 10665827)]
        );
    }
}
