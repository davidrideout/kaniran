//! Port of `ichiran/dict:suffix-iadj` (`dict-grammar.lisp:492`).
//!
//! ```lisp
//! (def-simple-suffix suffix-iadj :iadj (:connector "" :score 1) (root)
//!   (find-word-with-conj-type root +conj-adjective-stem+))
//! ```
//!
//! `+conj-adjective-stem+` is `51` (`dict-errata.lisp:1237`); the
//! `&rest` conj-type set is `(51)`. Mapcar tail delegated to
//! [`def_simple_suffix_body`].
//!
//! Divergences from `(root sv suf)`:
//! - `suf` typed `&KanaText` (the `:iadj` cache rows are loaded by
//!   `(load-kf :iadj (get-kana-form 2006580 "げ"))` /
//!   `(load-kf :iadj (get-kana-form 1604890 "め") :class :me)` — both
//!   materialize kana-texts).
//! - `+conj-adjective-stem+` is inlined as the literal `51` since the
//!   constant has no Rust port file (precedent: `suffix_neg.rs` inlines
//!   `52` for `+conj-negative-stem+`).

use crate::conn::kani_context::KaniranContext;
use crate::dict::compound_text_class::{CompoundText, ScoreMod};
use crate::dict::def_simple_suffix_macro::{
    def_simple_suffix_body, DefSimpleSuffixOpts, PrimaryWord,
};
use crate::dict::find_word_with_conj_type::find_word_with_conj_type;
use crate::dict::kana_text_dao::KanaText;

pub async fn suffix_iadj(
    ctx: &KaniranContext,
    root: &str,
    suffix: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:493 — (find-word-with-conj-type root +conj-adjective-stem+)
    // dict-errata.lisp:1237 — (defconstant +conj-adjective-stem+ 51)
    let primary_words: Vec<PrimaryWord> = find_word_with_conj_type(ctx, root, &[51])
        .await?
        .into_iter()
        .map(PrimaryWord::from)
        .collect();

    // dict-grammar.lisp:492 — (:connector "" :score 1), :stem 0 default.
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
    use crate::dict::kani::KaniWordDispatchEnum;
    use crate::dict::simple_text_class::SimpleText;

    /// `:iadj` suffix-cache `kf` for "げ", REPL pinned: `(get-kana-form
    /// 2006580 "げ")` → id=107976, seq=2006580, text="げ", common=:NULL,
    /// common_tags="", conjugate_p=T, nokanji=nil, best_kanji="気".
    fn kf_iadj_ge() -> KanaText {
        KanaText {
            id: 107976,
            seq: 2006580,
            text: "げ".into(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: Some("気".into()),
            state: SimpleText::default(),
        }
    }

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL IADJ1: `(suffix-iadj "悲し" "げ" kf-iadj-ge)` → 1
    /// COMPOUND text="悲しげ" kana="かなしげ" score-mod=1
    /// score-base=NIL primary=KANJI-TEXT (悲し seq 10101813),
    /// words=(primary kf).
    #[tokio::test]
    async fn iadj1_kanashi_ge() {
        let ctx = ctx().await;
        let kf = kf_iadj_ge();
        let result = suffix_iadj(&ctx, "悲し", "げ", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "悲しげ");
        assert_eq!(c.kana, "かなしげ");
        assert!(matches!(c.score_mod, ScoreMod::Single(1)));
        assert!(c.score_base.is_none());
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "悲し");
                assert_eq!(k.seq, 10101813);
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

    /// REPL IADJ2: `(suffix-iadj "嬉し" "げ" kf-iadj-ge)` → 1
    /// COMPOUND text="嬉しげ" kana="うれしげ" primary=KANJI-TEXT
    /// (嬉し seq 10215030).
    #[tokio::test]
    async fn iadj2_ureshi_ge() {
        let ctx = ctx().await;
        let kf = kf_iadj_ge();
        let result = suffix_iadj(&ctx, "嬉し", "げ", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "嬉しげ");
        assert_eq!(c.kana, "うれしげ");
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "嬉し");
                assert_eq!(k.seq, 10215030);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL IADJ3: `(suffix-iadj "やわらか" "げ" kf-iadj-ge)` → 1
    /// COMPOUND text="やわらかげ" kana="やわらかげ" primary=KANA-TEXT
    /// (やわらか seq 10639355). Exercises the kana-text arm.
    #[tokio::test]
    async fn iadj3_yawaraka_ge_kana() {
        let ctx = ctx().await;
        let kf = kf_iadj_ge();
        let result = suffix_iadj(&ctx, "やわらか", "げ", &kf).await.unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "やわらかげ");
        assert_eq!(c.kana, "やわらかげ");
        assert!(matches!(c.score_mod, ScoreMod::Single(1)));
        match &*c.primary {
            KaniWordDispatchEnum::Kana(k) => {
                assert_eq!(k.text, "やわらか");
                assert_eq!(k.seq, 10639355);
            }
            other => panic!("expected Kana primary, got {:?}", other),
        }
    }

    /// REPL IADJ4: `(suffix-iadj "無理" "げ" kf-iadj-ge)` → NIL.
    /// 無理 has no conj-type-51 row.
    #[tokio::test]
    async fn iadj4_non_adjective_root() {
        let ctx = ctx().await;
        let kf = kf_iadj_ge();
        let result = suffix_iadj(&ctx, "無理", "げ", &kf).await.unwrap();
        assert!(result.is_empty());
    }
}
