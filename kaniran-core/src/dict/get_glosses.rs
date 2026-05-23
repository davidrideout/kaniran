//! Port of `ichiran/dict:get-glosses` (`dict.lisp:1892`).
//!
//! Joins `gloss` to `sense` on `gloss.sense_id = sense.id`, filters
//! `sense.seq` to the requested set, orders by `sense.seq`, then
//! groups rows by `seq` into `(seq, glosses)` pairs. Within each
//! group the inner Vec mirrors the upstream `(push text (cdar al))`
//! accumulation — texts appear in **reverse** physical-row order.
//! [`super::match_glosses::match_glosses`] reverses again before
//! scanning, matching the upstream `(loop for gloss in (nreverse
//! glosses))`.
//!
//! Diverges from the upstream lambda list `(seqs)` only by taking
//! `&KaniranContext` for the database handle, replacing the upstream
//! dynamic `*connection*` per [`crate::conn::kani_context`].

use crate::conn::kani_context::KaniranContext;

pub async fn get_glosses(
    ctx: &KaniranContext,
    seqs: &[i32],
) -> Result<Vec<(i32, Vec<String>)>, sqlx::Error> {
    let glosses: Vec<(i32, String)> = sqlx::query_as(
        "SELECT sense.seq, gloss.text FROM gloss, sense \
         WHERE sense.seq = ANY($1) AND gloss.sense_id = sense.id \
         ORDER BY sense.seq",
    )
    .bind(seqs)
    .fetch_all(&ctx.pool)
    .await?;

    let mut al: Vec<(i32, Vec<String>)> = Vec::new();
    for (seq, text) in glosses {
        // dict.lisp:1896-1899 — `if (eql (caar al) seq) do (push text (cdar al))`
        match al.last_mut() {
            Some((s, inner)) if *s == seq => inner.insert(0, text),
            _ => al.push((seq, vec![text])),
        }
    }
    Ok(al)
}

#[cfg(test)]
mod tests {
    //! Unit tests against the real .103 PG via `KaniranContext::from_env()`.
    //! REPL probe pinned 2026-05-22 using `(get-glosses ...)` after
    //! `(ichiran/conn:with-db nil ...)`. Verifies the upstream
    //! reverse-physical-row inner ordering, multi-seq outer ordering,
    //! and empty / no-rows edge cases.
    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    /// REPL: `(get-glosses '(1372640 1577100))` → outer order is
    /// ascending sense.seq; inner order is reverse physical-row.
    #[tokio::test]
    async fn multi_seq_grouping_and_inner_reversal() {
        let ctx = ctx_from_env().await;
        let out = get_glosses(&ctx, &[1372640, 1577100]).await.unwrap();
        assert_eq!(
            out,
            vec![
                (1372640, vec!["execution".to_string(), "accomplishment".to_string()]),
                (
                    1577100,
                    vec![
                        "oh (certainly not)".to_string(),
                        "why (it's nothing)".to_string(),
                        "oh, no (it's fine)".to_string(),
                        "come on!".to_string(),
                        "hey!".to_string(),
                        "huh?".to_string(),
                        "what?".to_string(),
                        "(not) in the slightest".to_string(),
                        "(not) at all".to_string(),
                        "dick".to_string(),
                        "(one's) thing".to_string(),
                        "penis".to_string(),
                        "what's-her-name".to_string(),
                        "what's-his-name".to_string(),
                        "whachamacallit".to_string(),
                        "whatsit".to_string(),
                        "that thing".to_string(),
                        "you-know-what".to_string(),
                        "what".to_string(),
                    ],
                ),
            ],
        );
    }

    /// REPL: `(get-glosses nil)` → `NIL`.
    #[tokio::test]
    async fn empty_seqs_returns_empty() {
        let ctx = ctx_from_env().await;
        let out = get_glosses(&ctx, &[]).await.unwrap();
        assert!(out.is_empty());
    }

    /// REPL: `(get-glosses '(9999999))` → single-row JMdict header
    /// gloss attached to the placeholder seq.
    #[tokio::test]
    async fn unknown_seq_returns_header_row() {
        let ctx = ctx_from_env().await;
        let out = get_glosses(&ctx, &[9999999]).await.unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].0, 9999999);
        assert_eq!(out[0].1.len(), 1);
        assert!(out[0].1[0].starts_with("Japanese-Multilingual Dictionary Project"));
    }
}
