//! Port of `ichiran/dict:abbr-ii` (`dict-grammar.lisp:660-661`).
//!
//! ```lisp
//! (def-abbr-suffix abbr-ii :ii 2 (root)
//!   (find-word-full (concatenate 'string root "いい")))
//! ```
//!
//! Mapcar tail delegated to [`def_abbr_suffix_body`] (CONVENTIONS
//! §4.6 case (c)).

use crate::conn::kani_context::KaniranContext;
use crate::dict::def_abbr_suffix_macro::def_abbr_suffix_body;
use crate::dict::find_word_full::find_word_full;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kani::KaniWordDispatchEnum;

pub async fn abbr_ii(
    ctx: &KaniranContext,
    root: &str,
    suf_var: &str,
    _suf: Option<&KanaText>,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    let ctx_rebound = ctx.with_suffix_map_temp(None);
    let wordstr = format!("{}{}", root, "いい");
    let primary_words = find_word_full(&ctx_rebound, &wordstr, false, None).await?;
    def_abbr_suffix_body(&ctx_rebound, primary_words, root, suf_var, 2, None).await
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("DATABASE_URL / kaniran.toml required")
    }

    /// REPL II1: `(abbr-ii "良" "ええ" nil)` → NIL.
    /// `find-word-full "良いい"` returns no rows.
    #[tokio::test]
    async fn ii1_yoi_empty() {
        let ctx = ctx().await;
        let result = abbr_ii(&ctx, "良", "ええ", None).await.unwrap();
        assert!(result.is_empty());
    }

    /// REPL II2: `(abbr-ii "い" "ええ" nil)` → NIL.
    /// `find-word-full "いいい"` returns no rows.
    #[tokio::test]
    async fn ii2_i_empty() {
        let ctx = ctx().await;
        let result = abbr_ii(&ctx, "い", "ええ", None).await.unwrap();
        assert!(result.is_empty());
    }
}
