//! Rust-only sidecar (feature `rkyv`): the archive schema for the
//! rkyv-backed dictionary store. [`KaniSnapshot`] holds every table the
//! runtime seam ([`KaniPostgresBackend`](crate::conn::kani_postgres_backend::KaniPostgresBackend))
//! reads, each as one row vector captured in physical row order
//! (`ORDER BY ctid`) — the order Postgres returns rows for queries with
//! no `ORDER BY`, which several seam queries and their downstream
//! consumers depend on. Written by the `kaniran-rkyv-dumper` binary in
//! kaniran-cli; read by the in-memory backend.

use crate::dict::dao::{
    ConjProp, ConjSourceReading, Conjugation, Entry, Gloss, RestrictedReadings, Sense, SenseProp,
};
use crate::kanji::dao::{Kanji, Meaning, Okurigana, Reading};

/// Bumped on any change to [`KaniSnapshot`] or the row structs it
/// contains. Readers must refuse an archive whose
/// [`KaniSnapshotMeta::format_version`] differs.
///
/// v2: dropped the unused `Entry.content` field (raw JMdict XML,
/// ~80 MB across the corpus).
pub const KANI_SNAPSHOT_FORMAT_VERSION: u32 = 2;

/// Identity stamp recorded inside every archive, so audits can name
/// the snapshot they ran against.
///
/// ```text
/// KaniSnapshotMeta {
///     format_version: 1,
///     source_db_name: "ichiran_latest",
///     dumped_at_unix_seconds: 1781050000,
/// }
/// ```
#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct KaniSnapshotMeta {
    pub format_version: u32,
    /// Database name component of the source URL (credentials and
    /// host are deliberately not recorded).
    pub source_db_name: String,
    pub dumped_at_unix_seconds: u64,
}

/// One `kanji_text` table row — the persisted columns of
/// [`crate::dict::dao::KanjiText`] without its runtime-only `state`
/// field.
///
/// ```text
/// KaniKanjiTextRow {
///     id: 50963, seq: 1536670, text: "夜空", ord: 0, common: Some(17),
///     common_tags: "[ichi1][news1][nf17]", conjugate_p: true,
///     nokanji: false, best_kana: Some("よぞら"),
/// }
/// ```
#[cfg_attr(feature = "postgres", derive(sqlx::FromRow))]
#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct KaniKanjiTextRow {
    pub id: i32,
    pub seq: i32,
    pub text: String,
    pub ord: i32,
    pub common: Option<i32>,
    pub common_tags: String,
    pub conjugate_p: bool,
    pub nokanji: bool,
    pub best_kana: Option<String>,
}

/// One `kana_text` table row — the persisted columns of
/// [`crate::dict::dao::KanaText`] without its runtime-only `state`
/// field.
///
/// ```text
/// KaniKanaTextRow {
///     id: 6954, seq: 1047070, text: "グリーンランド", ord: 0,
///     common: Some(0), common_tags: "[gai1]", conjugate_p: true,
///     nokanji: false, best_kanji: Some("臥児狼徳"),
/// }
/// ```
#[cfg_attr(feature = "postgres", derive(sqlx::FromRow))]
#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct KaniKanaTextRow {
    pub id: i32,
    pub seq: i32,
    pub text: String,
    pub ord: i32,
    pub common: Option<i32>,
    pub common_tags: String,
    pub conjugate_p: bool,
    pub nokanji: bool,
    pub best_kanji: Option<String>,
}

/// The whole runtime dictionary as one serializable value. Every row
/// vector is in the source table's physical (ctid) order; a row's
/// index within its vector is its tie-break ordinal for reproducing
/// Postgres result orderings.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct KaniSnapshot {
    pub meta: KaniSnapshotMeta,
    pub entries: Vec<Entry>,
    pub kanji_texts: Vec<KaniKanjiTextRow>,
    pub kana_texts: Vec<KaniKanaTextRow>,
    pub conjugations: Vec<Conjugation>,
    pub conj_props: Vec<ConjProp>,
    pub conj_source_readings: Vec<ConjSourceReading>,
    pub senses: Vec<Sense>,
    pub glosses: Vec<Gloss>,
    pub sense_props: Vec<SenseProp>,
    pub restricted_readings: Vec<RestrictedReadings>,
    /// Kanjidic character records (table `kanji` — single characters,
    /// distinct from the JMdict word writings in `kanji_texts`).
    pub kanji: Vec<Kanji>,
    /// Kanjidic per-character readings (table `reading`).
    pub readings: Vec<Reading>,
    /// Kanjidic okurigana fragments (table `okurigana`).
    pub okurigana: Vec<Okurigana>,
    /// Kanjidic English meanings (table `meaning`).
    pub meanings: Vec<Meaning>,
}
