//! Port of `ichiran/dict:get-all-readings` (`dict-errata.lisp:257`).
//!
//! Returns the union of `kanji_text.text` and `kana_text.text` for a
//! single entry (by `seq`) as a list of strings.
//!
//! Diverges from the upstream lambda list `(seq)` only by taking
//! `&KaniranContext` for the database handle, replacing the upstream
//! dynamic `*connection*` per [`crate::conn::kani_context`].

use crate::conn::kani_context::KaniranContext;

pub async fn get_all_readings(
    ctx: &KaniranContext,
    seq: i32,
) -> Result<Vec<String>, sqlx::Error> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT text FROM kanji_text WHERE seq = $1 \
         UNION \
         SELECT text FROM kana_text WHERE seq = $1",
    )
    .bind(seq)
    .fetch_all(&ctx.pool)
    .await?;
    Ok(rows.into_iter().map(|(t,)| t).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// REPL probe (`(ichiran/dict::get-all-readings seq)` under
    /// `with-db`), 2026-05-31. Pins three real entries plus a
    /// guaranteed-missing seq. UNION lacks `ORDER BY` in the upstream
    /// query so the returned order is not specified — assertions
    /// compare sorted sets, the same way the probe script sorted with
    /// `(sort (copy-list …) #'string<)`.
    #[tokio::test]
    #[ignore = "DB test; requires KANIRAN_TEST_DATABASE_URL"]
    async fn get_all_readings_corpus() {
        let ctx = crate::conn::kani_context::KaniranContext::from_env()
            .await
            .expect("from_env");
        let cases: &[(i32, Vec<&str>)] = &[
            // seq 1582920 — この (with kanji writings 此の / 斯の and kana この / こん)
            (1582920, vec!["この", "こん", "斯の", "此の"]),
            // seq 1409080 — 駄弁 (with kanji writings 駄弁 / 駄辯 and kana だべん)
            (1409080, vec!["だべん", "駄弁", "駄辯"]),
            // seq 9000000 — no such entry
            (9000000, vec![]),
        ];
        for (seq, expected) in cases {
            let mut got = get_all_readings(&ctx, *seq).await.expect("query");
            got.sort();
            let mut want: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
            want.sort();
            assert_eq!(got, want, "seq={seq}");
        }
    }
}
