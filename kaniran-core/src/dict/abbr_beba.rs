//! Port of `ichiran/dict:abbr-beba` (`dict-grammar.lisp:641-642`).
//!
//! ```lisp
//! (def-abbr-suffix abbr-beba :beba 2 (root)
//!   (find-word-full (concatenate 'string root "べば")))
//! ```
//!
//! Mapcar tail delegated to [`def_abbr_suffix_body`] (CONVENTIONS
//! §4.6 case (c)).

use crate::conn::kani_context::KaniranContext;
use crate::dict::def_abbr_suffix_macro::def_abbr_suffix_body;
use crate::dict::find_word_full::find_word_full;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kani::KaniWordDispatchEnum;

pub async fn abbr_beba(
    ctx: &KaniranContext,
    root: &str,
    suf_var: &str,
    _suf: Option<&KanaText>,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    let ctx_rebound = ctx.with_suffix_map_temp(None);
    let wordstr = format!("{}{}", root, "べば");
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

    /// REPL BEBA1: `(abbr-beba "遊" "びゃ" nil)` → 2 PROXY
    /// text="遊びゃ" source=KANJI-TEXT seqs {10202469, 10225128}.
    /// 遊 has two readings → polysemy: 10202469 (kana="すさぶ") and
    /// 10225128 (kana="あそぶ"), both source-text="遊べば" but
    /// distinct source-kanas → distinct proxy kanas
    /// "すさびゃ" / "あそびゃ".
    #[tokio::test]
    async fn beba1_asobu_polysemy() {
        let ctx = ctx().await;
        let result = abbr_beba(&ctx, "遊", "びゃ", None).await.unwrap();
        assert_eq!(result.len(), 2);
        for w in &result {
            let KaniWordDispatchEnum::Proxy(p) = w else {
                panic!("expected Proxy");
            };
            assert_eq!(p.text, "遊びゃ");
            assert!(p.state.hintedp);
            let KaniSimpleTextDispatchEnum::Kanji(k) = &*p.source else {
                panic!("expected Kanji source");
            };
            assert_eq!(k.text, "遊べば");
        }
        // Two reading branches: すさぶ → "すさびゃ" and あそぶ → "あそびゃ".
        let mut kanas: Vec<String> = result
            .iter()
            .map(|w| match w {
                KaniWordDispatchEnum::Proxy(p) => p.kana.clone(),
                _ => unreachable!(),
            })
            .collect();
        kanas.sort();
        assert_eq!(kanas, vec!["あそびゃ", "すさびゃ"]);
        let mut seqs: Vec<i32> = result
            .iter()
            .map(|w| match w {
                KaniWordDispatchEnum::Proxy(p) => match &*p.source {
                    KaniSimpleTextDispatchEnum::Kanji(k) => k.seq,
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            })
            .collect();
        seqs.sort();
        assert_eq!(seqs, vec![10202469, 10225128]);
    }
}
