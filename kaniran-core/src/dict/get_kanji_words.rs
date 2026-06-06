//! Port of `ichiran/dict:get-kanji-words` (`dict.lisp:1834`).
//!
//! Returns `(seq, kanji-text, kana-text, common)` rows for every root
//! entry whose kanji writing contains `char` as a substring, restricted
//! to the best-kana reading with a non-null `common`.

use crate::conn::kani_context::KaniranContext;

pub async fn get_kanji_words(
    ctx: &KaniranContext,
    char: &str,
) -> Result<Vec<(i32, String, String, i32)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT e.seq, k.text, r.text, k.common \
         FROM entry AS e, kanji_text AS k, kana_text AS r \
         WHERE e.seq = k.seq \
           AND e.seq = r.seq \
           AND r.text = k.best_kana \
           AND k.common IS NOT NULL \
           AND e.root_p \
           AND k.text LIKE '%' || $1 || '%'",
    )
    .bind(char)
    .fetch_all(&ctx.pool)
    .await
}

#[cfg(test)]
mod tests {
    //! Every assertion is REPL-verified against the .103 SBCL via
    //! `(ichiran/dict::get-kanji-words …)` (2026-05-25 probe).
    //! Run with `-- --test-threads=1` per the DB-test convention.
    use super::*;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
    }

    fn row(seq: i32, kanji: &str, kana: &str, common: i32) -> (i32, String, String, i32) {
        (seq, kanji.to_string(), kana.to_string(), common)
    }

    /// The query has no ORDER BY, so the result is an unordered set; both
    /// sides are sorted by seq before comparison. `蜂蜜` carries
    /// `common = 0`, exercising the non-null-but-zero branch of the
    /// `(:not-null 'k.common)` filter.
    #[tokio::test]
    async fn get_kanji_words_fixtures() {
        let ctx = ctx().await;
        let cases: &[(&str, Vec<(i32, String, String, i32)>)] = &[
            (
                "鯨",
                vec![
                    row(1253270, "鯨", "くじら", 13),
                    row(1253290, "鯨肉", "げいにく", 30),
                    row(1514180, "捕鯨", "ほげい", 6),
                ],
            ),
            ("錐", vec![row(1175930, "円錐", "まるぎり", 44)]),
            (
                "蜂",
                vec![
                    row(1517840, "蜂", "はち", 34),
                    row(1517860, "蜂蜜", "はちみつ", 0),
                    row(1729030, "蜂起", "ほうき", 39),
                ],
            ),
        ];
        for (char, expected) in cases {
            let mut got = get_kanji_words(&ctx, char).await.unwrap();
            // Result is an unordered set (no ORDER BY); sort the whole
            // tuple so the comparison is deterministic even if a char
            // ever yields two rows sharing a seq.
            got.sort();
            let mut expected = expected.clone();
            expected.sort();
            assert_eq!(got, expected, "char={char:?}");
        }
    }

    /// `#\火` (char) and `"火"` (string) hit the same query in upstream;
    /// the Rust port collapses both to `&str`, so a single-character
    /// argument returns the full substring match set.
    #[tokio::test]
    async fn single_char_argument() {
        let ctx = ctx().await;
        let words = get_kanji_words(&ctx, "火").await.unwrap();
        assert_eq!(words.len(), 75);
    }
}
