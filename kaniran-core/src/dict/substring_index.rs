//! Port of `ichiran/dict:substring-index` (`dict.lisp:1131`).
//!
//! Re-indexes `join-substring-words`' output by each segment-list's
//! `(start, end)` slice.

use std::collections::HashMap;

use crate::conn::kani_context::KaniranContext;
use crate::dict::join_substring_words::join_substring_words;
use crate::dict::segment_list_struct::SegmentList;

pub async fn substring_index(
    ctx: &KaniranContext,
    str: &str,
) -> Result<HashMap<(usize, usize), SegmentList>, sqlx::Error> {
    let sls = join_substring_words(ctx, str).await?;
    let mut index: HashMap<(usize, usize), SegmentList> = HashMap::new();
    for sl in sls {
        index.insert((sl.start, sl.end), sl);
    }
    Ok(index)
}

#[cfg(test)]
mod tests {
    //! Every assertion is REPL-verified against the .103 SBCL via
    //! `(ichiran/dict::substring-index …)` (2026-05-25 probe).
    //! Run with `-- --test-threads=1` per the DB-test convention.
    use super::*;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
    }

    /// Per index entry: `(key, sl.start, sl.end, n_segments)`, sorted by
    /// key so the unordered hash compares deterministically.
    fn summarize(
        index: &HashMap<(usize, usize), SegmentList>,
    ) -> Vec<((usize, usize), usize, usize, usize)> {
        let mut rows: Vec<((usize, usize), usize, usize, usize)> = index
            .iter()
            .map(|(key, sl)| (*key, sl.start, sl.end, sl.segments.len()))
            .collect();
        rows.sort_unstable();
        rows
    }

    /// REPL `(substring-index "日本語")`: 5 entries; each value's
    /// start/end equals its key, segment counts match join-substring-words.
    #[tokio::test]
    async fn nihongo() {
        let ctx = ctx().await;
        let index = substring_index(&ctx, "日本語").await.unwrap();
        assert_eq!(
            summarize(&index),
            vec![
                ((0, 1), 0, 1, 2),
                ((0, 2), 0, 2, 1),
                ((0, 3), 0, 3, 1),
                ((1, 2), 1, 2, 2),
                ((2, 3), 2, 3, 1),
            ]
        );
    }

    /// REPL `(substring-index "特大")`: 2 entries.
    #[tokio::test]
    async fn tokudai() {
        let ctx = ctx().await;
        let index = substring_index(&ctx, "特大").await.unwrap();
        assert_eq!(
            summarize(&index),
            vec![((0, 2), 0, 2, 1), ((1, 2), 1, 2, 1)]
        );
    }

    /// REPL `(substring-index "5本")`: 3 entries; the counter slice
    /// `(0 2)` keeps 2 segments.
    #[tokio::test]
    async fn counter_5hon() {
        let ctx = ctx().await;
        let index = substring_index(&ctx, "5本").await.unwrap();
        assert_eq!(
            summarize(&index),
            vec![((0, 1), 0, 1, 1), ((0, 2), 0, 2, 2), ((1, 2), 1, 2, 2)]
        );
    }

    /// REPL `(substring-index "")`: empty input → empty index.
    #[tokio::test]
    async fn empty() {
        let ctx = ctx().await;
        let index = substring_index(&ctx, "").await.unwrap();
        assert!(index.is_empty());
    }
}
