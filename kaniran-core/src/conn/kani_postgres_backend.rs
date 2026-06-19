//! Rust-only sidecar: the Postgres implementation of
//! [`KaniBackend`](crate::conn::kani_backend::KaniBackend). Every
//! runtime (lookup-serving) SQL query in the crate lives here as one
//! method per distinct SQL statement; callers go through `ctx.store`
//! and never touch SQL. Build-time code (`dict/load`, `dict/errata`,
//! the kanjidic loaders) writes the database and keeps using
//! `ctx.pool` directly.
//!
//! Method bodies are byte-identical moves of the SQL they replaced;
//! the contract each method serves (and the upstream function it came
//! from) is documented on the trait.

use crate::conn::kani_backend::KaniBackend;
use crate::dict::dao::{ConjProp, Conjugation, Entry, KanaText, KanjiText};
use crate::kanji::dao::{Kanji, Meaning, Reading};
use sqlx::PgPool;
use std::borrow::Cow;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Postgres-backed dictionary store. The [`KaniBackend`] contract is
/// synchronous, so each query blocks on a shared multi-thread tokio
/// runtime — the same runtime the pool was created on. Cheap to clone:
/// both the pool and the runtime handle are reference-counted.
#[derive(Clone)]
pub struct KaniPostgresBackend {
    pool: PgPool,
    rt: Arc<Runtime>,
}

impl KaniPostgresBackend {
    pub fn new(pool: PgPool, rt: Arc<Runtime>) -> Self {
        Self { pool, rt }
    }

    /// Drive a future on the pool's runtime. The build-time data loaders
    /// in `kaniran-loader` are async and must run their `sqlx` queries
    /// against `pool` on the runtime it was created on; this hands them
    /// that runtime — the same one the synchronous lookup methods below
    /// already block on.
    pub fn block_on<F: std::future::Future>(&self, fut: F) -> F::Output {
        self.rt.block_on(fut)
    }
}

impl KaniBackend for KaniPostgresBackend {
    // --- entry ---

    fn entry_by_seq(&self, seq: i32) -> Result<Option<Entry>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as("SELECT * FROM entry WHERE seq = $1")
                    .bind(seq)
                    .fetch_optional(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn root_seqs(&self, seqs: &[i32]) -> Result<Vec<i32>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_scalar("SELECT seq FROM entry WHERE seq = ANY($1) AND root_p")
                    .bind(seqs)
                    .fetch_all(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn candidate_seqs_kana(&self, text: &str) -> Result<Vec<i32>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                let rows: Vec<(i32,)> = sqlx::query_as(
                    "SELECT e.seq FROM entry e \
                     LEFT JOIN kana_text r ON e.seq = r.seq \
                     LEFT JOIN kanji_text k ON e.seq = k.seq \
                     WHERE e.root_p AND k.text IS NULL AND r.text = $1 AND r.ord = 0 \
                     ORDER BY e.seq",
                )
                .bind(text)
                .fetch_all(&self.pool)
                .await?;
                Ok::<Vec<i32>, sqlx::Error>(rows.into_iter().map(|(seq,)| seq).collect())
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn candidate_seqs_kanji(
        &self,
        text: &str,
        reading: Option<&str>,
    ) -> Result<Vec<i32>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                let rows: Vec<(i32,)> = sqlx::query_as(
                    "SELECT e.seq FROM entry e \
                     LEFT JOIN kana_text r ON e.seq = r.seq \
                     LEFT JOIN kanji_text k ON e.seq = k.seq \
                     WHERE k.text = $1 AND k.ord = 0 AND r.text = $2 AND r.ord = 0 \
                     ORDER BY e.seq",
                )
                .bind(text)
                .bind(reading)
                .fetch_all(&self.pool)
                .await?;
                Ok::<Vec<i32>, sqlx::Error>(rows.into_iter().map(|(seq,)| seq).collect())
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn kanji_words_containing_char(
        &self,
        char: &str,
    ) -> Result<Vec<(i32, Cow<'static, str>, Cow<'static, str>, i32)>, crate::conn::KaniDbError> {
        let rows: Vec<(i32, String, String, i32)> = self
            .rt
            .block_on(async {
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
                .fetch_all(&self.pool)
                .await
            })
            .map_err(crate::conn::KaniDbError::from)?;
        Ok(rows
            .into_iter()
            .map(|(seq, kanji, kana, common)| {
                (seq, Cow::Owned(kanji), Cow::Owned(kana), common)
            })
            .collect())
    }

    // --- kanji_text / kana_text ---

    fn headword_kanji_text(&self, seq: i32) -> Result<Option<Cow<'static, str>>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_scalar("SELECT text FROM kanji_text WHERE seq = $1 AND ord = 0")
                    .bind(seq)
                    .fetch_optional(&self.pool)
                    .await
            })
            .map(|opt: Option<String>| opt.map(Cow::Owned))
            .map_err(crate::conn::KaniDbError::from)
    }

    fn headword_kana_text(&self, seq: i32) -> Result<Option<Cow<'static, str>>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_scalar("SELECT text FROM kana_text WHERE seq = $1 AND ord = 0")
                    .bind(seq)
                    .fetch_optional(&self.pool)
                    .await
            })
            .map(|opt: Option<String>| opt.map(Cow::Owned))
            .map_err(crate::conn::KaniDbError::from)
    }

    fn kanji_texts_by_seq_and_text(
        &self,
        seq: i32,
        text: &str,
    ) -> Result<Vec<KanjiText>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as("SELECT * FROM kanji_text WHERE seq = $1 AND text = $2")
                    .bind(seq)
                    .bind(text)
                    .fetch_all(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn kana_texts_by_seq_and_text(
        &self,
        seq: i32,
        text: &str,
    ) -> Result<Vec<KanaText>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as("SELECT * FROM kana_text WHERE seq = $1 AND text = $2")
                    .bind(seq)
                    .bind(text)
                    .fetch_all(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn kana_texts_by_seq_and_text_any(
        &self,
        seq: i32,
        texts: &[String],
    ) -> Result<Vec<KanaText>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as("SELECT * FROM kana_text WHERE seq = $1 AND text = ANY($2)")
                    .bind(seq)
                    .bind(texts)
                    .fetch_all(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn kanji_texts_by_seq_and_text_any(
        &self,
        seq: i32,
        texts: &[String],
    ) -> Result<Vec<KanjiText>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as("SELECT * FROM kanji_text WHERE seq = $1 AND text = ANY($2)")
                    .bind(seq)
                    .bind(texts)
                    .fetch_all(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn kana_texts_by_seq_ordered(&self, seq: i32) -> Result<Vec<KanaText>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as("SELECT * FROM kana_text WHERE seq = $1 ORDER BY ord")
                    .bind(seq)
                    .fetch_all(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn kanji_text_by_id(&self, id: i32) -> Result<KanjiText, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as("SELECT * FROM kanji_text WHERE id = $1")
                    .bind(id)
                    .fetch_one(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn kana_text_by_id(&self, id: i32) -> Result<KanaText, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as("SELECT * FROM kana_text WHERE id = $1")
                    .bind(id)
                    .fetch_one(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn kana_texts_by_text(&self, text: &str) -> Result<Vec<KanaText>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as("SELECT * FROM kana_text WHERE text = $1")
                    .bind(text)
                    .fetch_all(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn kanji_texts_by_text(&self, text: &str) -> Result<Vec<KanjiText>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as("SELECT * FROM kanji_text WHERE text = $1")
                    .bind(text)
                    .fetch_all(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn kana_texts_root_by_text(&self, text: &str) -> Result<Vec<KanaText>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as(
                    "SELECT wt.* FROM kana_text wt \
                     INNER JOIN entry ON wt.seq = entry.seq \
                     WHERE wt.text = $1 AND entry.root_p",
                )
                .bind(text)
                .fetch_all(&self.pool)
                .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn kanji_texts_root_by_text(
        &self,
        text: &str,
    ) -> Result<Vec<KanjiText>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as(
                    "SELECT wt.* FROM kanji_text wt \
                     INNER JOIN entry ON wt.seq = entry.seq \
                     WHERE wt.text = $1 AND entry.root_p",
                )
                .bind(text)
                .fetch_all(&self.pool)
                .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn kana_texts_by_text_any(
        &self,
        texts: &[String],
    ) -> Result<Vec<KanaText>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as("SELECT * FROM kana_text WHERE text = ANY($1)")
                    .bind(texts)
                    .fetch_all(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn kanji_texts_by_text_any(
        &self,
        texts: &[String],
    ) -> Result<Vec<KanjiText>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as("SELECT * FROM kanji_text WHERE text = ANY($1)")
                    .bind(texts)
                    .fetch_all(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn kanji_texts_by_text_any_and_seq_any(
        &self,
        texts: &[&str],
        seqs: &[i32],
    ) -> Result<Vec<KanjiText>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as("SELECT * FROM kanji_text WHERE text = ANY($1) AND seq = ANY($2)")
                    .bind(texts)
                    .bind(seqs)
                    .fetch_all(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn kana_texts_by_text_any_and_seq_any(
        &self,
        texts: &[&str],
        seqs: &[i32],
    ) -> Result<Vec<KanaText>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as("SELECT * FROM kana_text WHERE text = ANY($1) AND seq = ANY($2)")
                    .bind(texts)
                    .bind(seqs)
                    .fetch_all(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn kana_texts_by_text_and_seq_any(
        &self,
        text: &str,
        seqs: &[i32],
    ) -> Result<Vec<KanaText>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as("SELECT * FROM kana_text WHERE text = $1 AND seq = ANY($2)")
                    .bind(text)
                    .bind(seqs)
                    .fetch_all(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn kanji_texts_by_text_and_seq_any(
        &self,
        text: &str,
        seqs: &[i32],
    ) -> Result<Vec<KanjiText>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as("SELECT * FROM kanji_text WHERE text = $1 AND seq = ANY($2)")
                    .bind(text)
                    .bind(seqs)
                    .fetch_all(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn kana_texts_conj_of(
        &self,
        seqs: &[i32],
        text: &str,
    ) -> Result<Vec<KanaText>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as(
                    "SELECT kt.* FROM kana_text kt, conjugation conj \
                     WHERE kt.seq = conj.seq AND conj.\"from\" = ANY($1) AND kt.text = $2",
                )
                .bind(seqs)
                .bind(text)
                .fetch_all(&self.pool)
                .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn kanji_texts_conj_of(
        &self,
        seqs: &[i32],
        text: &str,
    ) -> Result<Vec<KanjiText>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as(
                    "SELECT kt.* FROM kanji_text kt, conjugation conj \
                     WHERE kt.seq = conj.seq AND conj.\"from\" = ANY($1) AND kt.text = $2",
                )
                .bind(seqs)
                .bind(text)
                .fetch_all(&self.pool)
                .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn kana_texts_with_pos(
        &self,
        text: &str,
        posi: &[String],
    ) -> Result<Vec<KanaText>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as(
                    "SELECT DISTINCT kt.* FROM kana_text kt \
                     INNER JOIN sense_prop sp ON sp.seq = kt.seq AND sp.tag = 'pos' \
                     WHERE kt.text = $1 AND sp.text = ANY($2)",
                )
                .bind(text)
                .bind(posi)
                .fetch_all(&self.pool)
                .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn kanji_texts_with_pos(
        &self,
        text: &str,
        posi: &[String],
    ) -> Result<Vec<KanjiText>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as(
                    "SELECT DISTINCT kt.* FROM kanji_text kt \
                     INNER JOIN sense_prop sp ON sp.seq = kt.seq AND sp.tag = 'pos' \
                     WHERE kt.text = $1 AND sp.text = ANY($2)",
                )
                .bind(text)
                .bind(posi)
                .fetch_all(&self.pool)
                .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn kana_forms_rows(&self, seq: i32) -> Result<Vec<KanaText>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as(
                    "SELECT kt.* FROM kana_text kt WHERE kt.seq = $1 \
                     UNION \
                     SELECT kt.* FROM kana_text kt \
                     LEFT JOIN conjugation conj ON conj.seq = kt.seq \
                     WHERE conj.\"from\" = $1",
                )
                .bind(seq)
                .fetch_all(&self.pool)
                .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn kana_texts_by_text_and_seq(
        &self,
        text: &str,
        seq: i32,
    ) -> Result<Vec<KanaText>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as("SELECT * FROM kana_text WHERE text = $1 AND seq = $2")
                    .bind(text)
                    .bind(seq)
                    .fetch_all(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn kana_seqs_by_text(&self, text: &str) -> Result<Vec<i32>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_scalar("SELECT seq FROM kana_text WHERE text = $1")
                    .bind(text)
                    .fetch_all(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn kanji_seqs_by_text(&self, text: &str) -> Result<Vec<i32>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_scalar("SELECT seq FROM kanji_text WHERE text = $1")
                    .bind(text)
                    .fetch_all(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn kana_reading_texts_by_seq_any(
        &self,
        seqs: &[i32],
    ) -> Result<Vec<Cow<'static, str>>, crate::conn::KaniDbError> {
        let rows: Vec<String> = self
            .rt
            .block_on(async {
                sqlx::query_scalar("SELECT text FROM kana_text WHERE seq = ANY($1) ORDER BY id")
                    .bind(seqs)
                    .fetch_all(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)?;
        Ok(rows.into_iter().map(Cow::Owned).collect())
    }

    fn kana_seqs_by_seq_and_text(
        &self,
        seq: i32,
        text: &str,
    ) -> Result<Vec<i32>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_scalar("SELECT seq FROM kana_text WHERE seq = $1 AND text = $2")
                    .bind(seq)
                    .bind(text)
                    .fetch_all(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn kana_texts_by_regex(&self, pattern: &str) -> Result<Vec<KanaText>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as("SELECT * FROM kana_text WHERE text ~ $1")
                    .bind(pattern)
                    .fetch_all(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn kana_texts_by_seq_any(&self, seqs: &[i32]) -> Result<Vec<KanaText>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as("SELECT * FROM kana_text WHERE seq = ANY($1)")
                    .bind(seqs)
                    .fetch_all(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn kanji_texts_by_seq_any(
        &self,
        seqs: &[i32],
    ) -> Result<Vec<KanjiText>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as("SELECT * FROM kanji_text WHERE seq = ANY($1)")
                    .bind(seqs)
                    .fetch_all(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    // --- conjugation ---

    fn conjs_by_seq(&self, seq: i32) -> Result<Vec<Conjugation>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as("SELECT * FROM conjugation WHERE seq = $1")
                    .bind(seq)
                    .fetch_all(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn conjs_by_seq_and_ids(
        &self,
        seq: i32,
        ids: &[i32],
    ) -> Result<Vec<Conjugation>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as("SELECT * FROM conjugation WHERE seq = $1 AND id = ANY($2)")
                    .bind(seq)
                    .bind(ids)
                    .fetch_all(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn conjs_by_seq_and_from(
        &self,
        seq: i32,
        from: i32,
    ) -> Result<Vec<Conjugation>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as("SELECT * FROM conjugation WHERE seq = $1 AND \"from\" = $2")
                    .bind(seq)
                    .bind(from)
                    .fetch_all(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn conjs_by_seq_via_null(&self, seq: i32) -> Result<Vec<Conjugation>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as("SELECT * FROM conjugation WHERE seq = $1 AND via IS NULL")
                    .bind(seq)
                    .fetch_all(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn conj_by_id(&self, id: i32) -> Result<Conjugation, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as("SELECT * FROM conjugation WHERE id = $1")
                    .bind(id)
                    .fetch_one(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn conj_seqs_of_desu(&self, seqs: &[i32]) -> Result<Vec<i32>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_scalar(
                    "SELECT seq FROM conjugation \
                     WHERE seq = ANY($1) AND \"from\" = 2755350",
                )
                .bind(seqs)
                .fetch_all(&self.pool)
                .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn conj_seqs_from_any(&self, seqs: &[i32]) -> Result<Vec<i32>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_scalar("SELECT DISTINCT seq FROM conjugation WHERE \"from\" = ANY($1)")
                    .bind(seqs)
                    .fetch_all(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn no_conj_seqs(&self) -> Result<Vec<i32>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_scalar(
                    "SELECT entry.seq FROM entry \
                     LEFT JOIN conjugation c ON entry.seq = c.seq \
                     WHERE c.seq IS NULL",
                )
                .fetch_all(&self.pool)
                .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    // --- conj_prop / conj_source_reading ---

    fn conj_props_by_conj_id(
        &self,
        conj_id: i32,
    ) -> Result<Vec<ConjProp>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as("SELECT * FROM conj_prop WHERE conj_id = $1")
                    .bind(conj_id)
                    .fetch_all(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn conj_source_readings_by_conj_id(
        &self,
        conj_id: i32,
    ) -> Result<Vec<(Cow<'static, str>, Cow<'static, str>)>, crate::conn::KaniDbError> {
        let rows: Vec<(String, String)> = self
            .rt
            .block_on(async {
                sqlx::query_as("SELECT text, source_text FROM conj_source_reading WHERE conj_id = $1")
                    .bind(conj_id)
                    .fetch_all(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)?;
        Ok(rows
            .into_iter()
            .map(|(text, source)| (Cow::Owned(text), Cow::Owned(source)))
            .collect())
    }

    fn conj_source_readings_by_conj_id_and_texts(
        &self,
        conj_id: i32,
        texts: &[String],
    ) -> Result<Vec<(Cow<'static, str>, Cow<'static, str>)>, crate::conn::KaniDbError> {
        let rows: Vec<(String, String)> = self
            .rt
            .block_on(async {
                sqlx::query_as(
                    "SELECT text, source_text FROM conj_source_reading \
                     WHERE conj_id = $1 AND text = ANY($2)",
                )
                .bind(conj_id)
                .bind(texts)
                .fetch_all(&self.pool)
                .await
            })
            .map_err(crate::conn::KaniDbError::from)?;
        Ok(rows
            .into_iter()
            .map(|(text, source)| (Cow::Owned(text), Cow::Owned(source)))
            .collect())
    }

    fn conj_source_reading_texts(
        &self,
        conj_id: i32,
        source_text: &str,
    ) -> Result<Vec<Cow<'static, str>>, crate::conn::KaniDbError> {
        let rows: Vec<String> = self
            .rt
            .block_on(async {
                sqlx::query_scalar(
                    "SELECT text FROM conj_source_reading \
                     WHERE conj_id = $1 AND source_text = $2",
                )
                .bind(conj_id)
                .bind(source_text)
                .fetch_all(&self.pool)
                .await
            })
            .map_err(crate::conn::KaniDbError::from)?;
        Ok(rows.into_iter().map(Cow::Owned).collect())
    }

    fn parents_kanji(
        &self,
        seq: i32,
        text: &str,
    ) -> Result<Vec<(i32, i32)>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as(
                    "SELECT kt.id, conj.id \
                     FROM kanji_text kt, conj_source_reading csr, conjugation conj \
                     WHERE conj.seq = $1 \
                       AND conj.id = csr.conj_id \
                       AND csr.text = $2 \
                       AND kt.seq = CASE WHEN conj.via IS NOT NULL THEN conj.via ELSE conj.from END \
                       AND kt.text = csr.source_text",
                )
                .bind(seq)
                .bind(text)
                .fetch_all(&self.pool)
                .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn parents_kana(
        &self,
        seq: i32,
        text: &str,
    ) -> Result<Vec<(i32, i32)>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as(
                    "SELECT kt.id, conj.id \
                     FROM kana_text kt, conj_source_reading csr, conjugation conj \
                     WHERE conj.seq = $1 \
                       AND conj.id = csr.conj_id \
                       AND csr.text = $2 \
                       AND kt.seq = CASE WHEN conj.via IS NOT NULL THEN conj.via ELSE conj.from END \
                       AND kt.text = csr.source_text",
                )
                .bind(seq)
                .bind(text)
                .fetch_all(&self.pool)
                .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    // --- sense / gloss / sense_prop / restricted_readings ---

    fn sense_gloss_rows(
        &self,
        seq: i32,
    ) -> Result<Vec<(i32, Option<String>)>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as(
                    "SELECT sense.ord AS ord, \
                            string_agg(gloss.text, '; ' ORDER BY gloss.ord) AS gloss \
                     FROM sense LEFT JOIN gloss ON gloss.sense_id = sense.id \
                     WHERE sense.seq = $1 \
                     GROUP BY sense.id \
                     ORDER BY sense.ord",
                )
                .bind(seq)
                .fetch_all(&self.pool)
                .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn sense_prop_rows_tagged(
        &self,
        seq: i32,
        tags: &[&str],
    ) -> Result<Vec<(i32, Cow<'static, str>, Cow<'static, str>)>, crate::conn::KaniDbError> {
        let rows: Vec<(i32, String, String)> = self
            .rt
            .block_on(async {
                sqlx::query_as(
                    "SELECT sense.ord AS ord, sense_prop.tag AS tag, sense_prop.text AS text \
                     FROM sense, sense_prop \
                     WHERE sense.seq = $1 \
                       AND sense_prop.sense_id = sense.id \
                       AND sense_prop.tag = ANY($2) \
                     ORDER BY sense.ord, sense_prop.tag, sense_prop.ord",
                )
                .bind(seq)
                .bind(tags)
                .fetch_all(&self.pool)
                .await
            })
            .map_err(crate::conn::KaniDbError::from)?;
        Ok(rows
            .into_iter()
            .map(|(ord, tag, text)| (ord, Cow::Owned(tag), Cow::Owned(text)))
            .collect())
    }

    fn first_sense_gloss(
        &self,
        seq: i32,
    ) -> Result<Option<Option<String>>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
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
                .fetch_optional(&self.pool)
                .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn first_sense_gloss_with_pos(
        &self,
        seq: i32,
        pos: &str,
    ) -> Result<Option<Option<String>>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
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
                .bind(pos)
                .fetch_optional(&self.pool)
                .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn glosses_by_seq_any(
        &self,
        seqs: &[i32],
    ) -> Result<Vec<(i32, Cow<'static, str>)>, crate::conn::KaniDbError> {
        let rows: Vec<(i32, String)> = self
            .rt
            .block_on(async {
                sqlx::query_as(
                    "SELECT sense.seq, gloss.text FROM gloss, sense \
                     WHERE sense.seq = ANY($1) AND gloss.sense_id = sense.id \
                     ORDER BY sense.seq",
                )
                .bind(seqs)
                .fetch_all(&self.pool)
                .await
            })
            .map_err(crate::conn::KaniDbError::from)?;
        Ok(rows
            .into_iter()
            .map(|(seq, text)| (seq, Cow::Owned(text)))
            .collect())
    }

    fn uk_sense_ids(&self, seqs: &[i32]) -> Result<Vec<i32>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_scalar(
                    "SELECT sense_id FROM sense_prop \
                     WHERE seq = ANY($1) AND tag = 'misc' AND text = 'uk'",
                )
                .bind(seqs)
                .fetch_all(&self.pool)
                .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn sense_id_ord0(&self, ids: &[i32]) -> Result<Option<i32>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_scalar("SELECT id FROM sense WHERE id = ANY($1) AND ord = 0")
                    .bind(ids)
                    .fetch_optional(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn non_arch_posi(&self, seqs: &[i32]) -> Result<Vec<Cow<'static, str>>, crate::conn::KaniDbError> {
        let rows: Vec<String> = self
            .rt
            .block_on(async {
                sqlx::query_scalar(
                    "SELECT DISTINCT sp1.text \
                     FROM sense_prop sp1 \
                     LEFT JOIN sense_prop sp2 \
                            ON sp1.sense_id = sp2.sense_id \
                           AND sp2.tag = 'misc' \
                           AND sp2.text IN ('arch', 'obsc', 'rare') \
                     WHERE sp1.seq = ANY($1) \
                       AND sp1.tag = 'pos' \
                       AND sp2.id IS NULL",
                )
                .bind(seqs)
                .fetch_all(&self.pool)
                .await
            })
            .map_err(crate::conn::KaniDbError::from)?;
        Ok(rows.into_iter().map(Cow::Owned).collect())
    }

    fn arch_only_seqs(&self) -> Result<Vec<i32>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_scalar(
                    "SELECT sense.seq FROM sense \
                     LEFT JOIN sense_prop sp \
                            ON sp.sense_id = sense.id \
                           AND sp.tag = 'misc' \
                           AND sp.text IN ('arch', 'obsc', 'rare') \
                     GROUP BY sense.seq \
                     HAVING bool_and(sp.id IS NOT NULL)",
                )
                .fetch_all(&self.pool)
                .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn counter_seqs(&self) -> Result<Vec<i32>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_scalar(
                    "SELECT DISTINCT seq FROM sense_prop WHERE tag = 'pos' AND text = 'ctr'",
                )
                .fetch_all(&self.pool)
                .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn counter_stag_rows(
        &self,
        tag: &str,
        seqs: &[i32],
    ) -> Result<Vec<(i32, Cow<'static, str>)>, crate::conn::KaniDbError> {
        let rows: Vec<(i32, String)> = self
            .rt
            .block_on(async {
                sqlx::query_as(
                    "SELECT sp.seq, sp.text \
                     FROM sense_prop sp, sense_prop sp1 \
                     WHERE sp.seq = sp1.seq \
                       AND sp.sense_id = sp1.sense_id \
                       AND sp.tag = $1 \
                       AND sp1.tag = 'pos' \
                       AND sp1.text = 'ctr' \
                       AND sp.seq = ANY($2)",
                )
                .bind(tag)
                .bind(seqs)
                .fetch_all(&self.pool)
                .await
            })
            .map_err(crate::conn::KaniDbError::from)?;
        Ok(rows
            .into_iter()
            .map(|(seq, text)| (seq, Cow::Owned(text)))
            .collect())
    }

    fn restricted_readings_by_seq(
        &self,
        seq: i32,
    ) -> Result<Vec<(Cow<'static, str>, Cow<'static, str>)>, crate::conn::KaniDbError> {
        let rows: Vec<(String, String)> = self
            .rt
            .block_on(async {
                sqlx::query_as("SELECT reading, text FROM restricted_readings WHERE seq = $1")
                    .bind(seq)
                    .fetch_all(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)?;
        Ok(rows
            .into_iter()
            .map(|(reading, text)| (Cow::Owned(reading), Cow::Owned(text)))
            .collect())
    }

    // --- kanjidic ---

    fn readings_non_nanori_by_kanji_id(
        &self,
        kanji_id: i32,
    ) -> Result<Vec<Reading>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as(
                    "SELECT * FROM reading WHERE kanji_id = $1 AND NOT (type = 'ja_na') \
                     ORDER BY type DESC, stat_common DESC",
                )
                .bind(kanji_id)
                .fetch_all(&self.pool)
                .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn meanings_by_kanji_id(&self, kanji_id: i32) -> Result<Vec<Meaning>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as("SELECT * FROM meaning WHERE kanji_id = $1 ORDER BY id")
                    .bind(kanji_id)
                    .fetch_all(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn okurigana_texts_by_reading_id(
        &self,
        reading_id: i32,
    ) -> Result<Vec<Cow<'static, str>>, crate::conn::KaniDbError> {
        let rows: Vec<String> = self
            .rt
            .block_on(async {
                sqlx::query_scalar("SELECT DISTINCT text FROM okurigana WHERE reading_id = $1")
                    .bind(reading_id)
                    .fetch_all(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)?;
        Ok(rows.into_iter().map(Cow::Owned).collect())
    }

    fn kanji_by_text(&self, text: &str) -> Result<Vec<Kanji>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as("SELECT * FROM kanji WHERE text = $1")
                    .bind(text)
                    .fetch_all(&self.pool)
                    .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn reading_stats_rows(
        &self,
        kanji: &str,
        reading: &str,
        reading_type: &str,
    ) -> Result<Vec<(i32, i32, Option<i32>)>, crate::conn::KaniDbError> {
        self.rt
            .block_on(async {
                sqlx::query_as(
                    "SELECT r.stat_common, k.stat_common, k.grade FROM kanji k, reading r \
                     WHERE k.id = r.kanji_id AND k.text = $1 AND r.text = $2 AND r.type = $3",
                )
                .bind(kanji)
                .bind(reading)
                .bind(reading_type)
                .fetch_all(&self.pool)
                .await
            })
            .map_err(crate::conn::KaniDbError::from)
    }

    fn kanji_reading_pairs(
        &self,
        text: &str,
        typeset: &[String],
    ) -> Result<Vec<(Cow<'static, str>, Cow<'static, str>)>, crate::conn::KaniDbError> {
        let rows: Vec<(String, String)> = self
            .rt
            .block_on(async {
                sqlx::query_as(
                    "SELECT r.text, r.type FROM kanji k \
                     INNER JOIN reading r ON r.kanji_id = k.id \
                     WHERE k.text = $1 AND r.type <> ALL($2) \
                     ORDER BY r.id",
                )
                .bind(text)
                .bind(typeset)
                .fetch_all(&self.pool)
                .await
            })
            .map_err(crate::conn::KaniDbError::from)?;
        Ok(rows
            .into_iter()
            .map(|(reading, reading_type)| (Cow::Owned(reading), Cow::Owned(reading_type)))
            .collect())
    }
}
