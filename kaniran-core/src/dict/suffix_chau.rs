//! Port of `ichiran/dict:suffix-chau` (`dict-grammar.lisp:418`).
//!
//! ```lisp
//! (def-simple-suffix suffix-chau :chau (:stem 1 :score 5) (root suf)
//!   (let ((te (case (char suf 0)
//!               (#\HIRAGANA_LETTER_ZI "で")
//!               (#\HIRAGANA_LETTER_TI "て"))))
//!     (when te
//!       (find-word-with-conj-type (concatenate 'string root te) 3))))
//! ```
//!
//! `:stem 1` triggers the macro's `(let* ((*suffix-map-temp* nil)) …)`
//! rebind; the rebound ctx is threaded into both the primary-words
//! producer and [`def_simple_suffix_body`].
//!
//! `suf` typed `&KanaText`: every `:chau` cache entry under
//! `(load-conjs :chau …)` populates a kana-text (ちゃう id=108760 /
//! じゃう id=108761 / their conjugated forms).

use crate::conn::kani_context::KaniranContext;
use crate::dict::compound_text_class::{CompoundText, ScoreMod};
use crate::dict::def_simple_suffix_macro::{
    def_simple_suffix_body, DefSimpleSuffixOpts, PrimaryWord,
};
use crate::dict::find_word_with_conj_type::find_word_with_conj_type;
use crate::dict::kana_text_dao::KanaText;

pub async fn suffix_chau(
    ctx: &KaniranContext,
    root: &str,
    suf: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:418 — (def-simple-suffix suffix-chau :chau (:stem 1 …))
    // macro emits (let* ((*suffix-map-temp* nil)) …) for stem != 0.
    let ctx_rebound = ctx.with_suffix_map_temp(None);

    // dict-grammar.lisp:419-421 — (case (char suf 0) (#\じ "で") (#\ち "て"))
    let te = match suf.chars().next() {
        Some('じ') => Some("で"),
        Some('ち') => Some("て"),
        _ => None,
    };

    // dict-grammar.lisp:422-423 — (when te (find-word-with-conj-type (concatenate root te) 3))
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

    // dict-grammar.lisp:418 — (:stem 1 :score 5), :connector "" default.
    let opts = DefSimpleSuffixOpts {
        stem: 1,
        score: ScoreMod::Single(5),
        connector: "",
        patch: None,
    };
    def_simple_suffix_body(&ctx_rebound, primary_words, root, suf, kf, &opts).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::simple_text_class::SimpleText;

    /// `:chau` suffix-cache kf for "ちゃう", REPL pinned via
    /// `(postmodern:get-dao 'kana-text 108760)`: id=108760, seq=2013800,
    /// text="ちゃう", ord=0, common=0, common_tags="[spec1]",
    /// conjugate_p=T, nokanji=NIL, best_kanji=:NULL.
    fn kf_chau() -> KanaText {
        KanaText {
            id: 108760,
            seq: 2013800,
            text: "ちゃう".into(),
            ord: 0,
            common: Some(0),
            common_tags: "[spec1]".into(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    /// `:chau` cache kf for "じゃう", REPL pinned via
    /// `(postmodern:get-dao 'kana-text 108761)`: id=108761, seq=2013800,
    /// text="じゃう", ord=1, common=0, common_tags="[spec1]",
    /// conjugate_p=T, nokanji=NIL, best_kanji=:NULL.
    fn kf_jau() -> KanaText {
        KanaText {
            id: 108761,
            seq: 2013800,
            text: "じゃう".into(),
            ord: 1,
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

    /// REPL CHAU1: `(suffix-chau "食べ" "ちゃう" kf-chau)` → 1 COMPOUND
    /// text="食べちゃう" kana="たべちゃう" score-mod=5 score-base=NIL
    /// primary=KANJI-TEXT (食べて id=411243 seq=10092233),
    /// words=(primary, kf-chau). Exercises the ち→て arm.
    #[tokio::test]
    async fn chau1_ti_arm_kanji() {
        let ctx = ctx().await;
        let kf = kf_chau();
        let result = suffix_chau(&ctx, "食べ", "ちゃう", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べちゃう");
        assert_eq!(c.kana, "たべちゃう");
        assert!(matches!(c.score_mod, ScoreMod::Single(5)));
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

    /// REPL CHAU2: `(suffix-chau "読ん" "じゃう" kf-jau)` → 1 COMPOUND
    /// text="読んじゃう" kana="よんじゃう" score-mod=5 score-base=NIL
    /// primary=KANJI-TEXT (読んで id=431719 seq=10102130),
    /// words=(primary, kf-jau). Exercises the じ→で arm.
    #[tokio::test]
    async fn chau2_zi_arm_kanji() {
        let ctx = ctx().await;
        let kf = kf_jau();
        let result = suffix_chau(&ctx, "読ん", "じゃう", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "読んじゃう");
        assert_eq!(c.kana, "よんじゃう");
        assert!(matches!(c.score_mod, ScoreMod::Single(5)));
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

    /// REPL CHAU3: `(suffix-chau "食べ" "あう" kf-chau)` → NIL. First
    /// char あ is neither じ nor ち, so the `case` returns NIL and the
    /// outer `when te` guard suppresses the lookup.
    #[tokio::test]
    async fn chau3_other_first_char() {
        let ctx = ctx().await;
        let kf = kf_chau();
        let result = suffix_chau(&ctx, "食べ", "あう", &kf).await.unwrap();
        assert!(result.is_empty());
    }

    /// REPL CHAU4: `(suffix-chau "食べ" "じゃう" kf-jau)` → NIL.
    /// じ→で arm picks "で", but "食べで" is not a conj-type-3 form
    /// (only "食べて" is), so `find-word-with-conj-type` returns NIL.
    #[tokio::test]
    async fn chau4_de_arm_no_match() {
        let ctx = ctx().await;
        let kf = kf_jau();
        let result = suffix_chau(&ctx, "食べ", "じゃう", &kf).await.unwrap();
        assert!(result.is_empty());
    }
}
