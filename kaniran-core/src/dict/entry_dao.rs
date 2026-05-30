//! Port of `ichiran/dict:entry` (`dict.lisp:26`).
//!
//! Row representation of one JMdict entry, mapped 1:1 to the
//! `public.entry` Postgres table populated by ichiran's schema.
//! `seq` is the JMdict sequence number (primary key). `n_kanji` and
//! `n_kana` cache the row counts of the entry's `kanji_text` and
//! `kana_text` children — the upstream `recalc-entry-stats[-all]`
//! refreshes them after a corpus reload. `root_p` flags entries
//! retained as themselves through derivation pruning;
//! `primary_nokanji` flags kana-only headwords with no kanji form.
//!
//! The methods upstream defines on this class (`get-kana`,
//! `get-text`, `get-kanji`, `print-object`, `common`) belong to
//! generic functions that have their own symbol entries in the port
//! plan and land in their own files when those generics are ported.

use crate::conn::kani_context::KaniranContext;
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
    pub async fn get_text(
        &self,
        ctx: &KaniranContext,
    ) -> Result<Option<String>, sqlx::Error> {
        let table = if self.n_kanji > 0 { "kanji_text" } else { "kana_text" };
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
    /// [`super::kani::KaniWordDispatchEnum`] (see that module's
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
    pub async fn get_kana(
        &self,
        ctx: &KaniranContext,
    ) -> Result<Option<String>, sqlx::Error> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT text FROM kana_text WHERE seq = $1 AND ord = 0")
                .bind(self.seq)
                .fetch_optional(&ctx.pool)
                .await?;
        Ok(row.map(|(t,)| t))
    }
}
