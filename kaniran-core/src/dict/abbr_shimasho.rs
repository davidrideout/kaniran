//! Port of `ichiran/dict:abbr-shimasho` (`dict-grammar.lisp:615-616`).
//!
//! ```lisp
//! (def-abbr-suffix abbr-shimasho :shimashou 5 (root)
//!   (find-word-full (concatenate 'string root "しましょう")))
//! ```
//!
//! Mapcar tail delegated to [`def_abbr_suffix_body`] (CONVENTIONS
//! §4.6 case (c)).

use crate::conn::kani_context::KaniranContext;
use crate::dict::def_abbr_suffix_macro::def_abbr_suffix_body;
use crate::dict::find_word_full::find_word_full;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kani::KaniWordDispatchEnum;

pub async fn abbr_shimasho(
    ctx: &KaniranContext,
    root: &str,
    suf_var: &str,
    _suf: Option<&KanaText>,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    let ctx_rebound = ctx.with_suffix_map_temp(None);
    let wordstr = format!("{}{}", root, "しましょう");
    let primary_words = find_word_full(&ctx_rebound, &wordstr, false, None).await?;
    def_abbr_suffix_body(&ctx_rebound, primary_words, root, suf_var, 5, None).await
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL SHIMASHO1: `(abbr-shimasho "勉強" "しましょ" nil)` → 1
    /// COMPOUND text="勉強しましょ" kana="べんきょう しましょ".
    /// Exercises the compound-text branch of the etypecase: input
    /// is `find-word-full "勉強しましょう"` → compound from
    /// `suffix-suru` with text="勉強しましょう", kana=
    /// "べんきょう しましょう"; the abbr body mutates text/kana
    /// in-place to the abbreviated forms. primary, words, score_mod,
    /// score_base on the compound are NOT touched
    /// (dict-grammar.lisp:575-578 only `setf`s `stext` and `skana`).
    #[tokio::test]
    async fn shimasho1_benkyou_shimasho_compound_branch() {
        use crate::dict::kani::KaniWordDispatchEnum as W;
        let ctx = ctx().await;
        let result = abbr_shimasho(&ctx, "勉強", "しましょ", None).await.unwrap();
        assert_eq!(result.len(), 1);
        let W::Compound(c) = &result[0] else {
            panic!("expected Compound");
        };
        assert_eq!(c.text, "勉強しましょ");
        assert_eq!(c.kana, "べんきょう しましょ");
        // primary unchanged from the suffix-suru compound.
        let W::Kanji(primary) = &*c.primary else {
            panic!("expected Kanji primary");
        };
        assert_eq!(primary.seq, 1512670);
        assert_eq!(primary.text, "勉強");
        // words unchanged: [KANJI-TEXT 勉強, KANA-TEXT しましょう].
        assert_eq!(c.words.len(), 2);
        let W::Kanji(w0) = &c.words[0] else {
            panic!("expected words[0] Kanji");
        };
        assert_eq!(w0.seq, 1512670);
        assert_eq!(w0.text, "勉強");
        let W::Kana(w1) = &c.words[1] else {
            panic!("expected words[1] Kana");
        };
        assert_eq!(w1.seq, 10152277);
        assert_eq!(w1.text, "しましょう");
    }
}
