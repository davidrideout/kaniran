//! Port of `ichiran/dict:find-word-kana-pattern` (`dict.lisp:1877`).
//!
//! Selects every `kana_text` row whose `text` matches the POSIX regex
//! `pattern`, then stable-sorts the rows by [`compare_common`] over each
//! row's `common` rank (the `:null` sentinel sorts last).

use std::cmp::Ordering;

use super::compare_common::compare_common;
use super::kana_text_dao::KanaText;
use crate::conn::kani_context::KaniranContext;

pub async fn find_word_kana_pattern(
    ctx: &KaniranContext,
    pattern: &str,
) -> Result<Vec<KanaText>, sqlx::Error> {
    // (select-dao 'kana-text (:~ 'text pattern))
    let mut rows: Vec<KanaText> = sqlx::query_as("SELECT * FROM kana_text WHERE text ~ $1")
        .bind(pattern)
        .fetch_all(&ctx.pool)
        .await?;
    // (stable-sort … #'compare-common :key (lambda (r) (and (not (eql (common r) :null)) (common r))))
    // — `common = None` mirrors the `:null` sentinel, so the key is the
    // row's `common` slot directly.
    rows.sort_by(|a, b| {
        let key_a = a.common.map(i64::from);
        let key_b = b.common.map(i64::from);
        if compare_common(key_a, key_b).is_truthy() {
            Ordering::Less
        } else if compare_common(key_b, key_a).is_truthy() {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    });
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    /// REPL fixtures (.103, `ichiran/dict::find-word-kana-pattern`), 2026-05-25.
    /// Asserts the ordered `common` sequence each pattern yields (the
    /// values are deterministic; tied-`common` rows keep their DB scan
    /// order). `^はし$` exercises positive-ascending-then-null ordering
    /// across six homophones (5, 5, 19, null, null, null); `^あれ$`
    /// exercises the `0` rank sorting after positives but before nulls
    /// (21, 0, null, null); `^xyzzlkj$` matches nothing.
    #[tokio::test]
    async fn common_sort_order() {
        let ctx = ctx_from_env().await;
        let cases: &[(&str, &str, Vec<Option<i32>>)] = &[
            ("^はし$", "はし", vec![Some(5), Some(5), Some(19), None, None, None]),
            ("^あれ$", "あれ", vec![Some(21), Some(0), None, None]),
            ("^がっこう$", "がっこう", vec![Some(1), None]),
            ("^xyzzlkj$", "", vec![]),
        ];
        for (pattern, text, expected_commons) in cases {
            let rows = find_word_kana_pattern(&ctx, pattern).await.unwrap();
            assert!(
                rows.iter().all(|row| row.text == *text),
                "pattern={pattern:?}: every row text should be {text:?}"
            );
            let commons: Vec<Option<i32>> = rows.iter().map(|row| row.common).collect();
            assert_eq!(&commons, expected_commons, "pattern={pattern:?}");
        }
    }

    /// REPL fixtures (.103), 2026-05-25 — single-row patterns pin the
    /// exact selected row (regex select + identity sort of one element).
    #[tokio::test]
    async fn single_row_patterns() {
        let ctx = ctx_from_env().await;
        let cases: &[(&str, i32, i32, Option<i32>)] = &[
            // pattern, seq, id, common
            ("^ねこ$", 1467640, 54168, Some(7)),
            ("^きそうてんがい$", 1219430, 28651, Some(26)),
        ];
        for (pattern, seq, id, common) in cases {
            let rows = find_word_kana_pattern(&ctx, pattern).await.unwrap();
            assert_eq!(rows.len(), 1, "pattern={pattern:?}");
            assert_eq!(rows[0].seq, *seq, "pattern={pattern:?}");
            assert_eq!(rows[0].id, *id, "pattern={pattern:?}");
            assert_eq!(rows[0].common, *common, "pattern={pattern:?}");
        }
    }
}
