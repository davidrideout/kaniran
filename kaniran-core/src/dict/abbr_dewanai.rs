//! Port of `ichiran/dict:abbr-dewanai` (`dict-grammar.lisp:618-619`).
//!
//! ```lisp
//! (def-abbr-suffix abbr-dewanai :dewanai 4 (root)
//!   (find-word-full (concatenate 'string root "ではない")))
//! ```
//!
//! Mapcar tail delegated to [`def_abbr_suffix_body`] (CONVENTIONS
//! §4.6 case (c)).

use crate::conn::kani_context::KaniranContext;
use crate::dict::def_abbr_suffix_macro::def_abbr_suffix_body;
use crate::dict::find_word_full::find_word_full;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kani::KaniWordDispatchEnum;

pub async fn abbr_dewanai(
    ctx: &KaniranContext,
    root: &str,
    suf_var: &str,
    _suf: Option<&KanaText>,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    let ctx_rebound = ctx.with_suffix_map_temp(None);
    let wordstr = format!("{}{}", root, "ではない");
    let primary_words = find_word_full(&ctx_rebound, &wordstr, false, None).await?;
    def_abbr_suffix_body(&ctx_rebound, primary_words, root, suf_var, 4, None).await
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL DEWANAI1: `(abbr-dewanai "犬" "じゃない" nil)` → NIL.
    /// `find-word-full "犬ではない"` returns no rows (it's a phrase,
    /// not an entry / conjugated form), so primary-words is empty
    /// and mapcar yields nil.
    #[tokio::test]
    async fn dewanai1_inu_dehanai_empty() {
        let ctx = ctx().await;
        let result = abbr_dewanai(&ctx, "犬", "じゃない", None).await.unwrap();
        assert!(result.is_empty());
    }
}
