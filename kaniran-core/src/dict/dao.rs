//! Port of the dict.lisp DAO layer — entry / kanji-text / kana-text /
//! sense / sense-prop / gloss / restricted-readings / conjugation /
//! conj-prop / conj-source-reading DAOs plus entry-digest /
//! recalc-entry-stats[-all].

use crate::conn::kani_context::KaniranContext;
use crate::dict::text_classes::SimpleText;
use sqlx::postgres::PgRow;
use sqlx::{FromRow, Row};

#[derive(Debug, Clone)]
pub struct Entry {
    pub seq: i32,
    pub content: String,
    pub root_p: bool,
    pub n_kanji: i32,
    pub n_kana: i32,
    pub primary_nokanji: bool,
}

impl<'r> FromRow<'r, PgRow> for Entry {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(Entry {
            seq: row.try_get("seq")?,
            content: row.try_get("content")?,
            root_p: row.try_get("root_p")?,
            n_kanji: row.try_get("n_kanji")?,
            n_kana: row.try_get("n_kana")?,
            primary_nokanji: row.try_get("primary_nokanji")?,
        })
    }
}

impl Entry {
    /// `get-text` override — `dict.lisp:47-49`:
    ///
    /// ```lisp
    /// (defmethod get-text ((obj entry))
    ///   (text (car (select-dao (if (> (n-kanji obj) 0) 'kanji-text 'kana-text)
    ///                          (:and (:= 'seq (seq obj)) (:= 'ord 0))))))
    /// ```
    ///
    /// Returns the `text` of the entry's headword row at `ord = 0` —
    /// from `kanji_text` when the entry has any kanji writings,
    /// otherwise from `kana_text`. Reached only from locally-typed
    /// callsites (`dict.lisp:67` `entry-digest`, `dict-load.lisp:135`);
    /// `entry` is not a variant of
    /// [`crate::dict::kani::KaniWordDispatchEnum`] (see that
    /// module's header) so no polymorphic `get-text` dispatch flows
    /// here.
    ///
    /// Diverges from the upstream lambda list `(obj)` only by:
    /// - taking `&KaniranContext` for the database handle, replacing
    ///   the upstream dynamic `*connection*` per
    ///   [`crate::conn::kani_context`];
    /// - returning [`Option<String>`] rather than a raw string —
    ///   upstream errors on `(text nil)` if the headword row is
    ///   absent (data-integrity violation), which the Rust port
    ///   surfaces as [`None`].
    pub async fn get_text(&self, ctx: &KaniranContext) -> Result<Option<String>, sqlx::Error> {
        let table = if self.n_kanji > 0 {
            "kanji_text"
        } else {
            "kana_text"
        };
        let sql = format!("SELECT text FROM {} WHERE seq = $1 AND ord = 0", table);
        let row: Option<(String,)> = sqlx::query_as(&sql)
            .bind(self.seq)
            .fetch_optional(&ctx.pool)
            .await?;
        Ok(row.map(|(t,)| t))
    }

    /// `get-kana` override — `dict.lisp:44-45`:
    ///
    /// ```lisp
    /// (defmethod get-kana ((obj entry))
    ///   (text (car (select-dao 'kana-text (:and (:= 'seq (seq obj)) (:= 'ord 0))))))
    /// ```
    ///
    /// Returns the `text` of the entry's headword `kana_text` row at
    /// `ord = 0`. Reached only from locally-typed callsites
    /// (`entry-digest` at `dict.lisp:67` is the canonical one);
    /// `entry` is not a variant of
    /// [`crate::dict::kani::KaniWordDispatchEnum`] (see that module's
    /// header) so no polymorphic `get-kana` dispatch flows here.
    ///
    /// Diverges from the upstream lambda list `(obj)` only by:
    /// - taking `&KaniranContext` for the database handle, replacing
    ///   the upstream dynamic `*connection*` per
    ///   [`crate::conn::kani_context`];
    /// - returning [`Option<String>`] rather than a raw string —
    ///   upstream errors on `(text nil)` if the headword row is
    ///   absent (data-integrity violation), which the Rust port
    ///   surfaces as [`None`].
    pub async fn get_kana(&self, ctx: &KaniranContext) -> Result<Option<String>, sqlx::Error> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT text FROM kana_text WHERE seq = $1 AND ord = 0")
                .bind(self.seq)
                .fetch_optional(&ctx.pool)
                .await?;
        Ok(row.map(|(t,)| t))
    }
}

#[derive(Debug, Clone)]
pub struct KanjiText {
    pub id: i32,
    pub seq: i32,
    pub text: String,
    pub ord: i32,
    pub common: Option<i32>,
    pub common_tags: String,
    pub conjugate_p: bool,
    pub nokanji: bool,
    pub best_kana: Option<String>,
    pub state: SimpleText,
}

impl<'r> FromRow<'r, PgRow> for KanjiText {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(KanjiText {
            id: row.try_get("id")?,
            seq: row.try_get("seq")?,
            text: row.try_get("text")?,
            ord: row.try_get("ord")?,
            common: row.try_get("common")?,
            common_tags: row.try_get("common_tags")?,
            conjugate_p: row.try_get("conjugate_p")?,
            nokanji: row.try_get("nokanji")?,
            best_kana: row.try_get("best_kana")?,
            state: SimpleText::default(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct KanaText {
    pub id: i32,
    pub seq: i32,
    pub text: String,
    pub ord: i32,
    pub common: Option<i32>,
    pub common_tags: String,
    pub conjugate_p: bool,
    pub nokanji: bool,
    pub best_kanji: Option<String>,
    pub state: SimpleText,
}

impl<'r> FromRow<'r, PgRow> for KanaText {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(KanaText {
            id: row.try_get("id")?,
            seq: row.try_get("seq")?,
            text: row.try_get("text")?,
            ord: row.try_get("ord")?,
            common: row.try_get("common")?,
            common_tags: row.try_get("common_tags")?,
            conjugate_p: row.try_get("conjugate_p")?,
            nokanji: row.try_get("nokanji")?,
            best_kanji: row.try_get("best_kanji")?,
            state: SimpleText::default(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct Sense {
    pub id: i32,
    pub seq: i32,
    pub ord: i32,
}

impl<'r> FromRow<'r, PgRow> for Sense {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(Sense {
            id: row.try_get("id")?,
            seq: row.try_get("seq")?,
            ord: row.try_get("ord")?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct SenseProp {
    pub id: i32,
    pub tag: String,
    pub sense_id: i32,
    pub text: String,
    pub ord: i32,
    pub seq: i32,
}

impl<'r> FromRow<'r, PgRow> for SenseProp {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(SenseProp {
            id: row.try_get("id")?,
            tag: row.try_get("tag")?,
            sense_id: row.try_get("sense_id")?,
            text: row.try_get("text")?,
            ord: row.try_get("ord")?,
            seq: row.try_get("seq")?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Gloss {
    pub id: i32,
    pub sense_id: i32,
    pub text: String,
    pub ord: i32,
}

impl<'r> FromRow<'r, PgRow> for Gloss {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(Gloss {
            id: row.try_get("id")?,
            sense_id: row.try_get("sense_id")?,
            text: row.try_get("text")?,
            ord: row.try_get("ord")?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct RestrictedReadings {
    pub id: i32,
    pub seq: i32,
    pub reading: String,
    pub text: String,
}

impl<'r> FromRow<'r, PgRow> for RestrictedReadings {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(RestrictedReadings {
            id: row.try_get("id")?,
            seq: row.try_get("seq")?,
            reading: row.try_get("reading")?,
            text: row.try_get("text")?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Conjugation {
    pub id: i32,
    pub seq: i32,
    pub seq_from: i32,
    pub seq_via: Option<i32>,
}

impl<'r> FromRow<'r, PgRow> for Conjugation {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(Conjugation {
            id: row.try_get("id")?,
            seq: row.try_get("seq")?,
            seq_from: row.try_get("from")?,
            seq_via: row.try_get("via")?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ConjProp {
    pub id: i32,
    pub conj_id: i32,
    pub conj_type: i32,
    pub pos: String,
    pub neg: Option<bool>,
    pub fml: Option<bool>,
}

impl<'r> FromRow<'r, PgRow> for ConjProp {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(ConjProp {
            id: row.try_get("id")?,
            conj_id: row.try_get("conj_id")?,
            conj_type: row.try_get("conj_type")?,
            pos: row.try_get("pos")?,
            neg: row.try_get("neg")?,
            fml: row.try_get("fml")?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ConjSourceReading {
    pub id: i32,
    pub conj_id: i32,
    pub text: String,
    pub source_text: String,
}

impl<'r> FromRow<'r, PgRow> for ConjSourceReading {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(ConjSourceReading {
            id: row.try_get("id")?,
            conj_id: row.try_get("conj_id")?,
            text: row.try_get("text")?,
            source_text: row.try_get("source_text")?,
        })
    }
}

pub async fn entry_digest(
    ctx: &KaniranContext,
    entry: &Entry,
) -> Result<(i32, Option<String>, Option<String>), sqlx::Error> {
    Ok((
        entry.seq,
        entry.get_text(ctx).await?,
        entry.get_kana(ctx).await?,
    ))
}

pub async fn recalc_entry_stats(ctx: &KaniranContext, entries: &[i32]) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE entry SET \
         n_kanji = (SELECT COUNT(id) FROM kanji_text WHERE kanji_text.seq = entry.seq), \
         n_kana = (SELECT COUNT(id) FROM kana_text WHERE kana_text.seq = entry.seq) \
         WHERE entry.seq = ANY($1)",
    )
    .bind(entries)
    .execute(&ctx.pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn recalc_entry_stats_all(ctx: &KaniranContext) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE entry SET \
         n_kanji = (SELECT COUNT(id) FROM kanji_text WHERE kanji_text.seq = entry.seq), \
         n_kana = (SELECT COUNT(id) FROM kana_text WHERE kana_text.seq = entry.seq)",
    )
    .execute(&ctx.pool)
    .await?;
    Ok(result.rows_affected())
}

#[cfg(test)]
mod test_entry_digest {
    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    async fn load_entry(ctx: &KaniranContext, seq: i32) -> Entry {
        sqlx::query_as::<_, Entry>("SELECT * FROM entry WHERE seq = $1")
            .bind(seq)
            .fetch_one(&ctx.pool)
            .await
            .unwrap()
    }

    /// REPL fixtures (.103, `ichiran/dict::entry-digest`), 2026-05-25.
    /// Covers `n-kanji > 0` entries (get-text reads kanji-text: a noun,
    /// a 2-kanji noun, a verb) and `n-kanji = 0` entries (get-text reads
    /// kana-text, so text equals kana: a katakana loanword and an
    /// onomatopoeic adverb).
    #[tokio::test]
    async fn entry_digest_fixtures() {
        let ctx = ctx_from_env().await;
        let cases: &[(i32, &str, &str)] = &[
            (1257590, "憲法", "けんぽう"),
            (1386690, "雪崩", "なだれ"),
            (1573390, "躊躇う", "ためらう"),
            (1087690, "ドーナツ", "ドーナツ"),
            (1010900, "ぴったり", "ぴったり"),
        ];
        for (seq, text, kana) in cases {
            let entry = load_entry(&ctx, *seq).await;
            let digest = entry_digest(&ctx, &entry).await.unwrap();
            assert_eq!(
                (digest.0, digest.1.as_deref(), digest.2.as_deref()),
                (*seq, Some(*text), Some(*kana)),
                "seq={seq}"
            );
        }
    }
}

#[cfg(test)]
mod test_recalc_entry_stats {
    use super::*;

    // Idempotent on a consistent dictionary: the UPDATE rewrites each
    // matched entry's n_kanji / n_kana to the same values it already
    // holds. Affected count is the number of matched entry rows, not
    // changed rows. All cases REPL-pinned against ichiran 2026-05-25.
    #[tokio::test]
    async fn affected_count_matches_matched_rows() {
        let ctx = KaniranContext::from_env().await.expect("ctx");

        // (recalc-entry-stats 1591050) -> 1 row affected.
        let one = recalc_entry_stats(&ctx, &[1591050]).await.expect("one");
        assert_eq!(one, 1);

        // (recalc-entry-stats 1591050 1495740 1221520) -> 3.
        let multi = recalc_entry_stats(&ctx, &[1591050, 1495740, 1221520])
            .await
            .expect("multi");
        assert_eq!(multi, 3);

        // (recalc-entry-stats) -> 0 (empty set, seq IN (NULL)).
        let empty = recalc_entry_stats(&ctx, &[]).await.expect("empty");
        assert_eq!(empty, 0);

        // A seq with no matching entry row -> 0 affected.
        let missing = recalc_entry_stats(&ctx, &[99999999])
            .await
            .expect("missing");
        assert_eq!(missing, 0);

        // (recalc-entry-stats 1591050 99999999) -> 1 (only the present
        // seq is matched; the absent one contributes nothing).
        let mixed = recalc_entry_stats(&ctx, &[1591050, 99999999])
            .await
            .expect("mixed");
        assert_eq!(mixed, 1);
    }

    #[tokio::test]
    async fn stats_match_child_counts_after_recalc() {
        let ctx = KaniranContext::from_env().await.expect("ctx");

        // Varied vocabulary spanning the kanji/kana count combinations.
        // seq -> (n_kanji, n_kana). REPL-pinned 2026-05-25.
        let cases: &[(i32, i32, i32)] = &[
            (1603990, 2, 1), // 仄か
            (1000580, 2, 2), // 彼
            (1582710, 1, 2),
            (2028930, 0, 1), // が
            (1467640, 1, 2),
        ];
        let seqs: Vec<i32> = cases.iter().map(|(seq, _, _)| *seq).collect();

        let affected = recalc_entry_stats(&ctx, &seqs).await.expect("recalc");
        assert_eq!(affected, seqs.len() as u64);

        for (seq, exp_kanji, exp_kana) in cases {
            let (n_kanji, n_kana): (i32, i32) =
                sqlx::query_as("SELECT n_kanji, n_kana FROM entry WHERE seq = $1")
                    .bind(seq)
                    .fetch_one(&ctx.pool)
                    .await
                    .expect("entry row");
            assert_eq!(n_kanji, *exp_kanji, "seq={seq} n_kanji");
            assert_eq!(n_kana, *exp_kana, "seq={seq} n_kana");

            let actual_kanji: i64 =
                sqlx::query_scalar("SELECT COUNT(id) FROM kanji_text WHERE seq = $1")
                    .bind(seq)
                    .fetch_one(&ctx.pool)
                    .await
                    .expect("kanji count");
            let actual_kana: i64 =
                sqlx::query_scalar("SELECT COUNT(id) FROM kana_text WHERE seq = $1")
                    .bind(seq)
                    .fetch_one(&ctx.pool)
                    .await
                    .expect("kana count");
            assert_eq!(
                n_kanji as i64, actual_kanji,
                "seq={seq} stored vs actual kanji"
            );
            assert_eq!(
                n_kana as i64, actual_kana,
                "seq={seq} stored vs actual kana"
            );
        }
    }
}

#[cfg(test)]
mod test_recalc_entry_stats_all {
    use super::*;

    // Idempotent on a consistent dictionary: every entry's stored
    // n_kanji / n_kana already equals its child-row counts, so the
    // UPDATE rewrites them to the same values. Affected count is the
    // total entry row count regardless (Postgres counts matched rows,
    // not changed rows). REPL-pinned against ichiran 2026-05-25.
    #[tokio::test]
    async fn affects_all_entries_and_stats_match_children() {
        let ctx = KaniranContext::from_env().await.expect("ctx");

        let affected = recalc_entry_stats_all(&ctx).await.expect("recalc-all");

        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM entry")
            .fetch_one(&ctx.pool)
            .await
            .expect("count entries");
        assert_eq!(affected, total as u64, "affected != total entry rows");

        // Spot-check varied vocabulary post-recalc: stored stats equal
        // the independently-counted child rows. (REPL 2026-05-25.)
        // seq -> (n_kanji, n_kana)
        let cases: &[(i32, i32, i32)] = &[
            (1603990, 2, 1), // 仄か
            (1000580, 2, 2), // 彼
            (1582710, 1, 2),
            (1591050, 2, 1), // 気が付く
            (2028930, 0, 1), // が
            (1467640, 1, 2),
        ];
        for (seq, exp_kanji, exp_kana) in cases {
            let (n_kanji, n_kana): (i32, i32) =
                sqlx::query_as("SELECT n_kanji, n_kana FROM entry WHERE seq = $1")
                    .bind(seq)
                    .fetch_one(&ctx.pool)
                    .await
                    .expect("entry row");
            assert_eq!(n_kanji, *exp_kanji, "seq={seq} n_kanji");
            assert_eq!(n_kana, *exp_kana, "seq={seq} n_kana");

            let actual_kanji: i64 =
                sqlx::query_scalar("SELECT COUNT(id) FROM kanji_text WHERE seq = $1")
                    .bind(seq)
                    .fetch_one(&ctx.pool)
                    .await
                    .expect("kanji count");
            let actual_kana: i64 =
                sqlx::query_scalar("SELECT COUNT(id) FROM kana_text WHERE seq = $1")
                    .bind(seq)
                    .fetch_one(&ctx.pool)
                    .await
                    .expect("kana count");
            assert_eq!(
                n_kanji as i64, actual_kanji,
                "seq={seq} stored vs actual kanji"
            );
            assert_eq!(
                n_kana as i64, actual_kana,
                "seq={seq} stored vs actual kana"
            );
        }
    }
}
