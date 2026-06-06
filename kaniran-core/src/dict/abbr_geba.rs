//! Port of `ichiran/dict:abbr-geba` (`dict-grammar.lisp:635-636`).
//!
//! `:geba` abbreviated suffix: looks up `root + "げば"` and produces
//! the suffix candidates for it.

use crate::conn::kani_context::KaniranContext;
use crate::dict::def_abbr_suffix_macro::def_abbr_suffix_body;
use crate::dict::find_word_full::find_word_full;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kani_word::KaniWordDispatchEnum;

pub async fn abbr_geba(
    ctx: &KaniranContext,
    root: &str,
    suf_var: &str,
    _suf: Option<&KanaText>,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    let ctx_rebound = ctx.with_suffix_map_temp(None);
    let wordstr = format!("{}{}", root, "げば");
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

    /// REPL GEBA1: `(abbr-geba "泳" "ぎゃ" nil)` → 1 PROXY
    /// text="泳ぎゃ" kana="およぎゃ" hintedp=T source=KANJI-TEXT
    /// (seq 10485536, text "泳げば").
    #[tokio::test]
    async fn geba1_oyogu_gya() {
        let ctx = ctx().await;
        let result = abbr_geba(&ctx, "泳", "ぎゃ", None).await.unwrap();
        assert_eq!(result.len(), 1);
        let KaniWordDispatchEnum::Proxy(p) = &result[0] else {
            panic!("expected Proxy");
        };
        assert_eq!(p.text, "泳ぎゃ");
        assert_eq!(p.kana, "およぎゃ");
        assert!(p.state.hintedp);
        let KaniSimpleTextDispatchEnum::Kanji(k) = &*p.source else {
            panic!("expected Kanji source");
        };
        assert_eq!(k.seq, 10485536);
    }
}
