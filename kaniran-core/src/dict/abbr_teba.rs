//! Port of `ichiran/dict:abbr-teba` (`dict-grammar.lisp:626-627`).
//!
//! Matches the spoken abbreviation of `root + "てば"` (e.g. 立ちゃ
//! for 立てば).

use crate::conn::kani_context::KaniranContext;
use crate::dict::def_abbr_suffix_macro::def_abbr_suffix_body;
use crate::dict::find_word_full::find_word_full;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kani_word::KaniWordDispatchEnum;

pub async fn abbr_teba(
    ctx: &KaniranContext,
    root: &str,
    suf_var: &str,
    _suf: Option<&KanaText>,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    let ctx_rebound = ctx.with_suffix_map_temp(None);
    let wordstr = format!("{}{}", root, "てば");
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

    /// REPL TEBA1: `(abbr-teba "立" "ちゃ" nil)` → 1 PROXY
    /// text="立ちゃ" kana="たちゃ" hintedp=T source=KANJI-TEXT
    /// (seq 10435976, text "立てば").
    #[tokio::test]
    async fn teba1_tatsu_cha() {
        let ctx = ctx().await;
        let result = abbr_teba(&ctx, "立", "ちゃ", None).await.unwrap();
        assert_eq!(result.len(), 1);
        let KaniWordDispatchEnum::Proxy(p) = &result[0] else {
            panic!("expected Proxy");
        };
        assert_eq!(p.text, "立ちゃ");
        assert_eq!(p.kana, "たちゃ");
        assert!(p.state.hintedp);
        let KaniSimpleTextDispatchEnum::Kanji(k) = &*p.source else {
            panic!("expected Kanji source");
        };
        assert_eq!(k.seq, 10435976);
    }

    /// REPL TEBA2: `(abbr-teba "勝" "ちゃ" nil)` → 1 PROXY
    /// text="勝ちゃ" kana="かちゃ" source=KANJI-TEXT (seq 10316061).
    #[tokio::test]
    async fn teba2_katsu_cha() {
        let ctx = ctx().await;
        let result = abbr_teba(&ctx, "勝", "ちゃ", None).await.unwrap();
        assert_eq!(result.len(), 1);
        let KaniWordDispatchEnum::Proxy(p) = &result[0] else {
            panic!("expected Proxy");
        };
        assert_eq!(p.text, "勝ちゃ");
        assert_eq!(p.kana, "かちゃ");
        let KaniSimpleTextDispatchEnum::Kanji(k) = &*p.source else {
            panic!("expected Kanji source");
        };
        assert_eq!(k.seq, 10316061);
    }
}
