//! Port of `ichiran/dict:abbr-seba` (`dict-grammar.lisp:647-648`).
//!
//! ```lisp
//! (def-abbr-suffix abbr-seba :seba 2 (root)
//!   (find-word-full (concatenate 'string root "せば")))
//! ```
//!
//! Mapcar tail delegated to [`def_abbr_suffix_body`] (CONVENTIONS
//! §4.6 case (c)).

use crate::conn::kani_context::KaniranContext;
use crate::dict::def_abbr_suffix_macro::def_abbr_suffix_body;
use crate::dict::find_word_full::find_word_full;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kani_word::KaniWordDispatchEnum;

pub async fn abbr_seba(
    ctx: &KaniranContext,
    root: &str,
    suf_var: &str,
    _suf: Option<&KanaText>,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    let ctx_rebound = ctx.with_suffix_map_temp(None);
    let wordstr = format!("{}{}", root, "せば");
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

    /// REPL SEBA1: `(abbr-seba "話" "しゃ" nil)` → 1 PROXY
    /// text="話しゃ" kana="はなしゃ" hintedp=T source=KANJI-TEXT
    /// (seq 10143263, text "話せば").
    #[tokio::test]
    async fn seba1_hanasu_sha() {
        let ctx = ctx().await;
        let result = abbr_seba(&ctx, "話", "しゃ", None).await.unwrap();
        assert_eq!(result.len(), 1);
        let KaniWordDispatchEnum::Proxy(p) = &result[0] else {
            panic!("expected Proxy");
        };
        assert_eq!(p.text, "話しゃ");
        assert_eq!(p.kana, "はなしゃ");
        assert!(p.state.hintedp);
        let KaniSimpleTextDispatchEnum::Kanji(k) = &*p.source else {
            panic!("expected Kanji source");
        };
        assert_eq!(k.seq, 10143263);
    }
}
