//! Port of `ichiran/dict:entry-info-short` (`dict.lisp:1595`).
//!
//! ```lisp
//! (defun entry-info-short (seq &key with-pos)
//!   (let ((sense-str (short-sense-str seq :with-pos with-pos)))
//!     (with-output-to-string (s)
//!       (format s "~a : " (reading-str-seq seq))
//!       (when sense-str (princ sense-str s)))))
//! ```

use super::reading_str_seq::reading_str_seq;
use super::short_sense_str::short_sense_str;
use crate::conn::kani_context::KaniranContext;

pub async fn entry_info_short(
    ctx: &KaniranContext,
    seq: i32,
    with_pos: Option<&str>,
) -> Result<String, sqlx::Error> {
    let sense_str = short_sense_str(ctx, seq, with_pos).await?;
    // ~a of a nil reading prints "NIL"
    let mut s = format!(
        "{} : ",
        reading_str_seq(ctx, seq).await?.as_deref().unwrap_or("NIL")
    );
    if let Some(sense_str) = sense_str {
        s.push_str(&sense_str);
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    /// REPL fixtures (.103, `ichiran/dict::entry-info-short`), 2026-05-24.
    /// Covers a noun (1582710 日本) and a verb (1358280 食べる) with no
    /// with-pos, with-pos matching the entry's pos, and with-pos that
    /// matches no sense (→ sense-str nil → trailing nothing after " : "),
    /// plus an unknown seq (reading nil → "NIL").
    #[tokio::test]
    async fn entry_info_short_fixtures() {
        let ctx = ctx_from_env().await;
        let cases: &[(i32, Option<&str>, &str)] = &[
            (1582710, None, "日本 【にほん】 : Japan"),
            (1358280, None, "食べる 【たべる】 : to eat"),
            (1358280, Some("v1"), "食べる 【たべる】 : to eat"),
            (1358280, Some("n"), "食べる 【たべる】 : "),
            (1582710, Some("v1"), "日本 【にほん】 : "),
            (999999, None, "NIL : "),
            (999999, Some("v1"), "NIL : "),
        ];
        for (seq, with_pos, expected) in cases {
            assert_eq!(
                entry_info_short(&ctx, *seq, *with_pos).await.unwrap(),
                *expected,
                "seq={seq} with_pos={with_pos:?}"
            );
        }
    }
}
