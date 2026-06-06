//! Port of `ichiran/dict:short-sense-str` (`dict.lisp:1562`).
//!
//! Returns the joined gloss string of the first sense (lowest `ord`)
//! for `seq`, optionally restricted to senses tagged with the given
//! part of speech.

use crate::conn::kani_context::KaniranContext;

pub async fn short_sense_str(
    ctx: &KaniranContext,
    seq: i32,
    with_pos: Option<&str>,
) -> Result<Option<String>, sqlx::Error> {
    // dict.lisp:1562 — `,@(if with-pos …)` splices the sense-prop join
    // only when with-pos is supplied.
    let single: Option<Option<String>> = match with_pos {
        Some(with_pos) => {
            sqlx::query_scalar(
                "SELECT (SELECT string_agg(gloss.text, '; ' ORDER BY gloss.ord) \
                         FROM gloss WHERE gloss.sense_id = sense.id) \
                 FROM sense \
                 INNER JOIN sense_prop AS pos \
                   ON (pos.sense_id = sense.id AND pos.tag = 'pos' AND pos.text = $2) \
                 WHERE sense.seq = $1 \
                 GROUP BY sense.id \
                 ORDER BY sense.ord \
                 LIMIT 1",
            )
            .bind(seq)
            .bind(with_pos)
            .fetch_optional(&ctx.pool)
            .await?
        }
        None => {
            sqlx::query_scalar(
                "SELECT (SELECT string_agg(gloss.text, '; ' ORDER BY gloss.ord) \
                         FROM gloss WHERE gloss.sense_id = sense.id) \
                 FROM sense \
                 WHERE sense.seq = $1 \
                 GROUP BY sense.id \
                 ORDER BY sense.ord \
                 LIMIT 1",
            )
            .bind(seq)
            .fetch_optional(&ctx.pool)
            .await?
        }
    };
    Ok(single.flatten())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    /// REPL fixtures (.103, `ichiran/dict::short-sense-str`), 2026-05-24.
    /// Covers no with-pos (first sense's glosses), with-pos matching a
    /// sense's pos, with-pos with no matching sense (→ nil), and an
    /// unknown seq (→ nil), across a noun entry (1582710 日本) and a
    /// verb entry (1358280 食べる, pos v1).
    #[tokio::test]
    async fn short_sense_str_fixtures() {
        let ctx = ctx_from_env().await;
        let cases: &[(i32, Option<&str>, Option<&str>)] = &[
            (1582710, None, Some("Japan")),
            (1358280, None, Some("to eat")),
            (1358280, Some("v1"), Some("to eat")),
            (1358280, Some("n"), None),
            (1582710, Some("v1"), None),
            (1582710, Some("n"), Some("Japan")),
            (999999, None, None),
            (999999, Some("v1"), None),
        ];
        for (seq, with_pos, expected) in cases {
            assert_eq!(
                short_sense_str(&ctx, *seq, *with_pos)
                    .await
                    .unwrap()
                    .as_deref(),
                *expected,
                "seq={seq} with_pos={with_pos:?}"
            );
        }
    }
}
