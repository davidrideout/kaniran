//! Port of `ichiran/dict:abbr-keba` (`dict-grammar.lisp:632-633`).
//!
//! ```lisp
//! (def-abbr-suffix abbr-keba :keba 2 (root)
//!   (find-word-full (concatenate 'string root "けば")))
//! ```
//!
//! Mapcar tail delegated to [`def_abbr_suffix_body`] (CONVENTIONS
//! §4.6 case (c)).

use crate::conn::kani_context::KaniranContext;
use crate::dict::def_abbr_suffix_macro::def_abbr_suffix_body;
use crate::dict::find_word_full::find_word_full;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kani::KaniWordDispatchEnum;

pub async fn abbr_keba(
    ctx: &KaniranContext,
    root: &str,
    suf_var: &str,
    _suf: Option<&KanaText>,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    let ctx_rebound = ctx.with_suffix_map_temp(None);
    let wordstr = format!("{}{}", root, "けば");
    let primary_words = find_word_full(&ctx_rebound, &wordstr, false, None).await?;
    def_abbr_suffix_body(&ctx_rebound, primary_words, root, suf_var, 2, None).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::kani::KaniSimpleTextDispatchEnum;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL KEBA1: `(abbr-keba "書" "きゃ" nil)` → 1 PROXY
    /// text="書きゃ" kana="かきゃ" hintedp=T source=KANJI-TEXT
    /// (seq 10526936, text "書けば").
    #[tokio::test]
    async fn keba1_kaku_kya() {
        let ctx = ctx().await;
        let result = abbr_keba(&ctx, "書", "きゃ", None).await.unwrap();
        assert_eq!(result.len(), 1);
        let KaniWordDispatchEnum::Proxy(p) = &result[0] else {
            panic!("expected Proxy");
        };
        assert_eq!(p.text, "書きゃ");
        assert_eq!(p.kana, "かきゃ");
        assert!(p.state.hintedp);
        let KaniSimpleTextDispatchEnum::Kanji(k) = &*p.source else {
            panic!("expected Kanji source");
        };
        assert_eq!(k.seq, 10526936);
    }
}
