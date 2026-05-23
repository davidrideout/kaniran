//! Port of `ichiran/dict:abbr-meba` (`dict-grammar.lisp:644-645`).
//!
//! ```lisp
//! (def-abbr-suffix abbr-meba :meba 2 (root)
//!   (find-word-full (concatenate 'string root "めば")))
//! ```
//!
//! Mapcar tail delegated to [`def_abbr_suffix_body`] (CONVENTIONS
//! §4.6 case (c)).

use crate::conn::kani_context::KaniranContext;
use crate::dict::def_abbr_suffix_macro::def_abbr_suffix_body;
use crate::dict::find_word_full::find_word_full;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kani_word::KaniWordDispatchEnum;

pub async fn abbr_meba(
    ctx: &KaniranContext,
    root: &str,
    suf_var: &str,
    _suf: Option<&KanaText>,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    let ctx_rebound = ctx.with_suffix_map_temp(None);
    let wordstr = format!("{}{}", root, "めば");
    let primary_words = find_word_full(&ctx_rebound, &wordstr, false, None).await?;
    def_abbr_suffix_body(&ctx_rebound, primary_words, root, suf_var, 2, None).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::kani_word::KaniSimpleTextDispatchEnum;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL MEBA1: `(abbr-meba "飲" "みゃ" nil)` → 1 PROXY
    /// text="飲みゃ" kana="のみゃ" hintedp=T source=KANJI-TEXT
    /// (seq 10665831, text "飲めば").
    #[tokio::test]
    async fn meba1_nomu_mya() {
        let ctx = ctx().await;
        let result = abbr_meba(&ctx, "飲", "みゃ", None).await.unwrap();
        assert_eq!(result.len(), 1);
        let KaniWordDispatchEnum::Proxy(p) = &result[0] else {
            panic!("expected Proxy");
        };
        assert_eq!(p.text, "飲みゃ");
        assert_eq!(p.kana, "のみゃ");
        assert!(p.state.hintedp);
        let KaniSimpleTextDispatchEnum::Kanji(k) = &*p.source else {
            panic!("expected Kanji source");
        };
        assert_eq!(k.seq, 10665831);
    }
}
