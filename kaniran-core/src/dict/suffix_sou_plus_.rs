//! Port of `ichiran/dict:suffix-sou+` (`dict-grammar.lisp:468`).
//!
//! ```lisp
//! (def-simple-suffix suffix-sou+ :sou+ (:connector "" :score 1)
//!     (root suf patch)
//!   (suffix-sou-base root patch))
//! ```
//!
//! Same body as [`super::suffix_sou::suffix_sou`] but with `:score 1`
//! (an integer literal — [`ScoreMod::Single`]) instead of the
//! `(constantly …)` cond. Mapcar tail delegated to
//! [`def_simple_suffix_body`]; cond body delegated to
//! [`suffix_sou_base_body`].
//!
//! [`def_simple_suffix_body`]: super::def_simple_suffix_macro::def_simple_suffix_body
//! [`suffix_sou_base_body`]: super::suffix_sou_base_macro::suffix_sou_base_body
//! [`ScoreMod::Single`]: super::compound_text_class::ScoreMod::Single

use crate::conn::kani_context::KaniranContext;
use crate::dict::compound_text_class::{CompoundText, ScoreMod};
use crate::dict::def_simple_suffix_macro::{def_simple_suffix_body, DefSimpleSuffixOpts};
use crate::dict::kana_text_dao::KanaText;
use crate::dict::suffix_sou_base_macro::suffix_sou_base_body;

pub async fn suffix_sou_plus_(
    ctx: &KaniranContext,
    root: &str,
    suf: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:470 (suffix-sou-base root patch)
    let (primary_words, patch) = suffix_sou_base_body(ctx, root).await?;

    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(1),
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

    /// `:sou+` cache `kf` for "そう". The :sou+ entry shares the cache
    /// row text "そう" with :sou (the `(load-conjs :sou+ 2141080)`
    /// callsite at `dict-grammar.lisp:251` loads conjugations of
    /// そうにない / そうにありません; each load-kf overwrites the
    /// `text -> (key kf)` slot without `:join t`). The :sou+ key is
    /// observable in the cache for the conjugated forms; the base
    /// suffix kf used at runtime is the cache row registered against
    /// "そう". Pinned from REPL (cache row at id=876 seq=1006610).
    fn kf_sou_plus_() -> KanaText {
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

    /// REPL: `(suffix-sou+ "美味し" "そうにない" kf)` → 1 COMPOUND
    /// text="美味しそうにない" kana="おいしそうにない" score-mod=1
    /// primary=KANJI-TEXT (美味し id=1433173 seq=10597564). Same body
    /// as suffix-sou's catch-all arm, but with the literal `:score 1`.
    #[tokio::test]
    async fn sou_plus_1_adj_stem_kanji() {
        let ctx = ctx().await;
        let kf = kf_sou_plus_();
        let result = suffix_sou_plus_(&ctx, "美味し", "そうにない", &kf)
            .await
            .unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "美味しそうにない");
        assert_eq!(c.kana, "おいしそうにない");
        assert!(matches!(c.score_mod, ScoreMod::Single(1)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.id, 1433173);
                assert_eq!(k.seq, 10597564);
            }
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL: `(suffix-sou+ "出来" "そうにない" kf)` → 1 COMPOUND
    /// text="出来そうにない" kana="できそうにない" score-mod=1
    /// primary=KANJI-TEXT (出来 seq 10230657). Exercises the
    /// conj-adj-stem arm with a different root.
    #[tokio::test]
    async fn sou_plus_2_dekiru() {
        let ctx = ctx().await;
        let kf = kf_sou_plus_();
        let result = suffix_sou_plus_(&ctx, "出来", "そうにない", &kf)
            .await
            .unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "出来そうにない");
        assert_eq!(c.kana, "できそうにない");
        assert!(matches!(c.score_mod, ScoreMod::Single(1)));
        assert!(c.score_base.is_none());
        assert_eq!(c.words.len(), 2);
        match &*c.primary {
            KaniWordDispatchEnum::Kanji(k) => assert_eq!(k.seq, 10230657),
            other => panic!("expected Kanji primary, got {:?}", other),
        }
    }

    /// REPL: `(suffix-sou+ "つまらなさ" "そう" kf)` → 1 COMPOUND
    /// text="つまらなさそう" kana="つまらなさそう" score-mod=1
    /// primary=KANA-TEXT (つまらない id=1082 seq=1008190). Pins the
    /// "なさ"-tail branch path through suffix-sou-base with `:score 1`.
    #[tokio::test]
    async fn sou_plus_3_nasa_branch() {
        let ctx = ctx().await;
        let kf = kf_sou_plus_();
        let result = suffix_sou_plus_(&ctx, "つまらなさ", "そう", &kf)
            .await
            .unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.text, "つまらなさそう");
        assert_eq!(c.kana, "つまらなさそう");
        assert!(matches!(c.score_mod, ScoreMod::Single(1)));
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

    /// REPL: `(suffix-sou+ "な" "そうにない" kf)` → NIL — `root` "な" is
    /// in the exclusion list `'("な" "よ" "よさ" "に" "き")`.
    #[tokio::test]
    async fn sou_plus_4_excluded_root() {
        let ctx = ctx().await;
        let kf = kf_sou_plus_();
        let result = suffix_sou_plus_(&ctx, "な", "そうにない", &kf)
            .await
            .unwrap();
        assert!(result.is_empty());
    }
}
