//! Port of `ichiran/dict:suffix-neg` (`dict-grammar.lisp:381`).
//!
//! ```lisp
//! (def-simple-suffix suffix-neg :neg (:connector "" :score 5) (root)
//!   (find-word-with-conj-type root 13 +conj-negative-stem+))
//! ```
//!
//! `+conj-negative-stem+` is `52` (`dict-errata.lisp:1238`); the
//! `&rest`-collected conj-type set is `(13 52)`. Mapcar tail delegated
//! to [`def_simple_suffix_body`] per CONVENTIONS §4.6 case (c).
//!
//! Divergences from `(root sv suf)`:
//! - `suf` typed `&KanaText` (the `:neg` cache row is loaded by
//!   `(load-kf :neg (car (find-word-conj-of "なく" 1529520)) …)` — a
//!   kana-text).
//! - `+conj-negative-stem+` is inlined as the literal `52` since the
//!   constant has no Rust port file (precedent:
//!   `_star_weak_conj_forms_star_.rs` also inlines `52` for the same
//!   constant).

use crate::conn::kani_context::KaniranContext;
use crate::dict::compound_text_class::{CompoundText, ScoreMod};
use crate::dict::def_simple_suffix_macro::{
    def_simple_suffix_body, DefSimpleSuffixOpts, PrimaryWord,
};
use crate::dict::find_word_with_conj_type::find_word_with_conj_type;
use crate::dict::kana_text_dao::KanaText;

pub async fn suffix_neg(
    ctx: &KaniranContext,
    root: &str,
    suffix: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:382 — (find-word-with-conj-type root 13 +conj-negative-stem+)
    // dict-errata.lisp:1238 — (defconstant +conj-negative-stem+ 52)
    let primary_words: Vec<PrimaryWord> = find_word_with_conj_type(ctx, root, &[13, 52])
        .await?
        .into_iter()
        .map(PrimaryWord::from)
        .collect();

    // dict-grammar.lisp:381 — (:connector "" :score 5), :stem 0 default.
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

    /// `:neg` suffix-cache `kf`, REPL pinned: `(car (find-word-conj-of
    /// "なく" 1529520))` → id=1030305, seq=10648808, text="なく",
    /// common=:NULL, common_tags="", conjugate_p=nil, nokanji=nil,
    /// best_kanji=:NULL.
    fn kf_neg() -> KanaText {
        KanaText {
            id: 1030305,
            seq: 10648808,
            text: "なく".into(),
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

    /// REPL NEG1: `(suffix-neg "知ら" "なく" kf-neg)` → 1 COMPOUND
    /// text="知らなく" kana="しらなく" score-mod=5 primary=KANJI-TEXT
    /// (知ら seq 10106011), words=(primary kf). Hits conj-type 52.
    /// REPL-confirmed: (find-word-with-conj-type "知ら" 13) → 0,
    /// (… "知ら" 52) → 1.
    #[tokio::test]
    async fn neg1_godan_negative_stem_kanji() {
        let ctx = ctx().await;
        let kf = kf_neg();
        let result = suffix_neg(&ctx, "知ら", "なく", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "知らなく");
        assert_eq!(c.kana, "しらなく");
        assert!(matches!(c.score_mod, ScoreMod::Single(5)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "知ら");
                assert_eq!(k.seq, 10106011);
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

    /// REPL NEG2: `(suffix-neg "食べ" "なく" kf-neg)` → 1 COMPOUND
    /// text="食べなく" kana="たべなく" score-mod=5 score-base=NIL
    /// primary=KANJI-TEXT (食べ seq 10092273), words=(primary kf).
    /// Hits conj-type 13 (ichidan ren'youkei stem) — the other end
    /// of the &[13, 52] set.
    #[tokio::test]
    async fn neg2_ichidan_via_type_13() {
        let ctx = ctx().await;
        let kf = kf_neg();
        let result = suffix_neg(&ctx, "食べ", "なく", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "食べなく");
        assert_eq!(c.kana, "たべなく");
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
        match &c.words[1] {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.seq, kf.seq);
                assert_eq!(k.text, kf.text);
            }
            other => panic!("expected Kana word2 (kf), got {:?}", other),
        }
    }

    /// REPL NEG3: `(suffix-neg "無理" "なく" kf-neg)` → NIL. 無理 has
    /// neither a conj-type-13 nor a conj-type-52 row.
    #[tokio::test]
    async fn neg3_non_verb_root() {
        let ctx = ctx().await;
        let kf = kf_neg();
        let result = suffix_neg(&ctx, "無理", "なく", &kf).await.unwrap();
        assert!(result.is_empty());
    }

    /// REPL NEG4: `(suffix-neg "しら" "なく" kf-neg)` → 1 COMPOUND
    /// text="しらなく" kana="しらなく" score-mod=5 score-base=NIL
    /// primary=KANA-TEXT (しら seq 10106011), words=(primary kf).
    /// Exercises the kana-text arm of the type-52 match.
    #[tokio::test]
    async fn neg4_kana_root_negative_stem() {
        let ctx = ctx().await;
        let kf = kf_neg();
        let result = suffix_neg(&ctx, "しら", "なく", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "しらなく");
        assert_eq!(c.kana, "しらなく");
        assert!(matches!(c.score_mod, ScoreMod::Single(5)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.text, "しら");
                assert_eq!(k.seq, 10106011);
            }
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
}
