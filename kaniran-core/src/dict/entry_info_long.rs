//! Port of `ichiran/dict:entry-info-long` (`dict.lisp:1601`).
//!
//! ```lisp
//! (defun entry-info-long (seq)
//!   (format nil "~a~@[ ~a~%~]~a" seq (reading-str-seq seq) (get-senses-str seq)))
//! ```

use std::fmt::Write;

use super::get_senses_str::get_senses_str;
use super::reading_str_seq::reading_str_seq;
use crate::conn::kani_context::KaniranContext;

pub async fn entry_info_long(
    ctx: &KaniranContext,
    seq: i32,
) -> Result<String, sqlx::Error> {
    let mut out = seq.to_string();
    // dict.lisp:1601 (~@[ ~a~%~]) — " <reading>\n" only when reading-str-seq
    // is non-nil; nil prints nothing (unlike entry-info-short's bare ~a).
    if let Some(reading) = reading_str_seq(ctx, seq).await? {
        writeln!(out, " {}", reading).unwrap();
    }
    out.push_str(&get_senses_str(ctx, seq).await?);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    /// REPL fixtures (.103, `ichiran/dict::entry-info-long`), 2026-05-25.
    /// Covers a single-sense noun (reading with 【kanji】), a verb,
    /// a kana-only entry (reading-str* returns bare kana, no 【】),
    /// a multi-sense entry exercising the `《s_inf》` annotation and the
    /// `get-senses-raw` last-tag-group reversal (sense 4 pos), and an
    /// unknown seq (reading nil + empty senses → just the seq, the
    /// `~@[` nil branch).
    #[tokio::test]
    async fn entry_info_long_fixtures() {
        let ctx = ctx_from_env().await;
        let cases: &[(i32, &str)] = &[
            (1562520, "1562520 賄賂 【わいろ】\n1. [n] bribe; sweetener; douceur"),
            (1573390, "1573390 躊躇う 【ためらう】\n1. [vi,v5u] to hesitate; to waver"),
            (1087690, "1087690 ドーナツ\n1. [n] doughnut; donut"),
            (
                1010900,
                "1010900 ぴったり\n1. [adv,adv-to,vs] 《ぴったし is colloquial》 tightly; closely\n2. [adv,adv-to,vs] exactly; precisely\n3. [adv,adv-to,vs] suddenly (stopping)\n4. [adj-na,vs,adv-to,adv] perfectly (suited); ideally",
            ),
            (999999, "999999"),
        ];
        for (seq, expected) in cases {
            assert_eq!(
                &entry_info_long(&ctx, *seq).await.unwrap(),
                expected,
                "seq={seq}"
            );
        }
    }
}
