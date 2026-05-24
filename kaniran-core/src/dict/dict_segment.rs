//! Port of `ichiran/dict:dict-segment` (`dict.lisp:1450`).
//!
//! ```lisp
//! (defun dict-segment (str &key (limit 5))
//!   (with-connection *connection*
//!     (loop for (path . score) in (find-best-path (join-substring-words str) (length str) :limit limit)
//!          collect (cons (fill-segment-path str path) score))))
//! ```
//!
//! Diverges from the upstream lambda list `(str &key (limit 5))` by taking
//! `&KaniranContext` for the database handle, replacing the upstream
//! `(with-connection *connection*)` per [`crate::conn::kani_context`]. The
//! `&key (limit 5)` becomes `Option<usize>`; each `(word-info-list . score)`
//! cons becomes a `(Vec<WordInfo>, i32)` tuple.

use crate::conn::kani_context::KaniranContext;
use crate::dict::fill_segment_path::fill_segment_path;
use crate::dict::find_best_path::find_best_path;
use crate::dict::join_substring_words::join_substring_words;
use crate::dict::word_info_class::WordInfo;

pub async fn dict_segment(
    ctx: &KaniranContext,
    str: &str,
    limit: Option<usize>,
) -> Result<Vec<(Vec<WordInfo>, i32)>, sqlx::Error> {
    let limit = limit.unwrap_or(5);

    // (find-best-path (join-substring-words str) (length str) :limit limit)
    let mut segment_lists = join_substring_words(ctx, str).await?;
    let best_paths = find_best_path(ctx, &mut segment_lists, str.chars().count(), Some(limit)).await?;

    // (loop for (path . score) in ... collect (cons (fill-segment-path str path) score))
    let mut result = Vec::with_capacity(best_paths.len());
    for (mut path, score) in best_paths {
        let word_info_list = fill_segment_path(ctx, str, &mut path).await?;
        result.push((word_info_list, score));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    //! Unit tests against the real .103 PG via `KaniranContext::from_env()`.
    //! Expected paths / scores captured from `ichiran/dict:dict-segment` on
    //! the capture host. Coverage:
    //! - multi-path result (loop runs N times), scores descending
    //! - `:limit` caps the number of paths and is forwarded to find-best-path
    //! - default limit (None) resolves to 5
    //! - empty string yields one seed path with an empty word-info-list
    //! - all-gap input yields one path with the gap-penalty score
    use super::*;
    use crate::dict::word_info_class::WordInfoType;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    fn texts(word_info_list: &[WordInfo]) -> Vec<String> {
        word_info_list
            .iter()
            .map(|wi| {
                if wi.kind == WordInfoType::Gap {
                    ":GAP".to_string()
                } else {
                    wi.text.clone()
                }
            })
            .collect()
    }

    #[tokio::test]
    async fn multi_path_scores_descending() {
        let ctx = ctx_from_env().await;
        let result = dict_segment(&ctx, "しませんか", Some(3)).await.unwrap();
        let scores: Vec<i32> = result.iter().map(|(_, score)| *score).collect();
        assert_eq!(scores, vec![352, 52, 48]);
        assert_eq!(texts(&result[0].0), vec!["しません", "か"]);
        assert_eq!(texts(&result[1].0), vec!["しま", "せんか"]);
        assert_eq!(texts(&result[2].0), vec!["しま", "せん", "か"]);
    }

    #[tokio::test]
    async fn limit_one_returns_single_best_path() {
        let ctx = ctx_from_env().await;
        let result = dict_segment(&ctx, "しませんか", Some(1)).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, 352);
        assert_eq!(texts(&result[0].0), vec!["しません", "か"]);
    }

    #[tokio::test]
    async fn default_limit_is_five() {
        let ctx = ctx_from_env().await;
        let result = dict_segment(&ctx, "ご注文はうさぎですか", None).await.unwrap();
        let scores: Vec<i32> = result.iter().map(|(_, score)| *score).collect();
        assert_eq!(scores, vec![518, 504, 485, 474, 465]);
        assert_eq!(texts(&result[0].0), vec!["ご注文", "は", "うさぎ", "です", "か"]);
    }

    #[tokio::test]
    async fn empty_string_seeds_one_empty_path() {
        let ctx = ctx_from_env().await;
        let result = dict_segment(&ctx, "", Some(5)).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, 0);
        assert!(result[0].0.is_empty());
    }

    #[tokio::test]
    async fn all_gap_input_one_path_with_gap_penalty() {
        let ctx = ctx_from_env().await;
        let result = dict_segment(&ctx, "abcde", Some(5)).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, -2500);
        assert_eq!(texts(&result[0].0), vec![":GAP"]);
    }
}
