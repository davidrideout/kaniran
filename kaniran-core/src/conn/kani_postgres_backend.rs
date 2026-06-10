//! Rust-only sidecar: the Postgres lookup backend behind
//! [`KaniranContext::store`](crate::conn::kani_context::KaniranContext).
//! Every runtime (lookup-serving) SQL query in the crate lives here as
//! one method per distinct SQL statement; callers go through
//! `ctx.store` and never touch SQL. Build-time code (`dict/load`,
//! `dict/errata`, the kanjidic loaders) writes the database and keeps
//! using `ctx.pool` directly.
//!
//! Method bodies are byte-identical moves of the SQL they replaced —
//! the doc comment on each method names the port function the query
//! came from. A future in-memory backend swaps in behind the same
//! method set.
//!
//! The type is split into one `impl` block per table family (entry;
//! kanji_text/kana_text; conjugation; conj_prop/conj_source_reading;
//! the sense family; the kanjidic family).

use crate::dict::dao::{ConjProp, Conjugation, Entry, KanaText, KanjiText};
use crate::kanji::dao::{Kanji, Meaning, Reading};
use sqlx::PgPool;

/// Postgres-backed dictionary store. Cheap to clone — wraps the
/// connection pool, which is internally reference-counted.
#[derive(Clone)]
pub struct KaniPostgresBackend {
    pool: PgPool,
}

impl KaniPostgresBackend {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Lookups against `entry` — the JMdict entry header rows carrying
/// `seq`, `root_p`, and the `n_kanji`/`n_kana` reading counts.
impl KaniPostgresBackend {
    /// `(get-dao 'entry seq)` — `calc-score` (`dict.lisp:803`),
    /// `best-kanji-conj` (`dict.lisp:461`).
    pub async fn entry_by_seq(&self, seq: i32) -> Result<Option<Entry>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM entry WHERE seq = $1")
            .bind(seq)
            .fetch_optional(&self.pool)
            .await
    }

    /// Root-flagged seqs among `seqs` — `match-unique` `:sa` arm
    /// (`dict-grammar.lisp:486`).
    pub async fn root_seqs(&self, seqs: &[i32]) -> Result<Vec<i32>, sqlx::Error> {
        sqlx::query_scalar("SELECT seq FROM entry WHERE seq = ANY($1) AND root_p")
            .bind(seqs)
            .fetch_all(&self.pool)
            .await
    }

    /// `get-candidates` kana branch (`dict.lisp:1902`).
    pub async fn candidate_seqs_kana(&self, text: &str) -> Result<Vec<i32>, sqlx::Error> {
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
        Ok(rows.into_iter().map(|(seq,)| seq).collect())
    }

    /// `get-candidates` kanji branch (`dict.lisp:1902`).
    pub async fn candidate_seqs_kanji(
        &self,
        text: &str,
        reading: Option<&str>,
    ) -> Result<Vec<i32>, sqlx::Error> {
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
        Ok(rows.into_iter().map(|(seq,)| seq).collect())
    }

    /// `get-kanji-words` (`dict.lisp:1834`).
    pub async fn kanji_words_containing_char(
        &self,
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
        .fetch_all(&self.pool)
        .await
    }
}

/// Lookups against the reading-row tables `kanji_text` / `kana_text`
/// — the kanji writings and kana spellings of each entry. The bulk of
/// segmentation traffic lands here. Most methods come in kanji/kana
/// twins because upstream picks the table by testing whether the input
/// is pure kana; keep both twins together when adding a pair.
impl KaniPostgresBackend {
    /// Ord-0 kanji headword text — `entry` `get-kanji`/`get-text`
    /// (`dict.lisp:47,51`), `reading-str-seq` (`dict.lisp:1584`).
    pub async fn headword_kanji_text(&self, seq: i32) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar("SELECT text FROM kanji_text WHERE seq = $1 AND ord = 0")
            .bind(seq)
            .fetch_optional(&self.pool)
            .await
    }

    /// Ord-0 kana headword text — `entry` `get-kana`/`get-text`
    /// (`dict.lisp:44,47`), `reading-str-seq` (`dict.lisp:1584`).
    pub async fn headword_kana_text(&self, seq: i32) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar("SELECT text FROM kana_text WHERE seq = $1 AND ord = 0")
            .bind(seq)
            .fetch_optional(&self.pool)
            .await
    }

    /// `get-original-text` simple-text arm, kanji table (`dict.lisp:399`).
    pub async fn kanji_texts_by_seq_and_text(
        &self,
        seq: i32,
        text: &str,
    ) -> Result<Vec<KanjiText>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM kanji_text WHERE seq = $1 AND text = $2")
            .bind(seq)
            .bind(text)
            .fetch_all(&self.pool)
            .await
    }

    /// `get-original-text` simple-text arm, kana table (`dict.lisp:399`).
    pub async fn kana_texts_by_seq_and_text(
        &self,
        seq: i32,
        text: &str,
    ) -> Result<Vec<KanaText>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM kana_text WHERE seq = $1 AND text = $2")
            .bind(seq)
            .bind(text)
            .fetch_all(&self.pool)
            .await
    }

    /// `match-sense-restrictions` stagr rows (`dict.lisp:1527`).
    pub async fn kana_texts_by_seq_and_text_any(
        &self,
        seq: i32,
        texts: &[String],
    ) -> Result<Vec<KanaText>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM kana_text WHERE seq = $1 AND text = ANY($2)")
            .bind(seq)
            .bind(texts)
            .fetch_all(&self.pool)
            .await
    }

    /// `match-sense-restrictions` stagk rows (`dict.lisp:1530`).
    pub async fn kanji_texts_by_seq_and_text_any(
        &self,
        seq: i32,
        texts: &[String],
    ) -> Result<Vec<KanjiText>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM kanji_text WHERE seq = $1 AND text = ANY($2)")
            .bind(seq)
            .bind(texts)
            .fetch_all(&self.pool)
            .await
    }

    /// `get-kanji-kana-old` kana walk (`dict.lisp:117`).
    pub async fn kana_texts_by_seq_ordered(&self, seq: i32) -> Result<Vec<KanaText>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM kana_text WHERE seq = $1 ORDER BY ord")
            .bind(seq)
            .fetch_all(&self.pool)
            .await
    }

    /// `(get-dao 'kanji-text pid)` — `best-kana-conj` (`dict.lisp:436`).
    /// Errors with [`sqlx::Error::RowNotFound`] on a missing id,
    /// preserving the fail-loud stance of the callsite.
    pub async fn kanji_text_by_id(&self, id: i32) -> Result<KanjiText, sqlx::Error> {
        sqlx::query_as("SELECT * FROM kanji_text WHERE id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await
    }

    /// `(get-dao 'kana-text pid)` — `best-kanji-conj` (`dict.lisp:465`).
    pub async fn kana_text_by_id(&self, id: i32) -> Result<KanaText, sqlx::Error> {
        sqlx::query_as("SELECT * FROM kana_text WHERE id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await
    }

    /// `find-word` kana arm (`dict.lisp:489`), `word-info-reading`
    /// (`dict.lisp:1773`).
    pub async fn kana_texts_by_text(&self, text: &str) -> Result<Vec<KanaText>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM kana_text WHERE text = $1")
            .bind(text)
            .fetch_all(&self.pool)
            .await
    }

    /// `find-word` kanji arm (`dict.lisp:489`), `word-info-reading`
    /// (`dict.lisp:1773`).
    pub async fn kanji_texts_by_text(&self, text: &str) -> Result<Vec<KanjiText>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM kanji_text WHERE text = $1")
            .bind(text)
            .fetch_all(&self.pool)
            .await
    }

    /// `find-word` `:root-only` kana arm (`dict.lisp:489`).
    pub async fn kana_texts_root_by_text(&self, text: &str) -> Result<Vec<KanaText>, sqlx::Error> {
        sqlx::query_as(
            "SELECT wt.* FROM kana_text wt \
             INNER JOIN entry ON wt.seq = entry.seq \
             WHERE wt.text = $1 AND entry.root_p",
        )
        .bind(text)
        .fetch_all(&self.pool)
        .await
    }

    /// `find-word` `:root-only` kanji arm (`dict.lisp:489`).
    pub async fn kanji_texts_root_by_text(
        &self,
        text: &str,
    ) -> Result<Vec<KanjiText>, sqlx::Error> {
        sqlx::query_as(
            "SELECT wt.* FROM kanji_text wt \
             INNER JOIN entry ON wt.seq = entry.seq \
             WHERE wt.text = $1 AND entry.root_p",
        )
        .bind(text)
        .fetch_all(&self.pool)
        .await
    }

    /// `find-substring-words` kana bulk fetch (`dict.lisp:514`).
    pub async fn kana_texts_by_text_any(
        &self,
        texts: &[String],
    ) -> Result<Vec<KanaText>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM kana_text WHERE text = ANY($1)")
            .bind(texts)
            .fetch_all(&self.pool)
            .await
    }

    /// `find-substring-words` kanji bulk fetch (`dict.lisp:514`).
    pub async fn kanji_texts_by_text_any(
        &self,
        texts: &[String],
    ) -> Result<Vec<KanjiText>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM kanji_text WHERE text = ANY($1)")
            .bind(texts)
            .fetch_all(&self.pool)
            .await
    }

    /// `find-words-seqs` kanji arm (`dict.lisp:532`).
    pub async fn kanji_texts_by_text_any_and_seq_any(
        &self,
        texts: &[&str],
        seqs: &[i32],
    ) -> Result<Vec<KanjiText>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM kanji_text WHERE text = ANY($1) AND seq = ANY($2)")
            .bind(texts)
            .bind(seqs)
            .fetch_all(&self.pool)
            .await
    }

    /// `find-words-seqs` kana arm (`dict.lisp:533`).
    pub async fn kana_texts_by_text_any_and_seq_any(
        &self,
        texts: &[&str],
        seqs: &[i32],
    ) -> Result<Vec<KanaText>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM kana_text WHERE text = ANY($1) AND seq = ANY($2)")
            .bind(texts)
            .bind(seqs)
            .fetch_all(&self.pool)
            .await
    }

    /// `find-word-seq` kana arm (`dict-grammar.lisp:75`).
    pub async fn kana_texts_by_text_and_seq_any(
        &self,
        text: &str,
        seqs: &[i32],
    ) -> Result<Vec<KanaText>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM kana_text WHERE text = $1 AND seq = ANY($2)")
            .bind(text)
            .bind(seqs)
            .fetch_all(&self.pool)
            .await
    }

    /// `find-word-seq` kanji arm (`dict-grammar.lisp:75`).
    pub async fn kanji_texts_by_text_and_seq_any(
        &self,
        text: &str,
        seqs: &[i32],
    ) -> Result<Vec<KanjiText>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM kanji_text WHERE text = $1 AND seq = ANY($2)")
            .bind(text)
            .bind(seqs)
            .fetch_all(&self.pool)
            .await
    }

    /// `find-word-conj-of` kana join (`dict-grammar.lisp:79`).
    pub async fn kana_texts_conj_of(
        &self,
        seqs: &[i32],
        text: &str,
    ) -> Result<Vec<KanaText>, sqlx::Error> {
        sqlx::query_as(
            "SELECT kt.* FROM kana_text kt, conjugation conj \
             WHERE kt.seq = conj.seq AND conj.\"from\" = ANY($1) AND kt.text = $2",
        )
        .bind(seqs)
        .bind(text)
        .fetch_all(&self.pool)
        .await
    }

    /// `find-word-conj-of` kanji join (`dict-grammar.lisp:79`).
    pub async fn kanji_texts_conj_of(
        &self,
        seqs: &[i32],
        text: &str,
    ) -> Result<Vec<KanjiText>, sqlx::Error> {
        sqlx::query_as(
            "SELECT kt.* FROM kanji_text kt, conjugation conj \
             WHERE kt.seq = conj.seq AND conj.\"from\" = ANY($1) AND kt.text = $2",
        )
        .bind(seqs)
        .bind(text)
        .fetch_all(&self.pool)
        .await
    }

    /// `find-word-with-pos` kana arm (`dict-grammar.lisp:89`).
    pub async fn kana_texts_with_pos(
        &self,
        text: &str,
        posi: &[String],
    ) -> Result<Vec<KanaText>, sqlx::Error> {
        sqlx::query_as(
            "SELECT DISTINCT kt.* FROM kana_text kt \
             INNER JOIN sense_prop sp ON sp.seq = kt.seq AND sp.tag = 'pos' \
             WHERE kt.text = $1 AND sp.text = ANY($2)",
        )
        .bind(text)
        .bind(posi)
        .fetch_all(&self.pool)
        .await
    }

    /// `find-word-with-pos` kanji arm (`dict-grammar.lisp:89`).
    pub async fn kanji_texts_with_pos(
        &self,
        text: &str,
        posi: &[String],
    ) -> Result<Vec<KanjiText>, sqlx::Error> {
        sqlx::query_as(
            "SELECT DISTINCT kt.* FROM kanji_text kt \
             INNER JOIN sense_prop sp ON sp.seq = kt.seq AND sp.tag = 'pos' \
             WHERE kt.text = $1 AND sp.text = ANY($2)",
        )
        .bind(text)
        .bind(posi)
        .fetch_all(&self.pool)
        .await
    }

    /// `get-kana-forms*` union of direct and derived rows
    /// (`dict-grammar.lisp:17`).
    pub async fn kana_forms_rows(&self, seq: i32) -> Result<Vec<KanaText>, sqlx::Error> {
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
    }

    /// `get-kana-form` (`dict-grammar.lisp:38`).
    pub async fn kana_texts_by_text_and_seq(
        &self,
        text: &str,
        seq: i32,
    ) -> Result<Vec<KanaText>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM kana_text WHERE text = $1 AND seq = $2")
            .bind(text)
            .bind(seq)
            .fetch_all(&self.pool)
            .await
    }

    /// `word-readings` kana-seq probe (`dict.lisp:537`).
    pub async fn kana_seqs_by_text(&self, text: &str) -> Result<Vec<i32>, sqlx::Error> {
        sqlx::query_scalar("SELECT seq FROM kana_text WHERE text = $1")
            .bind(text)
            .fetch_all(&self.pool)
            .await
    }

    /// `word-readings` kanji-seq probe (`dict.lisp:540`).
    pub async fn kanji_seqs_by_text(&self, text: &str) -> Result<Vec<i32>, sqlx::Error> {
        sqlx::query_scalar("SELECT seq FROM kanji_text WHERE text = $1")
            .bind(text)
            .fetch_all(&self.pool)
            .await
    }

    /// `word-readings` kana spellings for kanji seqs (`dict.lisp:542`).
    pub async fn kana_reading_texts_by_seq_any(
        &self,
        seqs: &[i32],
    ) -> Result<Vec<String>, sqlx::Error> {
        sqlx::query_scalar("SELECT text FROM kana_text WHERE seq = ANY($1) ORDER BY id")
            .bind(seqs)
            .fetch_all(&self.pool)
            .await
    }

    /// `exists-reading` (`dict.lisp:1846`).
    pub async fn kana_seqs_by_seq_and_text(
        &self,
        seq: i32,
        text: &str,
    ) -> Result<Vec<i32>, sqlx::Error> {
        sqlx::query_scalar("SELECT seq FROM kana_text WHERE seq = $1 AND text = $2")
            .bind(seq)
            .bind(text)
            .fetch_all(&self.pool)
            .await
    }

    /// `exists-reading` over a compound word-info seq (`dict.lisp:1866`):
    /// `seq_expr` is postmodern's row-literal rendering of the seq list,
    /// so the statement always fails with SQLSTATE 42883 — reproduced
    /// deliberately so the failure propagates identically.
    pub async fn kana_seqs_by_seq_expr(
        &self,
        seq_expr: &str,
        text: &str,
    ) -> Result<Vec<i32>, sqlx::Error> {
        let query = format!(
            "SELECT seq FROM kana_text WHERE seq = {} AND text = $1",
            seq_expr,
        );
        sqlx::query_scalar(&query)
            .bind(text)
            .fetch_all(&self.pool)
            .await
    }

    /// `find-word-kana-pattern` POSIX-regex match (`dict.lisp:1877`).
    pub async fn kana_texts_by_regex(&self, pattern: &str) -> Result<Vec<KanaText>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM kana_text WHERE text ~ $1")
            .bind(pattern)
            .fetch_all(&self.pool)
            .await
    }

    /// `check-easy-hints` (`dict-split.lisp:908`), `get-counter-readings`
    /// kana rows (`dict-counters.lisp:332`).
    pub async fn kana_texts_by_seq_any(&self, seqs: &[i32]) -> Result<Vec<KanaText>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM kana_text WHERE seq = ANY($1)")
            .bind(seqs)
            .fetch_all(&self.pool)
            .await
    }

    /// `get-counter-readings` kanji rows (`dict-counters.lisp:332`).
    pub async fn kanji_texts_by_seq_any(
        &self,
        seqs: &[i32],
    ) -> Result<Vec<KanjiText>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM kanji_text WHERE seq = ANY($1)")
            .bind(seqs)
            .fetch_all(&self.pool)
            .await
    }
}

/// Lookups against `conjugation` — links recording that entry `seq`
/// was derived from entry `from`, optionally via an intermediate
/// entry. Also holds the two whole-table scans (`no_conj_seqs`,
/// `conj_seqs_from_any`) that populate the `no_conj_data` / `is_arch`
/// caches at context construction.
impl KaniPostgresBackend {
    /// All conjugation rows for a seq — `get-conj-data` (`dict.lisp:340`),
    /// `select-conjs` fallback (`dict.lisp:1603`).
    pub async fn conjs_by_seq(&self, seq: i32) -> Result<Vec<Conjugation>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM conjugation WHERE seq = $1")
            .bind(seq)
            .fetch_all(&self.pool)
            .await
    }

    /// Conjugation rows restricted to ids — `get-conj-data`
    /// (`dict.lisp:340`), `select-conjs` (`dict.lisp:1603`).
    pub async fn conjs_by_seq_and_ids(
        &self,
        seq: i32,
        ids: &[i32],
    ) -> Result<Vec<Conjugation>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM conjugation WHERE seq = $1 AND id = ANY($2)")
            .bind(seq)
            .bind(ids)
            .fetch_all(&self.pool)
            .await
    }

    /// Conjugation rows by source seq — `get-conj-data` (`dict.lisp:340`).
    pub async fn conjs_by_seq_and_from(
        &self,
        seq: i32,
        from: i32,
    ) -> Result<Vec<Conjugation>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM conjugation WHERE seq = $1 AND \"from\" = $2")
            .bind(seq)
            .bind(from)
            .fetch_all(&self.pool)
            .await
    }

    /// Null-via (root) conjugation rows — `select-conjs` (`dict.lisp:1603`).
    pub async fn conjs_by_seq_via_null(&self, seq: i32) -> Result<Vec<Conjugation>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM conjugation WHERE seq = $1 AND via IS NULL")
            .bind(seq)
            .fetch_all(&self.pool)
            .await
    }

    /// `(get-dao 'conjugation conj-id)` — `pair-words-by-conj`
    /// (`dict-grammar.lisp:61`). Errors with
    /// [`sqlx::Error::RowNotFound`] on a missing id.
    pub async fn conj_by_id(&self, id: i32) -> Result<Conjugation, sqlx::Error> {
        sqlx::query_as("SELECT * FROM conjugation WHERE id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await
    }

    /// Seqs among `seqs` that are conjugations of です (seq 2755350) —
    /// `match-unique` `:desu` arm (`dict-grammar.lisp:478`).
    pub async fn conj_seqs_of_desu(&self, seqs: &[i32]) -> Result<Vec<i32>, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT seq FROM conjugation \
             WHERE seq = ANY($1) AND \"from\" = 2755350",
        )
        .bind(seqs)
        .fetch_all(&self.pool)
        .await
    }

    /// Distinct derived seqs of the given roots — `build_is_arch`
    /// second pass (upstream `*is-arch-cache*`, `dict.lisp:745`).
    pub async fn conj_seqs_from_any(&self, seqs: &[i32]) -> Result<Vec<i32>, sqlx::Error> {
        sqlx::query_scalar("SELECT DISTINCT seq FROM conjugation WHERE \"from\" = ANY($1)")
            .bind(seqs)
            .fetch_all(&self.pool)
            .await
    }

    /// Seqs with no conjugation rows — `build_no_conj_data` (upstream
    /// `*no-conj-data*`, `dict.lisp:329`).
    pub async fn no_conj_seqs(&self) -> Result<Vec<i32>, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT entry.seq FROM entry \
             LEFT JOIN conjugation c ON entry.seq = c.seq \
             WHERE c.seq IS NULL",
        )
        .fetch_all(&self.pool)
        .await
    }
}

/// Lookups against `conj_prop` (which grammatical form a conjugation
/// row represents: pos, conj-type, negative/formal flags) and
/// `conj_source_reading` (pairs tying a rendered conjugated text to
/// the dictionary surface form it derives from), plus the
/// parent-reading joins that walk a conjugated reading back to the
/// reading row that produced it.
impl KaniPostgresBackend {
    /// `(select-dao 'conj-prop (:= 'conj-id …))` — `get-conj-data`
    /// (`dict.lisp:340`), `select-conjs-and-props` (`dict.lisp:1638`).
    pub async fn conj_props_by_conj_id(&self, conj_id: i32) -> Result<Vec<ConjProp>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM conj_prop WHERE conj_id = $1")
            .bind(conj_id)
            .fetch_all(&self.pool)
            .await
    }

    /// `(text, source_text)` pairs of a conjugation — `get-conj-data`
    /// (`dict.lisp:340`).
    pub async fn conj_source_readings_by_conj_id(
        &self,
        conj_id: i32,
    ) -> Result<Vec<(String, String)>, sqlx::Error> {
        sqlx::query_as("SELECT text, source_text FROM conj_source_reading WHERE conj_id = $1")
            .bind(conj_id)
            .fetch_all(&self.pool)
            .await
    }

    /// `(text, source_text)` pairs restricted to surface texts —
    /// `get-conj-data` (`dict.lisp:340`).
    pub async fn conj_source_readings_by_conj_id_and_texts(
        &self,
        conj_id: i32,
        texts: &[String],
    ) -> Result<Vec<(String, String)>, sqlx::Error> {
        sqlx::query_as(
            "SELECT text, source_text FROM conj_source_reading \
             WHERE conj_id = $1 AND text = ANY($2)",
        )
        .bind(conj_id)
        .bind(texts)
        .fetch_all(&self.pool)
        .await
    }

    /// Rendered texts of a conjugation derived from `source_text` —
    /// `best-kana-conj` (`dict.lisp:439`), `best-kanji-conj`
    /// (`dict.lisp:467`).
    pub async fn conj_source_reading_texts(
        &self,
        conj_id: i32,
        source_text: &str,
    ) -> Result<Vec<String>, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT text FROM conj_source_reading \
             WHERE conj_id = $1 AND source_text = $2",
        )
        .bind(conj_id)
        .bind(source_text)
        .fetch_all(&self.pool)
        .await
    }

    /// `query-parents-kanji` (`dict.lisp:404`).
    pub async fn parents_kanji(
        &self,
        seq: i32,
        text: &str,
    ) -> Result<Vec<(i32, i32)>, sqlx::Error> {
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
    }

    /// `query-parents-kana` (`dict.lisp:417`).
    pub async fn parents_kana(&self, seq: i32, text: &str) -> Result<Vec<(i32, i32)>, sqlx::Error> {
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
    }
}

/// Lookups against the meaning-side tables: `sense` (numbered
/// meanings of an entry), `gloss` (English translations), `sense_prop`
/// (tagged sense properties — `pos`, `misc`, the `stagk`/`stagr`
/// restrictions), and `restricted_readings` (kana↔kanji pairing
/// restrictions). Also holds the startup scans for the `is_arch` and
/// counter caches, which classify entries by their sense tags.
impl KaniPostgresBackend {
    /// `(ord, joined-gloss)` per sense of a seq — `get-senses-raw`
    /// (`dict.lisp:1458`).
    pub async fn sense_gloss_rows(
        &self,
        seq: i32,
    ) -> Result<Vec<(i32, Option<String>)>, sqlx::Error> {
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
    }

    /// `(sense-ord, tag, text)` prop rows of a seq restricted to `tags`
    /// — `get-senses-raw` (`dict.lisp:1458`).
    pub async fn sense_prop_rows_tagged(
        &self,
        seq: i32,
        tags: &[&str],
    ) -> Result<Vec<(i32, String, String)>, sqlx::Error> {
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
    }

    /// First-sense joined gloss — `short-sense-str` (`dict.lisp:1562`).
    /// Outer [`Option`] is row presence, inner is the nullable aggregate.
    pub async fn first_sense_gloss(
        &self,
        seq: i32,
    ) -> Result<Option<Option<String>>, sqlx::Error> {
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
    }

    /// First-sense joined gloss restricted to a pos tag —
    /// `short-sense-str` (`dict.lisp:1562`).
    pub async fn first_sense_gloss_with_pos(
        &self,
        seq: i32,
        pos: &str,
    ) -> Result<Option<Option<String>>, sqlx::Error> {
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
    }

    /// `(seq, gloss-text)` rows for a seq set — `get-glosses`
    /// (`dict.lisp:1892`).
    pub async fn glosses_by_seq_any(
        &self,
        seqs: &[i32],
    ) -> Result<Vec<(i32, String)>, sqlx::Error> {
        sqlx::query_as(
            "SELECT sense.seq, gloss.text FROM gloss, sense \
             WHERE sense.seq = ANY($1) AND gloss.sense_id = sense.id \
             ORDER BY sense.seq",
        )
        .bind(seqs)
        .fetch_all(&self.pool)
        .await
    }

    /// Sense ids tagged `misc=uk` (usually-kana) — `calc-score`
    /// (`dict.lisp:822`).
    pub async fn uk_sense_ids(&self, seqs: &[i32]) -> Result<Vec<i32>, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT sense_id FROM sense_prop \
             WHERE seq = ANY($1) AND tag = 'misc' AND text = 'uk'",
        )
        .bind(seqs)
        .fetch_all(&self.pool)
        .await
    }

    /// Any of `ids` belonging to an ord-0 sense — `calc-score`
    /// (`dict.lisp:884`).
    pub async fn sense_id_ord0(&self, ids: &[i32]) -> Result<Option<i32>, sqlx::Error> {
        sqlx::query_scalar("SELECT id FROM sense WHERE id = ANY($1) AND ord = 0")
            .bind(ids)
            .fetch_optional(&self.pool)
            .await
    }

    /// `get-non-arch-posi` anti-join (`dict.lisp:762`).
    pub async fn non_arch_posi(&self, seqs: &[i32]) -> Result<Vec<String>, sqlx::Error> {
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
    }

    /// Seqs whose every sense is archaic/obscure/rare — `build_is_arch`
    /// first pass (upstream `*is-arch-cache*`, `dict.lisp:745`).
    pub async fn arch_only_seqs(&self) -> Result<Vec<i32>, sqlx::Error> {
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
    }

    /// Seqs tagged `pos=ctr` — `get-counter-ids`
    /// (`dict-counters.lisp:283`).
    pub async fn counter_seqs(&self) -> Result<Vec<i32>, sqlx::Error> {
        sqlx::query_scalar("SELECT DISTINCT seq FROM sense_prop WHERE tag = 'pos' AND text = 'ctr'")
            .fetch_all(&self.pool)
            .await
    }

    /// `(seq, text)` restriction rows on counter senses for one stag
    /// tag (`stagk` or `stagr`) — `get-counter-stags`
    /// (`dict-counters.lisp:291`).
    pub async fn counter_stag_rows(
        &self,
        tag: &str,
        seqs: &[i32],
    ) -> Result<Vec<(i32, String)>, sqlx::Error> {
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
    }

    /// `(reading, text)` restriction pairs of a seq —
    /// `match-sense-restrictions` (`dict.lisp:1524`).
    pub async fn restricted_readings_by_seq(
        &self,
        seq: i32,
    ) -> Result<Vec<(String, String)>, sqlx::Error> {
        sqlx::query_as("SELECT reading, text FROM restricted_readings WHERE seq = $1")
            .bind(seq)
            .fetch_all(&self.pool)
            .await
    }
}

/// Lookups against the kanjidic2 tables `kanji`, `reading`,
/// `okurigana`, and `meaning` — per-character info serving the
/// kanji-info JSON output and the lazily-filled reading-stats cache.
/// Not to be confused with `kanji_text` above: that table holds JMdict
/// word writings, these hold single-character kanjidic data.
impl KaniPostgresBackend {
    /// Non-nanori readings of a kanji row — `to-json` (`kanji.lisp:379`).
    pub async fn readings_non_nanori_by_kanji_id(
        &self,
        kanji_id: i32,
    ) -> Result<Vec<Reading>, sqlx::Error> {
        sqlx::query_as(
            "SELECT * FROM reading WHERE kanji_id = $1 AND NOT (type = 'ja_na') \
             ORDER BY type DESC, stat_common DESC",
        )
        .bind(kanji_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Meanings of a kanji row — `to-json` (`kanji.lisp:384`).
    pub async fn meanings_by_kanji_id(&self, kanji_id: i32) -> Result<Vec<Meaning>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM meaning WHERE kanji_id = $1 ORDER BY id")
            .bind(kanji_id)
            .fetch_all(&self.pool)
            .await
    }

    /// Distinct okurigana of a reading — `reading-info-json`
    /// (`kanji.lisp:360`).
    pub async fn okurigana_texts_by_reading_id(
        &self,
        reading_id: i32,
    ) -> Result<Vec<String>, sqlx::Error> {
        sqlx::query_scalar("SELECT DISTINCT text FROM okurigana WHERE reading_id = $1")
            .bind(reading_id)
            .fetch_all(&self.pool)
            .await
    }

    /// `(select-dao 'kanji (:= 'text str))` — `kanji-info-json`
    /// (`kanji.lisp:395`).
    pub async fn kanji_by_text(&self, text: &str) -> Result<Vec<Kanji>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM kanji WHERE text = $1")
            .bind(text)
            .fetch_all(&self.pool)
            .await
    }

    /// `(query-dao 'kanji query)` — `query-kanji-json` (`kanji.lisp:458`);
    /// the caller supplies the full SQL statement.
    pub async fn kanji_by_raw_query(&self, query: &str) -> Result<Vec<Kanji>, sqlx::Error> {
        sqlx::query_as(query).fetch_all(&self.pool).await
    }

    /// `(reading.stat_common, kanji.stat_common, kanji.grade)` rows for
    /// a `(kanji, reading, type)` match — `get-reading-stats`
    /// (`kanji.lisp:399`).
    pub async fn reading_stats_rows(
        &self,
        kanji: &str,
        reading: &str,
        reading_type: &str,
    ) -> Result<Vec<(i32, i32, Option<i32>)>, sqlx::Error> {
        sqlx::query_as(
            "SELECT r.stat_common, k.stat_common, k.grade FROM kanji k, reading r \
             WHERE k.id = r.kanji_id AND k.text = $1 AND r.text = $2 AND r.type = $3",
        )
        .bind(kanji)
        .bind(reading)
        .bind(reading_type)
        .fetch_all(&self.pool)
        .await
    }

    /// `(text, type)` reading pairs of a kanji excluding `typeset` —
    /// `get-readings-cache` (`kanji.lisp:201`). The `ORDER BY r.id` is
    /// a deliberate divergence from upstream's unordered SELECT (see
    /// the callsite comment in [`crate::kanji::readings`]).
    pub async fn kanji_reading_pairs(
        &self,
        text: &str,
        typeset: &[String],
    ) -> Result<Vec<(String, String)>, sqlx::Error> {
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
    }
}
