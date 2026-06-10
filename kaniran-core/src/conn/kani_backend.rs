//! Rust-only sidecar: the backend-agnostic dictionary-store contract.
//! [`KaniBackend`] declares one method per runtime lookup the crate
//! performs — the surface [`KaniPostgresBackend`] serves with SQL and
//! [`KaniRkyvBackend`](crate::conn::kani_rkyv_backend::KaniRkyvBackend)
//! serves from a memory-mapped snapshot. [`KaniStore`] is the
//! runtime-selected wrapper held by
//! [`KaniranContext::store`](crate::conn::kani_context::KaniranContext::store):
//! the backend is picked once at construction and every call statically
//! dispatches through a match — no boxed futures.
//!
//! The doc comment on each trait method names the upstream function the
//! lookup came from; the SQL itself lives in the Postgres impl. Two
//! Postgres-only operations deliberately sit OUTSIDE the trait:
//! `kanji_by_raw_query` (caller-supplied SQL, exposed as an inherent
//! method on [`KaniStore`] that fails on non-Postgres backends) and the
//! compound-seq `exists-reading` probe (its always-failing query is
//! synthesized at the callsite in [`crate::dict::find_word_info`]).

use crate::conn::kani_postgres_backend::KaniPostgresBackend;
#[cfg(feature = "rkyv")]
use crate::conn::kani_rkyv_backend::KaniRkyvBackend;
use crate::dict::dao::{ConjProp, Conjugation, Entry, KanaText, KanjiText};
use crate::kanji::dao::{Kanji, Meaning, Reading};

/// Every runtime dictionary lookup, one method per distinct query.
/// Implementations must reproduce Postgres result ORDER — including
/// the physical-row (insertion) order Postgres yields for queries with
/// no `ORDER BY` — because segmentation scoring is order-sensitive.
///
/// Callers always dispatch through concrete types ([`KaniStore`] or a
/// backend directly), so the returned futures' auto traits resolve
/// structurally and no `Send` bounds are needed on the trait itself.
#[allow(async_fn_in_trait)]
pub trait KaniBackend {
    // --- entry: the JMdict entry header rows carrying `seq`,
    // `root_p`, and the `n_kanji`/`n_kana` reading counts. ---

    /// `(get-dao 'entry seq)` — `calc-score` (`dict.lisp:803`),
    /// `best-kanji-conj` (`dict.lisp:461`).
    async fn entry_by_seq(&self, seq: i32) -> Result<Option<Entry>, sqlx::Error>;

    /// Root-flagged seqs among `seqs` — `match-unique` `:sa` arm
    /// (`dict-grammar.lisp:486`).
    async fn root_seqs(&self, seqs: &[i32]) -> Result<Vec<i32>, sqlx::Error>;

    /// `get-candidates` kana branch (`dict.lisp:1902`).
    async fn candidate_seqs_kana(&self, text: &str) -> Result<Vec<i32>, sqlx::Error>;

    /// `get-candidates` kanji branch (`dict.lisp:1902`).
    async fn candidate_seqs_kanji(
        &self,
        text: &str,
        reading: Option<&str>,
    ) -> Result<Vec<i32>, sqlx::Error>;

    /// `get-kanji-words` (`dict.lisp:1834`).
    async fn kanji_words_containing_char(
        &self,
        char: &str,
    ) -> Result<Vec<(i32, String, String, i32)>, sqlx::Error>;

    // --- kanji_text / kana_text: the kanji writings and kana
    // spellings of each entry. The bulk of segmentation traffic lands
    // here. Most methods come in kanji/kana twins because upstream
    // picks the table by testing whether the input is pure kana. ---

    /// Ord-0 kanji headword text — `entry` `get-kanji`/`get-text`
    /// (`dict.lisp:47,51`), `reading-str-seq` (`dict.lisp:1584`).
    async fn headword_kanji_text(&self, seq: i32) -> Result<Option<String>, sqlx::Error>;

    /// Ord-0 kana headword text — `entry` `get-kana`/`get-text`
    /// (`dict.lisp:44,47`), `reading-str-seq` (`dict.lisp:1584`).
    async fn headword_kana_text(&self, seq: i32) -> Result<Option<String>, sqlx::Error>;

    /// `get-original-text` simple-text arm, kanji table (`dict.lisp:399`).
    async fn kanji_texts_by_seq_and_text(
        &self,
        seq: i32,
        text: &str,
    ) -> Result<Vec<KanjiText>, sqlx::Error>;

    /// `get-original-text` simple-text arm, kana table (`dict.lisp:399`).
    async fn kana_texts_by_seq_and_text(
        &self,
        seq: i32,
        text: &str,
    ) -> Result<Vec<KanaText>, sqlx::Error>;

    /// `match-sense-restrictions` stagr rows (`dict.lisp:1527`).
    async fn kana_texts_by_seq_and_text_any(
        &self,
        seq: i32,
        texts: &[String],
    ) -> Result<Vec<KanaText>, sqlx::Error>;

    /// `match-sense-restrictions` stagk rows (`dict.lisp:1530`).
    async fn kanji_texts_by_seq_and_text_any(
        &self,
        seq: i32,
        texts: &[String],
    ) -> Result<Vec<KanjiText>, sqlx::Error>;

    /// `get-kanji-kana-old` kana walk (`dict.lisp:117`).
    async fn kana_texts_by_seq_ordered(&self, seq: i32) -> Result<Vec<KanaText>, sqlx::Error>;

    /// `(get-dao 'kanji-text pid)` — `best-kana-conj` (`dict.lisp:436`).
    /// Errors with [`sqlx::Error::RowNotFound`] on a missing id,
    /// preserving the fail-loud stance of the callsite.
    async fn kanji_text_by_id(&self, id: i32) -> Result<KanjiText, sqlx::Error>;

    /// `(get-dao 'kana-text pid)` — `best-kanji-conj` (`dict.lisp:465`).
    async fn kana_text_by_id(&self, id: i32) -> Result<KanaText, sqlx::Error>;

    /// `find-word` kana arm (`dict.lisp:489`), `word-info-reading`
    /// (`dict.lisp:1773`).
    async fn kana_texts_by_text(&self, text: &str) -> Result<Vec<KanaText>, sqlx::Error>;

    /// `find-word` kanji arm (`dict.lisp:489`), `word-info-reading`
    /// (`dict.lisp:1773`).
    async fn kanji_texts_by_text(&self, text: &str) -> Result<Vec<KanjiText>, sqlx::Error>;

    /// `find-word` `:root-only` kana arm (`dict.lisp:489`).
    async fn kana_texts_root_by_text(&self, text: &str) -> Result<Vec<KanaText>, sqlx::Error>;

    /// `find-word` `:root-only` kanji arm (`dict.lisp:489`).
    async fn kanji_texts_root_by_text(&self, text: &str) -> Result<Vec<KanjiText>, sqlx::Error>;

    /// `find-substring-words` kana bulk fetch (`dict.lisp:514`).
    async fn kana_texts_by_text_any(
        &self,
        texts: &[String],
    ) -> Result<Vec<KanaText>, sqlx::Error>;

    /// `find-substring-words` kanji bulk fetch (`dict.lisp:514`).
    async fn kanji_texts_by_text_any(
        &self,
        texts: &[String],
    ) -> Result<Vec<KanjiText>, sqlx::Error>;

    /// `find-words-seqs` kanji arm (`dict.lisp:532`).
    async fn kanji_texts_by_text_any_and_seq_any(
        &self,
        texts: &[&str],
        seqs: &[i32],
    ) -> Result<Vec<KanjiText>, sqlx::Error>;

    /// `find-words-seqs` kana arm (`dict.lisp:533`).
    async fn kana_texts_by_text_any_and_seq_any(
        &self,
        texts: &[&str],
        seqs: &[i32],
    ) -> Result<Vec<KanaText>, sqlx::Error>;

    /// `find-word-seq` kana arm (`dict-grammar.lisp:75`).
    async fn kana_texts_by_text_and_seq_any(
        &self,
        text: &str,
        seqs: &[i32],
    ) -> Result<Vec<KanaText>, sqlx::Error>;

    /// `find-word-seq` kanji arm (`dict-grammar.lisp:75`).
    async fn kanji_texts_by_text_and_seq_any(
        &self,
        text: &str,
        seqs: &[i32],
    ) -> Result<Vec<KanjiText>, sqlx::Error>;

    /// `find-word-conj-of` kana join (`dict-grammar.lisp:79`).
    async fn kana_texts_conj_of(
        &self,
        seqs: &[i32],
        text: &str,
    ) -> Result<Vec<KanaText>, sqlx::Error>;

    /// `find-word-conj-of` kanji join (`dict-grammar.lisp:79`).
    async fn kanji_texts_conj_of(
        &self,
        seqs: &[i32],
        text: &str,
    ) -> Result<Vec<KanjiText>, sqlx::Error>;

    /// `find-word-with-pos` kana arm (`dict-grammar.lisp:89`).
    async fn kana_texts_with_pos(
        &self,
        text: &str,
        posi: &[String],
    ) -> Result<Vec<KanaText>, sqlx::Error>;

    /// `find-word-with-pos` kanji arm (`dict-grammar.lisp:89`).
    async fn kanji_texts_with_pos(
        &self,
        text: &str,
        posi: &[String],
    ) -> Result<Vec<KanjiText>, sqlx::Error>;

    /// `get-kana-forms*` union of direct and derived rows
    /// (`dict-grammar.lisp:17`).
    async fn kana_forms_rows(&self, seq: i32) -> Result<Vec<KanaText>, sqlx::Error>;

    /// `get-kana-form` (`dict-grammar.lisp:38`).
    async fn kana_texts_by_text_and_seq(
        &self,
        text: &str,
        seq: i32,
    ) -> Result<Vec<KanaText>, sqlx::Error>;

    /// `word-readings` kana-seq probe (`dict.lisp:537`).
    async fn kana_seqs_by_text(&self, text: &str) -> Result<Vec<i32>, sqlx::Error>;

    /// `word-readings` kanji-seq probe (`dict.lisp:540`).
    async fn kanji_seqs_by_text(&self, text: &str) -> Result<Vec<i32>, sqlx::Error>;

    /// `word-readings` kana spellings for kanji seqs (`dict.lisp:542`).
    async fn kana_reading_texts_by_seq_any(
        &self,
        seqs: &[i32],
    ) -> Result<Vec<String>, sqlx::Error>;

    /// `exists-reading` (`dict.lisp:1846`).
    async fn kana_seqs_by_seq_and_text(
        &self,
        seq: i32,
        text: &str,
    ) -> Result<Vec<i32>, sqlx::Error>;

    /// `find-word-kana-pattern` POSIX-regex match (`dict.lisp:1877`).
    async fn kana_texts_by_regex(&self, pattern: &str) -> Result<Vec<KanaText>, sqlx::Error>;

    /// `check-easy-hints` (`dict-split.lisp:908`), `get-counter-readings`
    /// kana rows (`dict-counters.lisp:332`).
    async fn kana_texts_by_seq_any(&self, seqs: &[i32]) -> Result<Vec<KanaText>, sqlx::Error>;

    /// `get-counter-readings` kanji rows (`dict-counters.lisp:332`).
    async fn kanji_texts_by_seq_any(&self, seqs: &[i32]) -> Result<Vec<KanjiText>, sqlx::Error>;

    // --- conjugation: links recording that entry `seq` was derived
    // from entry `from`, optionally via an intermediate entry. ---

    /// All conjugation rows for a seq — `get-conj-data` (`dict.lisp:340`),
    /// `select-conjs` fallback (`dict.lisp:1603`).
    async fn conjs_by_seq(&self, seq: i32) -> Result<Vec<Conjugation>, sqlx::Error>;

    /// Conjugation rows restricted to ids — `get-conj-data`
    /// (`dict.lisp:340`), `select-conjs` (`dict.lisp:1603`).
    async fn conjs_by_seq_and_ids(
        &self,
        seq: i32,
        ids: &[i32],
    ) -> Result<Vec<Conjugation>, sqlx::Error>;

    /// Conjugation rows by source seq — `get-conj-data` (`dict.lisp:340`).
    async fn conjs_by_seq_and_from(
        &self,
        seq: i32,
        from: i32,
    ) -> Result<Vec<Conjugation>, sqlx::Error>;

    /// Null-via (root) conjugation rows — `select-conjs` (`dict.lisp:1603`).
    async fn conjs_by_seq_via_null(&self, seq: i32) -> Result<Vec<Conjugation>, sqlx::Error>;

    /// `(get-dao 'conjugation conj-id)` — `pair-words-by-conj`
    /// (`dict-grammar.lisp:61`). Errors with
    /// [`sqlx::Error::RowNotFound`] on a missing id.
    async fn conj_by_id(&self, id: i32) -> Result<Conjugation, sqlx::Error>;

    /// Seqs among `seqs` that are conjugations of です (seq 2755350) —
    /// `match-unique` `:desu` arm (`dict-grammar.lisp:478`).
    async fn conj_seqs_of_desu(&self, seqs: &[i32]) -> Result<Vec<i32>, sqlx::Error>;

    /// Distinct derived seqs of the given roots — `build_is_arch`
    /// second pass (upstream `*is-arch-cache*`, `dict.lisp:745`).
    async fn conj_seqs_from_any(&self, seqs: &[i32]) -> Result<Vec<i32>, sqlx::Error>;

    /// Seqs with no conjugation rows — `build_no_conj_data` (upstream
    /// `*no-conj-data*`, `dict.lisp:329`).
    async fn no_conj_seqs(&self) -> Result<Vec<i32>, sqlx::Error>;

    // --- conj_prop (which grammatical form a conjugation row
    // represents) and conj_source_reading (pairs tying a rendered
    // conjugated text to the dictionary surface form it derives
    // from), plus the parent-reading joins. ---

    /// `(select-dao 'conj-prop (:= 'conj-id …))` — `get-conj-data`
    /// (`dict.lisp:340`), `select-conjs-and-props` (`dict.lisp:1638`).
    async fn conj_props_by_conj_id(&self, conj_id: i32) -> Result<Vec<ConjProp>, sqlx::Error>;

    /// `(text, source_text)` pairs of a conjugation — `get-conj-data`
    /// (`dict.lisp:340`).
    async fn conj_source_readings_by_conj_id(
        &self,
        conj_id: i32,
    ) -> Result<Vec<(String, String)>, sqlx::Error>;

    /// `(text, source_text)` pairs restricted to surface texts —
    /// `get-conj-data` (`dict.lisp:340`).
    async fn conj_source_readings_by_conj_id_and_texts(
        &self,
        conj_id: i32,
        texts: &[String],
    ) -> Result<Vec<(String, String)>, sqlx::Error>;

    /// Rendered texts of a conjugation derived from `source_text` —
    /// `best-kana-conj` (`dict.lisp:439`), `best-kanji-conj`
    /// (`dict.lisp:467`).
    async fn conj_source_reading_texts(
        &self,
        conj_id: i32,
        source_text: &str,
    ) -> Result<Vec<String>, sqlx::Error>;

    /// `query-parents-kanji` (`dict.lisp:404`).
    async fn parents_kanji(
        &self,
        seq: i32,
        text: &str,
    ) -> Result<Vec<(i32, i32)>, sqlx::Error>;

    /// `query-parents-kana` (`dict.lisp:417`).
    async fn parents_kana(&self, seq: i32, text: &str)
        -> Result<Vec<(i32, i32)>, sqlx::Error>;

    // --- sense / gloss / sense_prop / restricted_readings: the
    // meaning-side tables, plus the startup scans for the `is_arch`
    // and counter caches. ---

    /// `(ord, joined-gloss)` per sense of a seq — `get-senses-raw`
    /// (`dict.lisp:1458`).
    async fn sense_gloss_rows(
        &self,
        seq: i32,
    ) -> Result<Vec<(i32, Option<String>)>, sqlx::Error>;

    /// `(sense-ord, tag, text)` prop rows of a seq restricted to `tags`
    /// — `get-senses-raw` (`dict.lisp:1458`).
    async fn sense_prop_rows_tagged(
        &self,
        seq: i32,
        tags: &[&str],
    ) -> Result<Vec<(i32, String, String)>, sqlx::Error>;

    /// First-sense joined gloss — `short-sense-str` (`dict.lisp:1562`).
    /// Outer [`Option`] is row presence, inner is the nullable aggregate.
    async fn first_sense_gloss(&self, seq: i32) -> Result<Option<Option<String>>, sqlx::Error>;

    /// First-sense joined gloss restricted to a pos tag —
    /// `short-sense-str` (`dict.lisp:1562`).
    async fn first_sense_gloss_with_pos(
        &self,
        seq: i32,
        pos: &str,
    ) -> Result<Option<Option<String>>, sqlx::Error>;

    /// `(seq, gloss-text)` rows for a seq set — `get-glosses`
    /// (`dict.lisp:1892`).
    async fn glosses_by_seq_any(
        &self,
        seqs: &[i32],
    ) -> Result<Vec<(i32, String)>, sqlx::Error>;

    /// Sense ids tagged `misc=uk` (usually-kana) — `calc-score`
    /// (`dict.lisp:822`).
    async fn uk_sense_ids(&self, seqs: &[i32]) -> Result<Vec<i32>, sqlx::Error>;

    /// Any of `ids` belonging to an ord-0 sense — `calc-score`
    /// (`dict.lisp:884`).
    async fn sense_id_ord0(&self, ids: &[i32]) -> Result<Option<i32>, sqlx::Error>;

    /// `get-non-arch-posi` anti-join (`dict.lisp:762`).
    async fn non_arch_posi(&self, seqs: &[i32]) -> Result<Vec<String>, sqlx::Error>;

    /// Seqs whose every sense is archaic/obscure/rare — `build_is_arch`
    /// first pass (upstream `*is-arch-cache*`, `dict.lisp:745`).
    async fn arch_only_seqs(&self) -> Result<Vec<i32>, sqlx::Error>;

    /// Seqs tagged `pos=ctr` — `get-counter-ids`
    /// (`dict-counters.lisp:283`).
    async fn counter_seqs(&self) -> Result<Vec<i32>, sqlx::Error>;

    /// `(seq, text)` restriction rows on counter senses for one stag
    /// tag (`stagk` or `stagr`) — `get-counter-stags`
    /// (`dict-counters.lisp:291`).
    async fn counter_stag_rows(
        &self,
        tag: &str,
        seqs: &[i32],
    ) -> Result<Vec<(i32, String)>, sqlx::Error>;

    /// `(reading, text)` restriction pairs of a seq —
    /// `match-sense-restrictions` (`dict.lisp:1524`).
    async fn restricted_readings_by_seq(
        &self,
        seq: i32,
    ) -> Result<Vec<(String, String)>, sqlx::Error>;

    // --- kanjidic: per-character info serving the kanji-info JSON
    // output and the lazily-filled reading-stats cache. Not to be
    // confused with kanji_text above: that table holds JMdict word
    // writings, these hold single-character kanjidic data. ---

    /// Non-nanori readings of a kanji row — `to-json` (`kanji.lisp:379`).
    async fn readings_non_nanori_by_kanji_id(
        &self,
        kanji_id: i32,
    ) -> Result<Vec<Reading>, sqlx::Error>;

    /// Meanings of a kanji row — `to-json` (`kanji.lisp:384`).
    async fn meanings_by_kanji_id(&self, kanji_id: i32) -> Result<Vec<Meaning>, sqlx::Error>;

    /// Distinct okurigana of a reading — `reading-info-json`
    /// (`kanji.lisp:360`).
    async fn okurigana_texts_by_reading_id(
        &self,
        reading_id: i32,
    ) -> Result<Vec<String>, sqlx::Error>;

    /// `(select-dao 'kanji (:= 'text str))` — `kanji-info-json`
    /// (`kanji.lisp:395`).
    async fn kanji_by_text(&self, text: &str) -> Result<Vec<Kanji>, sqlx::Error>;

    /// `(reading.stat_common, kanji.stat_common, kanji.grade)` rows for
    /// a `(kanji, reading, type)` match — `get-reading-stats`
    /// (`kanji.lisp:399`).
    async fn reading_stats_rows(
        &self,
        kanji: &str,
        reading: &str,
        reading_type: &str,
    ) -> Result<Vec<(i32, i32, Option<i32>)>, sqlx::Error>;

    /// `(text, type)` reading pairs of a kanji excluding `typeset` —
    /// `get-readings-cache` (`kanji.lisp:201`). The `ORDER BY r.id` is
    /// a deliberate divergence from upstream's unordered SELECT (see
    /// the callsite comment in [`crate::kanji::readings`]).
    async fn kanji_reading_pairs(
        &self,
        text: &str,
        typeset: &[String],
    ) -> Result<Vec<(String, String)>, sqlx::Error>;
}

/// Runtime-selected dictionary store held by `ctx.store`. Statically
/// dispatched: each call is a match on the variant, fully inlinable.
/// Cheap to clone — both backends are reference-counted handles.
#[derive(Clone)]
pub enum KaniStore {
    Postgres(KaniPostgresBackend),
    #[cfg(feature = "rkyv")]
    Rkyv(KaniRkyvBackend),
}

impl KaniStore {
    /// `(query-dao 'kanji query)` — `query-kanji-json` (`kanji.lisp:458`);
    /// the caller supplies the full SQL statement, so this is
    /// Postgres-only and lives outside [`KaniBackend`]. Non-Postgres
    /// backends fail loud.
    pub async fn kanji_by_raw_query(&self, query: &str) -> Result<Vec<Kanji>, sqlx::Error> {
        match self {
            KaniStore::Postgres(backend) => backend.kanji_by_raw_query(query).await,
            #[cfg(feature = "rkyv")]
            KaniStore::Rkyv(_) => Err(sqlx::Error::Protocol(
                "kanji_by_raw_query takes caller-supplied SQL and is Postgres-only".into(),
            )),
        }
    }
}

/// Delegation only — every method matches the variant and forwards.
macro_rules! kani_store_delegate {
    ($self:ident, $backend:ident => $call:expr) => {
        match $self {
            KaniStore::Postgres($backend) => $call,
            #[cfg(feature = "rkyv")]
            KaniStore::Rkyv($backend) => $call,
        }
    };
}

impl KaniBackend for KaniStore {
    async fn entry_by_seq(&self, seq: i32) -> Result<Option<Entry>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.entry_by_seq(seq).await)
    }

    async fn root_seqs(&self, seqs: &[i32]) -> Result<Vec<i32>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.root_seqs(seqs).await)
    }

    async fn candidate_seqs_kana(&self, text: &str) -> Result<Vec<i32>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.candidate_seqs_kana(text).await)
    }

    async fn candidate_seqs_kanji(
        &self,
        text: &str,
        reading: Option<&str>,
    ) -> Result<Vec<i32>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.candidate_seqs_kanji(text, reading).await)
    }

    async fn kanji_words_containing_char(
        &self,
        char: &str,
    ) -> Result<Vec<(i32, String, String, i32)>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.kanji_words_containing_char(char).await)
    }

    async fn headword_kanji_text(&self, seq: i32) -> Result<Option<String>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.headword_kanji_text(seq).await)
    }

    async fn headword_kana_text(&self, seq: i32) -> Result<Option<String>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.headword_kana_text(seq).await)
    }

    async fn kanji_texts_by_seq_and_text(
        &self,
        seq: i32,
        text: &str,
    ) -> Result<Vec<KanjiText>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.kanji_texts_by_seq_and_text(seq, text).await)
    }

    async fn kana_texts_by_seq_and_text(
        &self,
        seq: i32,
        text: &str,
    ) -> Result<Vec<KanaText>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.kana_texts_by_seq_and_text(seq, text).await)
    }

    async fn kana_texts_by_seq_and_text_any(
        &self,
        seq: i32,
        texts: &[String],
    ) -> Result<Vec<KanaText>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.kana_texts_by_seq_and_text_any(seq, texts).await)
    }

    async fn kanji_texts_by_seq_and_text_any(
        &self,
        seq: i32,
        texts: &[String],
    ) -> Result<Vec<KanjiText>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.kanji_texts_by_seq_and_text_any(seq, texts).await)
    }

    async fn kana_texts_by_seq_ordered(&self, seq: i32) -> Result<Vec<KanaText>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.kana_texts_by_seq_ordered(seq).await)
    }

    async fn kanji_text_by_id(&self, id: i32) -> Result<KanjiText, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.kanji_text_by_id(id).await)
    }

    async fn kana_text_by_id(&self, id: i32) -> Result<KanaText, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.kana_text_by_id(id).await)
    }

    async fn kana_texts_by_text(&self, text: &str) -> Result<Vec<KanaText>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.kana_texts_by_text(text).await)
    }

    async fn kanji_texts_by_text(&self, text: &str) -> Result<Vec<KanjiText>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.kanji_texts_by_text(text).await)
    }

    async fn kana_texts_root_by_text(&self, text: &str) -> Result<Vec<KanaText>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.kana_texts_root_by_text(text).await)
    }

    async fn kanji_texts_root_by_text(&self, text: &str) -> Result<Vec<KanjiText>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.kanji_texts_root_by_text(text).await)
    }

    async fn kana_texts_by_text_any(
        &self,
        texts: &[String],
    ) -> Result<Vec<KanaText>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.kana_texts_by_text_any(texts).await)
    }

    async fn kanji_texts_by_text_any(
        &self,
        texts: &[String],
    ) -> Result<Vec<KanjiText>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.kanji_texts_by_text_any(texts).await)
    }

    async fn kanji_texts_by_text_any_and_seq_any(
        &self,
        texts: &[&str],
        seqs: &[i32],
    ) -> Result<Vec<KanjiText>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.kanji_texts_by_text_any_and_seq_any(texts, seqs).await)
    }

    async fn kana_texts_by_text_any_and_seq_any(
        &self,
        texts: &[&str],
        seqs: &[i32],
    ) -> Result<Vec<KanaText>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.kana_texts_by_text_any_and_seq_any(texts, seqs).await)
    }

    async fn kana_texts_by_text_and_seq_any(
        &self,
        text: &str,
        seqs: &[i32],
    ) -> Result<Vec<KanaText>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.kana_texts_by_text_and_seq_any(text, seqs).await)
    }

    async fn kanji_texts_by_text_and_seq_any(
        &self,
        text: &str,
        seqs: &[i32],
    ) -> Result<Vec<KanjiText>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.kanji_texts_by_text_and_seq_any(text, seqs).await)
    }

    async fn kana_texts_conj_of(
        &self,
        seqs: &[i32],
        text: &str,
    ) -> Result<Vec<KanaText>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.kana_texts_conj_of(seqs, text).await)
    }

    async fn kanji_texts_conj_of(
        &self,
        seqs: &[i32],
        text: &str,
    ) -> Result<Vec<KanjiText>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.kanji_texts_conj_of(seqs, text).await)
    }

    async fn kana_texts_with_pos(
        &self,
        text: &str,
        posi: &[String],
    ) -> Result<Vec<KanaText>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.kana_texts_with_pos(text, posi).await)
    }

    async fn kanji_texts_with_pos(
        &self,
        text: &str,
        posi: &[String],
    ) -> Result<Vec<KanjiText>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.kanji_texts_with_pos(text, posi).await)
    }

    async fn kana_forms_rows(&self, seq: i32) -> Result<Vec<KanaText>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.kana_forms_rows(seq).await)
    }

    async fn kana_texts_by_text_and_seq(
        &self,
        text: &str,
        seq: i32,
    ) -> Result<Vec<KanaText>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.kana_texts_by_text_and_seq(text, seq).await)
    }

    async fn kana_seqs_by_text(&self, text: &str) -> Result<Vec<i32>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.kana_seqs_by_text(text).await)
    }

    async fn kanji_seqs_by_text(&self, text: &str) -> Result<Vec<i32>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.kanji_seqs_by_text(text).await)
    }

    async fn kana_reading_texts_by_seq_any(
        &self,
        seqs: &[i32],
    ) -> Result<Vec<String>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.kana_reading_texts_by_seq_any(seqs).await)
    }

    async fn kana_seqs_by_seq_and_text(
        &self,
        seq: i32,
        text: &str,
    ) -> Result<Vec<i32>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.kana_seqs_by_seq_and_text(seq, text).await)
    }

    async fn kana_texts_by_regex(&self, pattern: &str) -> Result<Vec<KanaText>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.kana_texts_by_regex(pattern).await)
    }

    async fn kana_texts_by_seq_any(&self, seqs: &[i32]) -> Result<Vec<KanaText>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.kana_texts_by_seq_any(seqs).await)
    }

    async fn kanji_texts_by_seq_any(&self, seqs: &[i32]) -> Result<Vec<KanjiText>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.kanji_texts_by_seq_any(seqs).await)
    }

    async fn conjs_by_seq(&self, seq: i32) -> Result<Vec<Conjugation>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.conjs_by_seq(seq).await)
    }

    async fn conjs_by_seq_and_ids(
        &self,
        seq: i32,
        ids: &[i32],
    ) -> Result<Vec<Conjugation>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.conjs_by_seq_and_ids(seq, ids).await)
    }

    async fn conjs_by_seq_and_from(
        &self,
        seq: i32,
        from: i32,
    ) -> Result<Vec<Conjugation>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.conjs_by_seq_and_from(seq, from).await)
    }

    async fn conjs_by_seq_via_null(&self, seq: i32) -> Result<Vec<Conjugation>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.conjs_by_seq_via_null(seq).await)
    }

    async fn conj_by_id(&self, id: i32) -> Result<Conjugation, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.conj_by_id(id).await)
    }

    async fn conj_seqs_of_desu(&self, seqs: &[i32]) -> Result<Vec<i32>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.conj_seqs_of_desu(seqs).await)
    }

    async fn conj_seqs_from_any(&self, seqs: &[i32]) -> Result<Vec<i32>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.conj_seqs_from_any(seqs).await)
    }

    async fn no_conj_seqs(&self) -> Result<Vec<i32>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.no_conj_seqs().await)
    }

    async fn conj_props_by_conj_id(&self, conj_id: i32) -> Result<Vec<ConjProp>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.conj_props_by_conj_id(conj_id).await)
    }

    async fn conj_source_readings_by_conj_id(
        &self,
        conj_id: i32,
    ) -> Result<Vec<(String, String)>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.conj_source_readings_by_conj_id(conj_id).await)
    }

    async fn conj_source_readings_by_conj_id_and_texts(
        &self,
        conj_id: i32,
        texts: &[String],
    ) -> Result<Vec<(String, String)>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.conj_source_readings_by_conj_id_and_texts(conj_id, texts).await)
    }

    async fn conj_source_reading_texts(
        &self,
        conj_id: i32,
        source_text: &str,
    ) -> Result<Vec<String>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.conj_source_reading_texts(conj_id, source_text).await)
    }

    async fn parents_kanji(
        &self,
        seq: i32,
        text: &str,
    ) -> Result<Vec<(i32, i32)>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.parents_kanji(seq, text).await)
    }

    async fn parents_kana(
        &self,
        seq: i32,
        text: &str,
    ) -> Result<Vec<(i32, i32)>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.parents_kana(seq, text).await)
    }

    async fn sense_gloss_rows(
        &self,
        seq: i32,
    ) -> Result<Vec<(i32, Option<String>)>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.sense_gloss_rows(seq).await)
    }

    async fn sense_prop_rows_tagged(
        &self,
        seq: i32,
        tags: &[&str],
    ) -> Result<Vec<(i32, String, String)>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.sense_prop_rows_tagged(seq, tags).await)
    }

    async fn first_sense_gloss(&self, seq: i32) -> Result<Option<Option<String>>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.first_sense_gloss(seq).await)
    }

    async fn first_sense_gloss_with_pos(
        &self,
        seq: i32,
        pos: &str,
    ) -> Result<Option<Option<String>>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.first_sense_gloss_with_pos(seq, pos).await)
    }

    async fn glosses_by_seq_any(
        &self,
        seqs: &[i32],
    ) -> Result<Vec<(i32, String)>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.glosses_by_seq_any(seqs).await)
    }

    async fn uk_sense_ids(&self, seqs: &[i32]) -> Result<Vec<i32>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.uk_sense_ids(seqs).await)
    }

    async fn sense_id_ord0(&self, ids: &[i32]) -> Result<Option<i32>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.sense_id_ord0(ids).await)
    }

    async fn non_arch_posi(&self, seqs: &[i32]) -> Result<Vec<String>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.non_arch_posi(seqs).await)
    }

    async fn arch_only_seqs(&self) -> Result<Vec<i32>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.arch_only_seqs().await)
    }

    async fn counter_seqs(&self) -> Result<Vec<i32>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.counter_seqs().await)
    }

    async fn counter_stag_rows(
        &self,
        tag: &str,
        seqs: &[i32],
    ) -> Result<Vec<(i32, String)>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.counter_stag_rows(tag, seqs).await)
    }

    async fn restricted_readings_by_seq(
        &self,
        seq: i32,
    ) -> Result<Vec<(String, String)>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.restricted_readings_by_seq(seq).await)
    }

    async fn readings_non_nanori_by_kanji_id(
        &self,
        kanji_id: i32,
    ) -> Result<Vec<Reading>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.readings_non_nanori_by_kanji_id(kanji_id).await)
    }

    async fn meanings_by_kanji_id(&self, kanji_id: i32) -> Result<Vec<Meaning>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.meanings_by_kanji_id(kanji_id).await)
    }

    async fn okurigana_texts_by_reading_id(
        &self,
        reading_id: i32,
    ) -> Result<Vec<String>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.okurigana_texts_by_reading_id(reading_id).await)
    }

    async fn kanji_by_text(&self, text: &str) -> Result<Vec<Kanji>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.kanji_by_text(text).await)
    }

    async fn reading_stats_rows(
        &self,
        kanji: &str,
        reading: &str,
        reading_type: &str,
    ) -> Result<Vec<(i32, i32, Option<i32>)>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.reading_stats_rows(kanji, reading, reading_type).await)
    }

    async fn kanji_reading_pairs(
        &self,
        text: &str,
        typeset: &[String],
    ) -> Result<Vec<(String, String)>, sqlx::Error> {
        kani_store_delegate!(self, backend => backend.kanji_reading_pairs(text, typeset).await)
    }
}
