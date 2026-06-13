//! Rust-only sidecar (feature `rkyv`): the in-memory implementation of
//! [`KaniBackend`], serving every lookup from a memory-mapped
//! [`KaniSnapshot`](crate::conn::kani_snapshot::KaniSnapshot) archive
//! written by `kaniran-rkyv-dumper`.
//!
//! Faithfulness to Postgres rests on row ORDER. The archive stores
//! every table in physical (ctid) order and each index map records row
//! ordinals ascending, so:
//! - single-key lookups return rows in the order a Postgres index scan
//!   yields them (duplicate btree keys are stored in heap order);
//! - `= ANY(…)` lookups merge per-key ordinal lists and sort,
//!   matching a bitmap heap scan's global physical order;
//! - explicit `ORDER BY` clauses are reproduced with a stable sort
//!   over the physically-ordered base, so equal keys keep heap order.
//!
//! Join output order and `DISTINCT`/`UNION` order are plan-dependent
//! in Postgres and cannot be replicated exactly: joins here iterate
//! the driving table in physical order, and `DISTINCT`/`UNION` dedupe
//! to first occurrence. The 252k audit measures whatever divergence
//! remains.

use crate::conn::kani_backend::KaniBackend;
use crate::conn::kani_snapshot::{
    ArchivedKaniKanaTextRow, ArchivedKaniKanjiTextRow, ArchivedKaniSnapshot,
    KANI_SNAPSHOT_FORMAT_VERSION,
};
use crate::dict::dao::{ConjProp, Conjugation, Entry, KanaText, KanjiText, SimpleText};
use crate::kanji::dao::{Kanji, Meaning, Reading};
use rkyv::hash::FxHasher64;
use std::collections::{HashMap, HashSet};
use std::hash::BuildHasherDefault;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

/// Std HashMap keyed by `i32` with rkyv's cross-platform FxHasher.
/// FxHash beats std's SipHash by ~2-3× on integer keys at the cost of
/// DoS resistance (irrelevant for an in-process JMdict index).
type FxIntMap<K, V> = HashMap<K, V, BuildHasherDefault<FxHasher64>>;

/// Direct-indexed lookup over Postgres-serial primary keys: `vec[id]`
/// is the row ordinal, or [`Self::SENTINEL`] when there is no row.
/// One cache miss, zero hashing — strictly faster than [`FxIntMap`]
/// for the 5 ID-keyed maps whose keys are dense serials.
#[derive(Default)]
struct DenseIdIndex(Vec<u32>);

impl DenseIdIndex {
    const SENTINEL: u32 = u32::MAX;

    fn insert(&mut self, id: i32, ordinal: u32) {
        debug_assert!(id >= 0, "Postgres serial ids are non-negative");
        let slot = id as usize;
        if slot >= self.0.len() {
            self.0.resize(slot + 1, Self::SENTINEL);
        }
        self.0[slot] = ordinal;
    }

    #[inline]
    fn get(&self, id: i32) -> Option<u32> {
        if id < 0 {
            return None;
        }
        self.0
            .get(id as usize)
            .copied()
            .filter(|ordinal| *ordinal != Self::SENTINEL)
    }
}

/// Memory-mapped snapshot store. Cheap to clone — wraps one `Arc`.
#[derive(Clone)]
pub struct KaniRkyvBackend {
    inner: Arc<KaniRkyvInner>,
}

struct KaniRkyvInner {
    mmap: memmap2::Mmap,
    indexes: KaniRkyvIndexes,
}

/// Index maps from key to row ordinal(s) in the archived table
/// vectors. Every ordinal vector is ascending = physical row order.
///
/// Integer-keyed maps use [`FxIntMap`] (FxHash) instead of std SipHash.
/// The 5 maps keyed by Postgres-serial primary keys (`*_by_id`) use
/// [`DenseIdIndex`] — direct `vec[id]` indexing with no hash at all.
/// `_by_text` maps stay on std HashMap; string keys are not on the hot
/// integer-lookup path.
#[derive(Default)]
struct KaniRkyvIndexes {
    entry_by_seq: FxIntMap<i32, u32>,
    kanji_text_by_id: DenseIdIndex,
    kanji_text_by_seq: FxIntMap<i32, Vec<u32>>,
    kanji_text_by_text: HashMap<String, Vec<u32>>,
    kana_text_by_id: DenseIdIndex,
    kana_text_by_seq: FxIntMap<i32, Vec<u32>>,
    kana_text_by_text: HashMap<String, Vec<u32>>,
    conj_by_id: DenseIdIndex,
    conj_by_seq: FxIntMap<i32, Vec<u32>>,
    conj_by_from: FxIntMap<i32, Vec<u32>>,
    conj_prop_by_conj_id: FxIntMap<i32, Vec<u32>>,
    csr_by_conj_id: FxIntMap<i32, Vec<u32>>,
    sense_by_id: DenseIdIndex,
    sense_by_seq: FxIntMap<i32, Vec<u32>>,
    gloss_by_sense_id: FxIntMap<i32, Vec<u32>>,
    sense_prop_by_seq: FxIntMap<i32, Vec<u32>>,
    sense_prop_by_sense_id: FxIntMap<i32, Vec<u32>>,
    restricted_by_seq: FxIntMap<i32, Vec<u32>>,
    kanji_by_id: DenseIdIndex,
    kanji_by_text: HashMap<String, Vec<u32>>,
    reading_by_kanji_id: FxIntMap<i32, Vec<u32>>,
    okurigana_by_reading_id: FxIntMap<i32, Vec<u32>>,
    meaning_by_kanji_id: FxIntMap<i32, Vec<u32>>,
}

fn build_indexes(tables: &ArchivedKaniSnapshot) -> KaniRkyvIndexes {
    let mut indexes = KaniRkyvIndexes::default();
    for (ordinal, row) in tables.entries.iter().enumerate() {
        indexes.entry_by_seq.insert(row.seq.to_native(), ordinal as u32);
    }
    for (ordinal, row) in tables.kanji_texts.iter().enumerate() {
        let ordinal = ordinal as u32;
        indexes.kanji_text_by_id.insert(row.id.to_native(), ordinal);
        indexes
            .kanji_text_by_seq
            .entry(row.seq.to_native())
            .or_default()
            .push(ordinal);
        indexes
            .kanji_text_by_text
            .entry(row.text.as_str().to_string())
            .or_default()
            .push(ordinal);
    }
    for (ordinal, row) in tables.kana_texts.iter().enumerate() {
        let ordinal = ordinal as u32;
        indexes.kana_text_by_id.insert(row.id.to_native(), ordinal);
        indexes
            .kana_text_by_seq
            .entry(row.seq.to_native())
            .or_default()
            .push(ordinal);
        indexes
            .kana_text_by_text
            .entry(row.text.as_str().to_string())
            .or_default()
            .push(ordinal);
    }
    for (ordinal, row) in tables.conjugations.iter().enumerate() {
        let ordinal = ordinal as u32;
        indexes.conj_by_id.insert(row.id.to_native(), ordinal);
        indexes
            .conj_by_seq
            .entry(row.seq.to_native())
            .or_default()
            .push(ordinal);
        indexes
            .conj_by_from
            .entry(row.seq_from.to_native())
            .or_default()
            .push(ordinal);
    }
    for (ordinal, row) in tables.conj_props.iter().enumerate() {
        indexes
            .conj_prop_by_conj_id
            .entry(row.conj_id.to_native())
            .or_default()
            .push(ordinal as u32);
    }
    for (ordinal, row) in tables.conj_source_readings.iter().enumerate() {
        indexes
            .csr_by_conj_id
            .entry(row.conj_id.to_native())
            .or_default()
            .push(ordinal as u32);
    }
    for (ordinal, row) in tables.senses.iter().enumerate() {
        let ordinal = ordinal as u32;
        indexes.sense_by_id.insert(row.id.to_native(), ordinal);
        indexes
            .sense_by_seq
            .entry(row.seq.to_native())
            .or_default()
            .push(ordinal);
    }
    for (ordinal, row) in tables.glosses.iter().enumerate() {
        indexes
            .gloss_by_sense_id
            .entry(row.sense_id.to_native())
            .or_default()
            .push(ordinal as u32);
    }
    for (ordinal, row) in tables.sense_props.iter().enumerate() {
        let ordinal = ordinal as u32;
        indexes
            .sense_prop_by_seq
            .entry(row.seq.to_native())
            .or_default()
            .push(ordinal);
        indexes
            .sense_prop_by_sense_id
            .entry(row.sense_id.to_native())
            .or_default()
            .push(ordinal);
    }
    for (ordinal, row) in tables.restricted_readings.iter().enumerate() {
        indexes
            .restricted_by_seq
            .entry(row.seq.to_native())
            .or_default()
            .push(ordinal as u32);
    }
    for (ordinal, row) in tables.kanji.iter().enumerate() {
        let ordinal = ordinal as u32;
        indexes.kanji_by_id.insert(row.id.to_native(), ordinal);
        indexes
            .kanji_by_text
            .entry(row.text.as_str().to_string())
            .or_default()
            .push(ordinal);
    }
    for (ordinal, row) in tables.readings.iter().enumerate() {
        indexes
            .reading_by_kanji_id
            .entry(row.kanji_id.to_native())
            .or_default()
            .push(ordinal as u32);
    }
    for (ordinal, row) in tables.okurigana.iter().enumerate() {
        indexes
            .okurigana_by_reading_id
            .entry(row.reading_id.to_native())
            .or_default()
            .push(ordinal as u32);
    }
    for (ordinal, row) in tables.meanings.iter().enumerate() {
        indexes
            .meaning_by_kanji_id
            .entry(row.kanji_id.to_native())
            .or_default()
            .push(ordinal as u32);
    }
    indexes
}

impl KaniRkyvBackend {
    /// mmap `path`, validate the archive once (bytecheck over the
    /// whole buffer), refuse a format-version mismatch, and build the
    /// index maps. Errors are strings — the caller wraps them.
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let load_started = Instant::now();
        let file = std::fs::File::open(path)
            .map_err(|open_error| format!("open {}: {open_error}", path.display()))?;
        // mmap safety contract: the file must not be truncated or
        // rewritten while mapped. Snapshot archives are write-once
        // artifacts of the dumper.
        let mmap = unsafe { memmap2::Mmap::map(&file) }
            .map_err(|map_error| format!("mmap {}: {map_error}", path.display()))?;
        let indexes = {
            let tables = rkyv::access::<ArchivedKaniSnapshot, rkyv::rancor::Error>(&mmap[..])
                .map_err(|validate_error| {
                    format!("validate {}: {validate_error}", path.display())
                })?;
            let format_version = tables.meta.format_version.to_native();
            if format_version != KANI_SNAPSHOT_FORMAT_VERSION {
                return Err(format!(
                    "format version mismatch: archive v{format_version}, \
                     this build reads v{KANI_SNAPSHOT_FORMAT_VERSION}",
                ));
            }
            let validated_at = load_started.elapsed().as_secs_f64();
            let index_started = Instant::now();
            let indexes = build_indexes(tables);
            eprintln!(
                "kaniran: rkyv snapshot {} ({} bytes, source {}): \
                 validated in {validated_at:.1}s, indexed in {:.1}s",
                path.display(),
                mmap.len(),
                tables.meta.source_db_name.as_str(),
                index_started.elapsed().as_secs_f64(),
            );
            indexes
        };
        Ok(Self {
            inner: Arc::new(KaniRkyvInner { mmap, indexes }),
        })
    }

    /// The archived root. The buffer was validated once in
    /// [`Self::from_file`]; unchecked access afterwards is sound
    /// because the mmap is read-only and never remapped.
    fn tables(&self) -> &ArchivedKaniSnapshot {
        unsafe { rkyv::access_unchecked::<ArchivedKaniSnapshot>(&self.inner.mmap[..]) }
    }

    fn idx(&self) -> &KaniRkyvIndexes {
        &self.inner.indexes
    }
}

// --- archived-row → DAO conversions ---

// Runtime never reads the `common_tags` columns on kanji_text /
// kana_text — those are only consumed by build-time errata writers
// that load via the Postgres backend. Leaving them as empty
// `String::new()` (a stack-only 24-byte placeholder with capacity 0)
// skips per-call heap allocations at zero runtime risk.
fn dao_entry(row: &<Entry as rkyv::Archive>::Archived) -> Entry {
    Entry {
        seq: row.seq.to_native(),
        root_p: row.root_p,
        n_kanji: row.n_kanji.to_native(),
        n_kana: row.n_kana.to_native(),
        primary_nokanji: row.primary_nokanji,
    }
}

fn dao_kanji_text(row: &ArchivedKaniKanjiTextRow) -> KanjiText {
    KanjiText {
        id: row.id.to_native(),
        seq: row.seq.to_native(),
        text: row.text.as_str().to_string(),
        ord: row.ord.to_native(),
        common: row.common.as_ref().map(|value| value.to_native()),
        common_tags: String::new(),
        conjugate_p: row.conjugate_p,
        nokanji: row.nokanji,
        best_kana: row.best_kana.as_ref().map(|value| value.as_str().to_string()),
        state: SimpleText::default(),
    }
}

fn dao_kana_text(row: &ArchivedKaniKanaTextRow) -> KanaText {
    KanaText {
        id: row.id.to_native(),
        seq: row.seq.to_native(),
        text: row.text.as_str().to_string(),
        ord: row.ord.to_native(),
        common: row.common.as_ref().map(|value| value.to_native()),
        common_tags: String::new(),
        conjugate_p: row.conjugate_p,
        nokanji: row.nokanji,
        best_kanji: row.best_kanji.as_ref().map(|value| value.as_str().to_string()),
        state: SimpleText::default(),
    }
}

fn dao_conjugation(row: &<Conjugation as rkyv::Archive>::Archived) -> Conjugation {
    Conjugation {
        id: row.id.to_native(),
        seq: row.seq.to_native(),
        seq_from: row.seq_from.to_native(),
        seq_via: row.seq_via.as_ref().map(|value| value.to_native()),
    }
}

fn dao_conj_prop(row: &<ConjProp as rkyv::Archive>::Archived) -> ConjProp {
    ConjProp {
        id: row.id.to_native(),
        conj_id: row.conj_id.to_native(),
        conj_type: row.conj_type.to_native(),
        pos: row.pos.as_str().to_string(),
        neg: row.neg.as_ref().map(|flag| *flag),
        fml: row.fml.as_ref().map(|flag| *flag),
    }
}

fn dao_kanji(row: &<Kanji as rkyv::Archive>::Archived) -> Kanji {
    Kanji {
        id: row.id.to_native(),
        text: row.text.as_str().to_string(),
        radical_c: row.radical_c.to_native(),
        radical_n: row.radical_n.to_native(),
        grade: row.grade.as_ref().map(|value| value.to_native()),
        strokes: row.strokes.to_native(),
        freq: row.freq.as_ref().map(|value| value.to_native()),
        stat_common: row.stat_common.to_native(),
        stat_irregular: row.stat_irregular.to_native(),
    }
}

fn dao_reading(row: &<Reading as rkyv::Archive>::Archived) -> Reading {
    Reading {
        id: row.id.to_native(),
        kanji_id: row.kanji_id.to_native(),
        reading_type: row.reading_type.as_str().to_string(),
        text: row.text.as_str().to_string(),
        suffixp: row.suffixp,
        prefixp: row.prefixp,
        stat_common: row.stat_common.to_native(),
    }
}

fn dao_meaning(row: &<Meaning as rkyv::Archive>::Archived) -> Meaning {
    Meaning {
        id: row.id.to_native(),
        kanji_id: row.kanji_id.to_native(),
        text: row.text.as_str().to_string(),
    }
}

// --- ordinal helpers ---

/// Merge per-key ordinal lists into one ascending (physical-order)
/// list. Sort + dedup also absorbs duplicate input keys, matching
/// `= ANY(array)` predicate semantics (a row matches once).
fn merged_i32(map: &FxIntMap<i32, Vec<u32>>, keys: &[i32]) -> Vec<u32> {
    let mut ordinals: Vec<u32> = Vec::new();
    for key in keys {
        if let Some(list) = map.get(key) {
            ordinals.extend_from_slice(list);
        }
    }
    ordinals.sort_unstable();
    ordinals.dedup();
    ordinals
}

fn merged_str<S: AsRef<str>>(map: &HashMap<String, Vec<u32>>, keys: &[S]) -> Vec<u32> {
    let mut ordinals: Vec<u32> = Vec::new();
    for key in keys {
        if let Some(list) = map.get(key.as_ref()) {
            ordinals.extend_from_slice(list);
        }
    }
    ordinals.sort_unstable();
    ordinals.dedup();
    ordinals
}

impl KaniBackend for KaniRkyvBackend {
    // --- entry ---

    async fn entry_by_seq(&self, seq: i32) -> Result<Option<Entry>, sqlx::Error> {
        let tables = self.tables();
        Ok(self
            .idx()
            .entry_by_seq
            .get(&seq)
            .map(|&ordinal| dao_entry(&tables.entries[ordinal as usize])))
    }

    async fn root_seqs(&self, seqs: &[i32]) -> Result<Vec<i32>, sqlx::Error> {
        let tables = self.tables();
        let mut ordinals: Vec<u32> = seqs
            .iter()
            .filter_map(|seq| self.idx().entry_by_seq.get(seq).copied())
            .collect();
        ordinals.sort_unstable();
        ordinals.dedup();
        Ok(ordinals
            .into_iter()
            .map(|ordinal| &tables.entries[ordinal as usize])
            .filter(|row| row.root_p)
            .map(|row| row.seq.to_native())
            .collect())
    }

    async fn candidate_seqs_kana(&self, text: &str) -> Result<Vec<i32>, sqlx::Error> {
        let tables = self.tables();
        let indexes = self.idx();
        let mut seqs_out: Vec<i32> = Vec::new();
        if let Some(list) = indexes.kana_text_by_text.get(text) {
            for &ordinal in list {
                let row = &tables.kana_texts[ordinal as usize];
                if row.ord.to_native() != 0 {
                    continue;
                }
                let seq = row.seq.to_native();
                let Some(&entry_ordinal) = indexes.entry_by_seq.get(&seq) else {
                    continue;
                };
                if !tables.entries[entry_ordinal as usize].root_p {
                    continue;
                }
                // `k.text IS NULL` through the LEFT JOIN = the entry
                // has no kanji_text rows at all.
                if indexes.kanji_text_by_seq.contains_key(&seq) {
                    continue;
                }
                seqs_out.push(seq);
            }
        }
        seqs_out.sort_unstable();
        Ok(seqs_out)
    }

    async fn candidate_seqs_kanji(
        &self,
        text: &str,
        reading: Option<&str>,
    ) -> Result<Vec<i32>, sqlx::Error> {
        // `r.text = NULL` is never true in SQL, so a missing reading
        // matches nothing.
        let Some(reading) = reading else {
            return Ok(Vec::new());
        };
        let tables = self.tables();
        let indexes = self.idx();
        let mut seqs_out: Vec<i32> = Vec::new();
        if let Some(list) = indexes.kanji_text_by_text.get(text) {
            for &ordinal in list {
                let row = &tables.kanji_texts[ordinal as usize];
                if row.ord.to_native() != 0 {
                    continue;
                }
                let seq = row.seq.to_native();
                let has_reading = indexes
                    .kana_text_by_seq
                    .get(&seq)
                    .is_some_and(|kana_list| {
                        kana_list.iter().any(|&kana_ordinal| {
                            let kana_row = &tables.kana_texts[kana_ordinal as usize];
                            kana_row.ord.to_native() == 0 && kana_row.text.as_str() == reading
                        })
                    });
                if has_reading {
                    seqs_out.push(seq);
                }
            }
        }
        seqs_out.sort_unstable();
        Ok(seqs_out)
    }

    async fn kanji_words_containing_char(
        &self,
        char: &str,
    ) -> Result<Vec<(i32, String, String, i32)>, sqlx::Error> {
        let tables = self.tables();
        let indexes = self.idx();
        let mut out: Vec<(i32, String, String, i32)> = Vec::new();
        for row in tables.kanji_texts.iter() {
            let Some(common) = row.common.as_ref() else {
                continue;
            };
            if !row.text.as_str().contains(char) {
                continue;
            }
            let Some(best_kana) = row.best_kana.as_ref() else {
                continue;
            };
            let seq = row.seq.to_native();
            let Some(&entry_ordinal) = indexes.entry_by_seq.get(&seq) else {
                continue;
            };
            if !tables.entries[entry_ordinal as usize].root_p {
                continue;
            }
            if let Some(kana_list) = indexes.kana_text_by_seq.get(&seq) {
                for &kana_ordinal in kana_list {
                    let kana_row = &tables.kana_texts[kana_ordinal as usize];
                    if kana_row.text.as_str() == best_kana.as_str() {
                        out.push((
                            seq,
                            row.text.as_str().to_string(),
                            kana_row.text.as_str().to_string(),
                            common.to_native(),
                        ));
                    }
                }
            }
        }
        Ok(out)
    }

    // --- kanji_text / kana_text ---

    async fn headword_kanji_text(&self, seq: i32) -> Result<Option<String>, sqlx::Error> {
        let tables = self.tables();
        Ok(self.idx().kanji_text_by_seq.get(&seq).and_then(|list| {
            list.iter()
                .map(|&ordinal| &tables.kanji_texts[ordinal as usize])
                .find(|row| row.ord.to_native() == 0)
                .map(|row| row.text.as_str().to_string())
        }))
    }

    async fn headword_kana_text(&self, seq: i32) -> Result<Option<String>, sqlx::Error> {
        let tables = self.tables();
        Ok(self.idx().kana_text_by_seq.get(&seq).and_then(|list| {
            list.iter()
                .map(|&ordinal| &tables.kana_texts[ordinal as usize])
                .find(|row| row.ord.to_native() == 0)
                .map(|row| row.text.as_str().to_string())
        }))
    }

    async fn kanji_texts_by_seq_and_text(
        &self,
        seq: i32,
        text: &str,
    ) -> Result<Vec<KanjiText>, sqlx::Error> {
        let tables = self.tables();
        Ok(self
            .idx()
            .kanji_text_by_seq
            .get(&seq)
            .map(|list| {
                list.iter()
                    .map(|&ordinal| &tables.kanji_texts[ordinal as usize])
                    .filter(|row| row.text.as_str() == text)
                    .map(dao_kanji_text)
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn kana_texts_by_seq_and_text(
        &self,
        seq: i32,
        text: &str,
    ) -> Result<Vec<KanaText>, sqlx::Error> {
        let tables = self.tables();
        Ok(self
            .idx()
            .kana_text_by_seq
            .get(&seq)
            .map(|list| {
                list.iter()
                    .map(|&ordinal| &tables.kana_texts[ordinal as usize])
                    .filter(|row| row.text.as_str() == text)
                    .map(dao_kana_text)
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn kana_texts_by_seq_and_text_any(
        &self,
        seq: i32,
        texts: &[String],
    ) -> Result<Vec<KanaText>, sqlx::Error> {
        let tables = self.tables();
        Ok(self
            .idx()
            .kana_text_by_seq
            .get(&seq)
            .map(|list| {
                list.iter()
                    .map(|&ordinal| &tables.kana_texts[ordinal as usize])
                    .filter(|row| texts.iter().any(|text| text == row.text.as_str()))
                    .map(dao_kana_text)
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn kanji_texts_by_seq_and_text_any(
        &self,
        seq: i32,
        texts: &[String],
    ) -> Result<Vec<KanjiText>, sqlx::Error> {
        let tables = self.tables();
        Ok(self
            .idx()
            .kanji_text_by_seq
            .get(&seq)
            .map(|list| {
                list.iter()
                    .map(|&ordinal| &tables.kanji_texts[ordinal as usize])
                    .filter(|row| texts.iter().any(|text| text == row.text.as_str()))
                    .map(dao_kanji_text)
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn kana_texts_by_seq_ordered(&self, seq: i32) -> Result<Vec<KanaText>, sqlx::Error> {
        let tables = self.tables();
        let mut rows: Vec<KanaText> = self
            .idx()
            .kana_text_by_seq
            .get(&seq)
            .map(|list| {
                list.iter()
                    .map(|&ordinal| dao_kana_text(&tables.kana_texts[ordinal as usize]))
                    .collect()
            })
            .unwrap_or_default();
        rows.sort_by_key(|row| row.ord);
        Ok(rows)
    }

    async fn kanji_text_by_id(&self, id: i32) -> Result<KanjiText, sqlx::Error> {
        let tables = self.tables();
        self.idx()
            .kanji_text_by_id
            .get(id)
            .map(|ordinal| dao_kanji_text(&tables.kanji_texts[ordinal as usize]))
            .ok_or(sqlx::Error::RowNotFound)
    }

    async fn kana_text_by_id(&self, id: i32) -> Result<KanaText, sqlx::Error> {
        let tables = self.tables();
        self.idx()
            .kana_text_by_id
            .get(id)
            .map(|ordinal| dao_kana_text(&tables.kana_texts[ordinal as usize]))
            .ok_or(sqlx::Error::RowNotFound)
    }

    async fn kana_texts_by_text(&self, text: &str) -> Result<Vec<KanaText>, sqlx::Error> {
        let tables = self.tables();
        Ok(self
            .idx()
            .kana_text_by_text
            .get(text)
            .map(|list| {
                list.iter()
                    .map(|&ordinal| dao_kana_text(&tables.kana_texts[ordinal as usize]))
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn kanji_texts_by_text(&self, text: &str) -> Result<Vec<KanjiText>, sqlx::Error> {
        let tables = self.tables();
        Ok(self
            .idx()
            .kanji_text_by_text
            .get(text)
            .map(|list| {
                list.iter()
                    .map(|&ordinal| dao_kanji_text(&tables.kanji_texts[ordinal as usize]))
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn kana_texts_root_by_text(&self, text: &str) -> Result<Vec<KanaText>, sqlx::Error> {
        let tables = self.tables();
        let indexes = self.idx();
        Ok(indexes
            .kana_text_by_text
            .get(text)
            .map(|list| {
                list.iter()
                    .map(|&ordinal| &tables.kana_texts[ordinal as usize])
                    .filter(|row| {
                        indexes
                            .entry_by_seq
                            .get(&row.seq.to_native())
                            .is_some_and(|&entry_ordinal| {
                                tables.entries[entry_ordinal as usize].root_p
                            })
                    })
                    .map(dao_kana_text)
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn kanji_texts_root_by_text(&self, text: &str) -> Result<Vec<KanjiText>, sqlx::Error> {
        let tables = self.tables();
        let indexes = self.idx();
        Ok(indexes
            .kanji_text_by_text
            .get(text)
            .map(|list| {
                list.iter()
                    .map(|&ordinal| &tables.kanji_texts[ordinal as usize])
                    .filter(|row| {
                        indexes
                            .entry_by_seq
                            .get(&row.seq.to_native())
                            .is_some_and(|&entry_ordinal| {
                                tables.entries[entry_ordinal as usize].root_p
                            })
                    })
                    .map(dao_kanji_text)
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn kana_texts_by_text_any(
        &self,
        texts: &[String],
    ) -> Result<Vec<KanaText>, sqlx::Error> {
        let tables = self.tables();
        Ok(merged_str(&self.idx().kana_text_by_text, texts)
            .into_iter()
            .map(|ordinal| dao_kana_text(&tables.kana_texts[ordinal as usize]))
            .collect())
    }

    async fn kanji_texts_by_text_any(
        &self,
        texts: &[String],
    ) -> Result<Vec<KanjiText>, sqlx::Error> {
        let tables = self.tables();
        Ok(merged_str(&self.idx().kanji_text_by_text, texts)
            .into_iter()
            .map(|ordinal| dao_kanji_text(&tables.kanji_texts[ordinal as usize]))
            .collect())
    }

    async fn kanji_texts_by_text_any_and_seq_any(
        &self,
        texts: &[&str],
        seqs: &[i32],
    ) -> Result<Vec<KanjiText>, sqlx::Error> {
        let tables = self.tables();
        let seq_set: HashSet<i32> = seqs.iter().copied().collect();
        Ok(merged_str(&self.idx().kanji_text_by_text, texts)
            .into_iter()
            .map(|ordinal| &tables.kanji_texts[ordinal as usize])
            .filter(|row| seq_set.contains(&row.seq.to_native()))
            .map(dao_kanji_text)
            .collect())
    }

    async fn kana_texts_by_text_any_and_seq_any(
        &self,
        texts: &[&str],
        seqs: &[i32],
    ) -> Result<Vec<KanaText>, sqlx::Error> {
        let tables = self.tables();
        let seq_set: HashSet<i32> = seqs.iter().copied().collect();
        Ok(merged_str(&self.idx().kana_text_by_text, texts)
            .into_iter()
            .map(|ordinal| &tables.kana_texts[ordinal as usize])
            .filter(|row| seq_set.contains(&row.seq.to_native()))
            .map(dao_kana_text)
            .collect())
    }

    async fn kana_texts_by_text_and_seq_any(
        &self,
        text: &str,
        seqs: &[i32],
    ) -> Result<Vec<KanaText>, sqlx::Error> {
        let tables = self.tables();
        let seq_set: HashSet<i32> = seqs.iter().copied().collect();
        Ok(self
            .idx()
            .kana_text_by_text
            .get(text)
            .map(|list| {
                list.iter()
                    .map(|&ordinal| &tables.kana_texts[ordinal as usize])
                    .filter(|row| seq_set.contains(&row.seq.to_native()))
                    .map(dao_kana_text)
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn kanji_texts_by_text_and_seq_any(
        &self,
        text: &str,
        seqs: &[i32],
    ) -> Result<Vec<KanjiText>, sqlx::Error> {
        let tables = self.tables();
        let seq_set: HashSet<i32> = seqs.iter().copied().collect();
        Ok(self
            .idx()
            .kanji_text_by_text
            .get(text)
            .map(|list| {
                list.iter()
                    .map(|&ordinal| &tables.kanji_texts[ordinal as usize])
                    .filter(|row| seq_set.contains(&row.seq.to_native()))
                    .map(dao_kanji_text)
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn kana_texts_conj_of(
        &self,
        seqs: &[i32],
        text: &str,
    ) -> Result<Vec<KanaText>, sqlx::Error> {
        let tables = self.tables();
        let indexes = self.idx();
        let from_set: HashSet<i32> = seqs.iter().copied().collect();
        let mut out: Vec<KanaText> = Vec::new();
        if let Some(list) = indexes.kana_text_by_text.get(text) {
            for &ordinal in list {
                let row = &tables.kana_texts[ordinal as usize];
                if let Some(conj_list) = indexes.conj_by_seq.get(&row.seq.to_native()) {
                    for &conj_ordinal in conj_list {
                        let conj = &tables.conjugations[conj_ordinal as usize];
                        if from_set.contains(&conj.seq_from.to_native()) {
                            // join multiplicity: one output row per
                            // matching conjugation row
                            out.push(dao_kana_text(row));
                        }
                    }
                }
            }
        }
        Ok(out)
    }

    async fn kanji_texts_conj_of(
        &self,
        seqs: &[i32],
        text: &str,
    ) -> Result<Vec<KanjiText>, sqlx::Error> {
        let tables = self.tables();
        let indexes = self.idx();
        let from_set: HashSet<i32> = seqs.iter().copied().collect();
        let mut out: Vec<KanjiText> = Vec::new();
        if let Some(list) = indexes.kanji_text_by_text.get(text) {
            for &ordinal in list {
                let row = &tables.kanji_texts[ordinal as usize];
                if let Some(conj_list) = indexes.conj_by_seq.get(&row.seq.to_native()) {
                    for &conj_ordinal in conj_list {
                        let conj = &tables.conjugations[conj_ordinal as usize];
                        if from_set.contains(&conj.seq_from.to_native()) {
                            out.push(dao_kanji_text(row));
                        }
                    }
                }
            }
        }
        Ok(out)
    }

    async fn kana_texts_with_pos(
        &self,
        text: &str,
        posi: &[String],
    ) -> Result<Vec<KanaText>, sqlx::Error> {
        let tables = self.tables();
        let indexes = self.idx();
        Ok(indexes
            .kana_text_by_text
            .get(text)
            .map(|list| {
                list.iter()
                    .map(|&ordinal| &tables.kana_texts[ordinal as usize])
                    .filter(|row| {
                        // DISTINCT collapses the join multiplicity:
                        // one output row if ANY matching pos prop.
                        indexes
                            .sense_prop_by_seq
                            .get(&row.seq.to_native())
                            .is_some_and(|props| {
                                props.iter().any(|&prop_ordinal| {
                                    let prop = &tables.sense_props[prop_ordinal as usize];
                                    prop.tag.as_str() == "pos"
                                        && posi.iter().any(|pos| pos == prop.text.as_str())
                                })
                            })
                    })
                    .map(dao_kana_text)
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn kanji_texts_with_pos(
        &self,
        text: &str,
        posi: &[String],
    ) -> Result<Vec<KanjiText>, sqlx::Error> {
        let tables = self.tables();
        let indexes = self.idx();
        Ok(indexes
            .kanji_text_by_text
            .get(text)
            .map(|list| {
                list.iter()
                    .map(|&ordinal| &tables.kanji_texts[ordinal as usize])
                    .filter(|row| {
                        indexes
                            .sense_prop_by_seq
                            .get(&row.seq.to_native())
                            .is_some_and(|props| {
                                props.iter().any(|&prop_ordinal| {
                                    let prop = &tables.sense_props[prop_ordinal as usize];
                                    prop.tag.as_str() == "pos"
                                        && posi.iter().any(|pos| pos == prop.text.as_str())
                                })
                            })
                    })
                    .map(dao_kanji_text)
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn kana_forms_rows(&self, seq: i32) -> Result<Vec<KanaText>, sqlx::Error> {
        let tables = self.tables();
        let indexes = self.idx();
        let mut ordinals: Vec<u32> = indexes
            .kana_text_by_seq
            .get(&seq)
            .cloned()
            .unwrap_or_default();
        if let Some(conj_list) = indexes.conj_by_from.get(&seq) {
            for &conj_ordinal in conj_list {
                let derived_seq = tables.conjugations[conj_ordinal as usize].seq.to_native();
                if let Some(list) = indexes.kana_text_by_seq.get(&derived_seq) {
                    ordinals.extend_from_slice(list);
                }
            }
        }
        // UNION dedups; emit in physical order.
        ordinals.sort_unstable();
        ordinals.dedup();
        Ok(ordinals
            .into_iter()
            .map(|ordinal| dao_kana_text(&tables.kana_texts[ordinal as usize]))
            .collect())
    }

    async fn kana_texts_by_text_and_seq(
        &self,
        text: &str,
        seq: i32,
    ) -> Result<Vec<KanaText>, sqlx::Error> {
        let tables = self.tables();
        Ok(self
            .idx()
            .kana_text_by_text
            .get(text)
            .map(|list| {
                list.iter()
                    .map(|&ordinal| &tables.kana_texts[ordinal as usize])
                    .filter(|row| row.seq.to_native() == seq)
                    .map(dao_kana_text)
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn kana_seqs_by_text(&self, text: &str) -> Result<Vec<i32>, sqlx::Error> {
        let tables = self.tables();
        Ok(self
            .idx()
            .kana_text_by_text
            .get(text)
            .map(|list| {
                list.iter()
                    .map(|&ordinal| tables.kana_texts[ordinal as usize].seq.to_native())
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn kanji_seqs_by_text(&self, text: &str) -> Result<Vec<i32>, sqlx::Error> {
        let tables = self.tables();
        Ok(self
            .idx()
            .kanji_text_by_text
            .get(text)
            .map(|list| {
                list.iter()
                    .map(|&ordinal| tables.kanji_texts[ordinal as usize].seq.to_native())
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn kana_reading_texts_by_seq_any(
        &self,
        seqs: &[i32],
    ) -> Result<Vec<String>, sqlx::Error> {
        let tables = self.tables();
        let mut rows: Vec<(i32, String)> = merged_i32(&self.idx().kana_text_by_seq, seqs)
            .into_iter()
            .map(|ordinal| {
                let row = &tables.kana_texts[ordinal as usize];
                (row.id.to_native(), row.text.as_str().to_string())
            })
            .collect();
        rows.sort_by_key(|(id, _)| *id);
        Ok(rows.into_iter().map(|(_, text)| text).collect())
    }

    async fn kana_seqs_by_seq_and_text(
        &self,
        seq: i32,
        text: &str,
    ) -> Result<Vec<i32>, sqlx::Error> {
        let tables = self.tables();
        Ok(self
            .idx()
            .kana_text_by_seq
            .get(&seq)
            .map(|list| {
                list.iter()
                    .map(|&ordinal| &tables.kana_texts[ordinal as usize])
                    .filter(|row| row.text.as_str() == text)
                    .map(|row| row.seq.to_native())
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn kana_texts_by_regex(&self, pattern: &str) -> Result<Vec<KanaText>, sqlx::Error> {
        let tables = self.tables();
        let matcher = fancy_regex::Regex::new(pattern).map_err(|regex_error| {
            sqlx::Error::Protocol(format!("kana_texts_by_regex pattern: {regex_error}"))
        })?;
        let mut out: Vec<KanaText> = Vec::new();
        for row in tables.kana_texts.iter() {
            let is_match = matcher.is_match(row.text.as_str()).map_err(|regex_error| {
                sqlx::Error::Protocol(format!("kana_texts_by_regex match: {regex_error}"))
            })?;
            if is_match {
                out.push(dao_kana_text(row));
            }
        }
        Ok(out)
    }

    async fn kana_texts_by_seq_any(&self, seqs: &[i32]) -> Result<Vec<KanaText>, sqlx::Error> {
        let tables = self.tables();
        Ok(merged_i32(&self.idx().kana_text_by_seq, seqs)
            .into_iter()
            .map(|ordinal| dao_kana_text(&tables.kana_texts[ordinal as usize]))
            .collect())
    }

    async fn kanji_texts_by_seq_any(&self, seqs: &[i32]) -> Result<Vec<KanjiText>, sqlx::Error> {
        let tables = self.tables();
        Ok(merged_i32(&self.idx().kanji_text_by_seq, seqs)
            .into_iter()
            .map(|ordinal| dao_kanji_text(&tables.kanji_texts[ordinal as usize]))
            .collect())
    }

    // --- conjugation ---

    async fn conjs_by_seq(&self, seq: i32) -> Result<Vec<Conjugation>, sqlx::Error> {
        let tables = self.tables();
        Ok(self
            .idx()
            .conj_by_seq
            .get(&seq)
            .map(|list| {
                list.iter()
                    .map(|&ordinal| dao_conjugation(&tables.conjugations[ordinal as usize]))
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn conjs_by_seq_and_ids(
        &self,
        seq: i32,
        ids: &[i32],
    ) -> Result<Vec<Conjugation>, sqlx::Error> {
        let tables = self.tables();
        Ok(self
            .idx()
            .conj_by_seq
            .get(&seq)
            .map(|list| {
                list.iter()
                    .map(|&ordinal| &tables.conjugations[ordinal as usize])
                    .filter(|row| ids.contains(&row.id.to_native()))
                    .map(dao_conjugation)
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn conjs_by_seq_and_from(
        &self,
        seq: i32,
        from: i32,
    ) -> Result<Vec<Conjugation>, sqlx::Error> {
        let tables = self.tables();
        Ok(self
            .idx()
            .conj_by_seq
            .get(&seq)
            .map(|list| {
                list.iter()
                    .map(|&ordinal| &tables.conjugations[ordinal as usize])
                    .filter(|row| row.seq_from.to_native() == from)
                    .map(dao_conjugation)
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn conjs_by_seq_via_null(&self, seq: i32) -> Result<Vec<Conjugation>, sqlx::Error> {
        let tables = self.tables();
        Ok(self
            .idx()
            .conj_by_seq
            .get(&seq)
            .map(|list| {
                list.iter()
                    .map(|&ordinal| &tables.conjugations[ordinal as usize])
                    .filter(|row| row.seq_via.as_ref().is_none())
                    .map(dao_conjugation)
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn conj_by_id(&self, id: i32) -> Result<Conjugation, sqlx::Error> {
        let tables = self.tables();
        self.idx()
            .conj_by_id
            .get(id)
            .map(|ordinal| dao_conjugation(&tables.conjugations[ordinal as usize]))
            .ok_or(sqlx::Error::RowNotFound)
    }

    async fn conj_seqs_of_desu(&self, seqs: &[i32]) -> Result<Vec<i32>, sqlx::Error> {
        let tables = self.tables();
        let seq_set: HashSet<i32> = seqs.iter().copied().collect();
        Ok(self
            .idx()
            .conj_by_from
            .get(&2755350)
            .map(|list| {
                list.iter()
                    .map(|&ordinal| &tables.conjugations[ordinal as usize])
                    .filter(|row| seq_set.contains(&row.seq.to_native()))
                    .map(|row| row.seq.to_native())
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn conj_seqs_from_any(&self, seqs: &[i32]) -> Result<Vec<i32>, sqlx::Error> {
        let tables = self.tables();
        let mut seen: HashSet<i32> = HashSet::new();
        let mut out: Vec<i32> = Vec::new();
        for ordinal in merged_i32(&self.idx().conj_by_from, seqs) {
            let seq = tables.conjugations[ordinal as usize].seq.to_native();
            if seen.insert(seq) {
                out.push(seq);
            }
        }
        Ok(out)
    }

    async fn no_conj_seqs(&self) -> Result<Vec<i32>, sqlx::Error> {
        let tables = self.tables();
        let indexes = self.idx();
        Ok(tables
            .entries
            .iter()
            .map(|row| row.seq.to_native())
            .filter(|seq| !indexes.conj_by_seq.contains_key(seq))
            .collect())
    }

    // --- conj_prop / conj_source_reading ---

    async fn conj_props_by_conj_id(&self, conj_id: i32) -> Result<Vec<ConjProp>, sqlx::Error> {
        let tables = self.tables();
        Ok(self
            .idx()
            .conj_prop_by_conj_id
            .get(&conj_id)
            .map(|list| {
                list.iter()
                    .map(|&ordinal| dao_conj_prop(&tables.conj_props[ordinal as usize]))
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn conj_source_readings_by_conj_id(
        &self,
        conj_id: i32,
    ) -> Result<Vec<(String, String)>, sqlx::Error> {
        let tables = self.tables();
        Ok(self
            .idx()
            .csr_by_conj_id
            .get(&conj_id)
            .map(|list| {
                list.iter()
                    .map(|&ordinal| {
                        let row = &tables.conj_source_readings[ordinal as usize];
                        (
                            row.text.as_str().to_string(),
                            row.source_text.as_str().to_string(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn conj_source_readings_by_conj_id_and_texts(
        &self,
        conj_id: i32,
        texts: &[String],
    ) -> Result<Vec<(String, String)>, sqlx::Error> {
        let tables = self.tables();
        Ok(self
            .idx()
            .csr_by_conj_id
            .get(&conj_id)
            .map(|list| {
                list.iter()
                    .map(|&ordinal| &tables.conj_source_readings[ordinal as usize])
                    .filter(|row| texts.iter().any(|text| text == row.text.as_str()))
                    .map(|row| {
                        (
                            row.text.as_str().to_string(),
                            row.source_text.as_str().to_string(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn conj_source_reading_texts(
        &self,
        conj_id: i32,
        source_text: &str,
    ) -> Result<Vec<String>, sqlx::Error> {
        let tables = self.tables();
        Ok(self
            .idx()
            .csr_by_conj_id
            .get(&conj_id)
            .map(|list| {
                list.iter()
                    .map(|&ordinal| &tables.conj_source_readings[ordinal as usize])
                    .filter(|row| row.source_text.as_str() == source_text)
                    .map(|row| row.text.as_str().to_string())
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn parents_kanji(
        &self,
        seq: i32,
        text: &str,
    ) -> Result<Vec<(i32, i32)>, sqlx::Error> {
        let tables = self.tables();
        let indexes = self.idx();
        let mut out: Vec<(i32, i32)> = Vec::new();
        if let Some(conj_list) = indexes.conj_by_seq.get(&seq) {
            for &conj_ordinal in conj_list {
                let conj = &tables.conjugations[conj_ordinal as usize];
                let conj_id = conj.id.to_native();
                let parent_seq = conj
                    .seq_via
                    .as_ref()
                    .map(|via| via.to_native())
                    .unwrap_or_else(|| conj.seq_from.to_native());
                if let Some(csr_list) = indexes.csr_by_conj_id.get(&conj_id) {
                    for &csr_ordinal in csr_list {
                        let csr = &tables.conj_source_readings[csr_ordinal as usize];
                        if csr.text.as_str() != text {
                            continue;
                        }
                        if let Some(parent_rows) = indexes.kanji_text_by_seq.get(&parent_seq) {
                            for &parent_ordinal in parent_rows {
                                let parent = &tables.kanji_texts[parent_ordinal as usize];
                                if parent.text.as_str() == csr.source_text.as_str() {
                                    out.push((parent.id.to_native(), conj_id));
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(out)
    }

    async fn parents_kana(
        &self,
        seq: i32,
        text: &str,
    ) -> Result<Vec<(i32, i32)>, sqlx::Error> {
        let tables = self.tables();
        let indexes = self.idx();
        let mut out: Vec<(i32, i32)> = Vec::new();
        if let Some(conj_list) = indexes.conj_by_seq.get(&seq) {
            for &conj_ordinal in conj_list {
                let conj = &tables.conjugations[conj_ordinal as usize];
                let conj_id = conj.id.to_native();
                let parent_seq = conj
                    .seq_via
                    .as_ref()
                    .map(|via| via.to_native())
                    .unwrap_or_else(|| conj.seq_from.to_native());
                if let Some(csr_list) = indexes.csr_by_conj_id.get(&conj_id) {
                    for &csr_ordinal in csr_list {
                        let csr = &tables.conj_source_readings[csr_ordinal as usize];
                        if csr.text.as_str() != text {
                            continue;
                        }
                        if let Some(parent_rows) = indexes.kana_text_by_seq.get(&parent_seq) {
                            for &parent_ordinal in parent_rows {
                                let parent = &tables.kana_texts[parent_ordinal as usize];
                                if parent.text.as_str() == csr.source_text.as_str() {
                                    out.push((parent.id.to_native(), conj_id));
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(out)
    }

    // --- sense / gloss / sense_prop / restricted_readings ---

    async fn sense_gloss_rows(
        &self,
        seq: i32,
    ) -> Result<Vec<(i32, Option<String>)>, sqlx::Error> {
        let tables = self.tables();
        let indexes = self.idx();
        let mut senses: Vec<(i32, i32)> = indexes
            .sense_by_seq
            .get(&seq)
            .map(|list| {
                list.iter()
                    .map(|&ordinal| {
                        let row = &tables.senses[ordinal as usize];
                        (row.ord.to_native(), row.id.to_native())
                    })
                    .collect()
            })
            .unwrap_or_default();
        senses.sort_by_key(|(sense_ord, _)| *sense_ord);
        Ok(senses
            .into_iter()
            .map(|(sense_ord, sense_id)| (sense_ord, self.joined_gloss(sense_id)))
            .collect())
    }

    async fn sense_prop_rows_tagged(
        &self,
        seq: i32,
        tags: &[&str],
    ) -> Result<Vec<(i32, String, String)>, sqlx::Error> {
        let tables = self.tables();
        let indexes = self.idx();
        let mut rows: Vec<(i32, String, String, i32)> = Vec::new();
        if let Some(sense_list) = indexes.sense_by_seq.get(&seq) {
            for &sense_ordinal in sense_list {
                let sense = &tables.senses[sense_ordinal as usize];
                let sense_ord = sense.ord.to_native();
                if let Some(prop_list) = indexes.sense_prop_by_sense_id.get(&sense.id.to_native())
                {
                    for &prop_ordinal in prop_list {
                        let prop = &tables.sense_props[prop_ordinal as usize];
                        if tags.contains(&prop.tag.as_str()) {
                            rows.push((
                                sense_ord,
                                prop.tag.as_str().to_string(),
                                prop.text.as_str().to_string(),
                                prop.ord.to_native(),
                            ));
                        }
                    }
                }
            }
        }
        rows.sort_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| left.1.cmp(&right.1))
                .then_with(|| left.3.cmp(&right.3))
        });
        Ok(rows
            .into_iter()
            .map(|(sense_ord, tag, text, _)| (sense_ord, tag, text))
            .collect())
    }

    async fn first_sense_gloss(&self, seq: i32) -> Result<Option<Option<String>>, sqlx::Error> {
        let tables = self.tables();
        let first_sense = self.idx().sense_by_seq.get(&seq).and_then(|list| {
            list.iter()
                .map(|&ordinal| &tables.senses[ordinal as usize])
                .min_by_key(|row| row.ord.to_native())
        });
        Ok(first_sense.map(|sense| self.joined_gloss(sense.id.to_native())))
    }

    async fn first_sense_gloss_with_pos(
        &self,
        seq: i32,
        pos: &str,
    ) -> Result<Option<Option<String>>, sqlx::Error> {
        let tables = self.tables();
        let indexes = self.idx();
        let first_sense = indexes.sense_by_seq.get(&seq).and_then(|list| {
            list.iter()
                .map(|&ordinal| &tables.senses[ordinal as usize])
                .filter(|sense| {
                    indexes
                        .sense_prop_by_sense_id
                        .get(&sense.id.to_native())
                        .is_some_and(|props| {
                            props.iter().any(|&prop_ordinal| {
                                let prop = &tables.sense_props[prop_ordinal as usize];
                                prop.tag.as_str() == "pos" && prop.text.as_str() == pos
                            })
                        })
                })
                .min_by_key(|row| row.ord.to_native())
        });
        Ok(first_sense.map(|sense| self.joined_gloss(sense.id.to_native())))
    }

    async fn glosses_by_seq_any(
        &self,
        seqs: &[i32],
    ) -> Result<Vec<(i32, String)>, sqlx::Error> {
        let tables = self.tables();
        let indexes = self.idx();
        let mut unique_seqs: Vec<i32> = seqs.to_vec();
        unique_seqs.sort_unstable();
        unique_seqs.dedup();
        let mut out: Vec<(i32, String)> = Vec::new();
        for seq in unique_seqs {
            if let Some(sense_list) = indexes.sense_by_seq.get(&seq) {
                for &sense_ordinal in sense_list {
                    let sense_id = tables.senses[sense_ordinal as usize].id.to_native();
                    if let Some(gloss_list) = indexes.gloss_by_sense_id.get(&sense_id) {
                        for &gloss_ordinal in gloss_list {
                            let gloss = &tables.glosses[gloss_ordinal as usize];
                            out.push((seq, gloss.text.as_str().to_string()));
                        }
                    }
                }
            }
        }
        Ok(out)
    }

    async fn uk_sense_ids(&self, seqs: &[i32]) -> Result<Vec<i32>, sqlx::Error> {
        let tables = self.tables();
        Ok(merged_i32(&self.idx().sense_prop_by_seq, seqs)
            .into_iter()
            .map(|ordinal| &tables.sense_props[ordinal as usize])
            .filter(|prop| prop.tag.as_str() == "misc" && prop.text.as_str() == "uk")
            .map(|prop| prop.sense_id.to_native())
            .collect())
    }

    async fn sense_id_ord0(&self, ids: &[i32]) -> Result<Option<i32>, sqlx::Error> {
        let tables = self.tables();
        let mut ordinals: Vec<u32> = ids
            .iter()
            .filter_map(|id| self.idx().sense_by_id.get(*id))
            .collect();
        ordinals.sort_unstable();
        ordinals.dedup();
        Ok(ordinals
            .into_iter()
            .map(|ordinal| &tables.senses[ordinal as usize])
            .find(|row| row.ord.to_native() == 0)
            .map(|row| row.id.to_native()))
    }

    async fn non_arch_posi(&self, seqs: &[i32]) -> Result<Vec<String>, sqlx::Error> {
        let tables = self.tables();
        let indexes = self.idx();
        let mut seen: HashSet<String> = HashSet::new();
        let mut out: Vec<String> = Vec::new();
        for ordinal in merged_i32(&indexes.sense_prop_by_seq, seqs) {
            let prop = &tables.sense_props[ordinal as usize];
            if prop.tag.as_str() != "pos" {
                continue;
            }
            let sense_is_arch = indexes
                .sense_prop_by_sense_id
                .get(&prop.sense_id.to_native())
                .is_some_and(|props| {
                    props.iter().any(|&other_ordinal| {
                        let other = &tables.sense_props[other_ordinal as usize];
                        other.tag.as_str() == "misc"
                            && matches!(other.text.as_str(), "arch" | "obsc" | "rare")
                    })
                });
            if sense_is_arch {
                continue;
            }
            let text = prop.text.as_str().to_string();
            if seen.insert(text.clone()) {
                out.push(text);
            }
        }
        Ok(out)
    }

    async fn arch_only_seqs(&self) -> Result<Vec<i32>, sqlx::Error> {
        let tables = self.tables();
        let indexes = self.idx();
        let mut first_seen: Vec<i32> = Vec::new();
        let mut every_sense_arch: HashMap<i32, bool> = HashMap::new();
        for sense in tables.senses.iter() {
            let seq = sense.seq.to_native();
            let sense_has_arch = indexes
                .sense_prop_by_sense_id
                .get(&sense.id.to_native())
                .is_some_and(|props| {
                    props.iter().any(|&prop_ordinal| {
                        let prop = &tables.sense_props[prop_ordinal as usize];
                        prop.tag.as_str() == "misc"
                            && matches!(prop.text.as_str(), "arch" | "obsc" | "rare")
                    })
                });
            match every_sense_arch.entry(seq) {
                std::collections::hash_map::Entry::Vacant(slot) => {
                    slot.insert(sense_has_arch);
                    first_seen.push(seq);
                }
                std::collections::hash_map::Entry::Occupied(mut slot) => {
                    *slot.get_mut() = *slot.get() && sense_has_arch;
                }
            }
        }
        Ok(first_seen
            .into_iter()
            .filter(|seq| every_sense_arch[seq])
            .collect())
    }

    async fn counter_seqs(&self) -> Result<Vec<i32>, sqlx::Error> {
        let tables = self.tables();
        let mut seen: HashSet<i32> = HashSet::new();
        let mut out: Vec<i32> = Vec::new();
        for prop in tables.sense_props.iter() {
            if prop.tag.as_str() == "pos" && prop.text.as_str() == "ctr" {
                let seq = prop.seq.to_native();
                if seen.insert(seq) {
                    out.push(seq);
                }
            }
        }
        Ok(out)
    }

    async fn counter_stag_rows(
        &self,
        tag: &str,
        seqs: &[i32],
    ) -> Result<Vec<(i32, String)>, sqlx::Error> {
        let tables = self.tables();
        let indexes = self.idx();
        let mut out: Vec<(i32, String)> = Vec::new();
        for ordinal in merged_i32(&indexes.sense_prop_by_seq, seqs) {
            let prop = &tables.sense_props[ordinal as usize];
            if prop.tag.as_str() != tag {
                continue;
            }
            // self-join multiplicity: one output row per pos=ctr prop
            // sharing the sense.
            let ctr_matches = indexes
                .sense_prop_by_sense_id
                .get(&prop.sense_id.to_native())
                .map(|props| {
                    props
                        .iter()
                        .filter(|&&other_ordinal| {
                            let other = &tables.sense_props[other_ordinal as usize];
                            other.tag.as_str() == "pos" && other.text.as_str() == "ctr"
                        })
                        .count()
                })
                .unwrap_or(0);
            for _ in 0..ctr_matches {
                out.push((prop.seq.to_native(), prop.text.as_str().to_string()));
            }
        }
        Ok(out)
    }

    async fn restricted_readings_by_seq(
        &self,
        seq: i32,
    ) -> Result<Vec<(String, String)>, sqlx::Error> {
        let tables = self.tables();
        Ok(self
            .idx()
            .restricted_by_seq
            .get(&seq)
            .map(|list| {
                list.iter()
                    .map(|&ordinal| {
                        let row = &tables.restricted_readings[ordinal as usize];
                        (
                            row.reading.as_str().to_string(),
                            row.text.as_str().to_string(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    // --- kanjidic ---

    async fn readings_non_nanori_by_kanji_id(
        &self,
        kanji_id: i32,
    ) -> Result<Vec<Reading>, sqlx::Error> {
        let tables = self.tables();
        let mut rows: Vec<Reading> = self
            .idx()
            .reading_by_kanji_id
            .get(&kanji_id)
            .map(|list| {
                list.iter()
                    .map(|&ordinal| &tables.readings[ordinal as usize])
                    .filter(|row| row.reading_type.as_str() != "ja_na")
                    .map(dao_reading)
                    .collect()
            })
            .unwrap_or_default();
        rows.sort_by(|left, right| {
            right
                .reading_type
                .cmp(&left.reading_type)
                .then_with(|| right.stat_common.cmp(&left.stat_common))
        });
        Ok(rows)
    }

    async fn meanings_by_kanji_id(&self, kanji_id: i32) -> Result<Vec<Meaning>, sqlx::Error> {
        let tables = self.tables();
        let mut rows: Vec<Meaning> = self
            .idx()
            .meaning_by_kanji_id
            .get(&kanji_id)
            .map(|list| {
                list.iter()
                    .map(|&ordinal| dao_meaning(&tables.meanings[ordinal as usize]))
                    .collect()
            })
            .unwrap_or_default();
        rows.sort_by_key(|row| row.id);
        Ok(rows)
    }

    async fn okurigana_texts_by_reading_id(
        &self,
        reading_id: i32,
    ) -> Result<Vec<String>, sqlx::Error> {
        let tables = self.tables();
        let mut seen: HashSet<String> = HashSet::new();
        let mut out: Vec<String> = Vec::new();
        if let Some(list) = self.idx().okurigana_by_reading_id.get(&reading_id) {
            for &ordinal in list {
                let text = tables.okurigana[ordinal as usize].text.as_str().to_string();
                if seen.insert(text.clone()) {
                    out.push(text);
                }
            }
        }
        Ok(out)
    }

    async fn kanji_by_text(&self, text: &str) -> Result<Vec<Kanji>, sqlx::Error> {
        let tables = self.tables();
        Ok(self
            .idx()
            .kanji_by_text
            .get(text)
            .map(|list| {
                list.iter()
                    .map(|&ordinal| dao_kanji(&tables.kanji[ordinal as usize]))
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn reading_stats_rows(
        &self,
        kanji: &str,
        reading: &str,
        reading_type: &str,
    ) -> Result<Vec<(i32, i32, Option<i32>)>, sqlx::Error> {
        let tables = self.tables();
        let indexes = self.idx();
        let mut out: Vec<(i32, i32, Option<i32>)> = Vec::new();
        if let Some(kanji_list) = indexes.kanji_by_text.get(kanji) {
            for &kanji_ordinal in kanji_list {
                let kanji_row = &tables.kanji[kanji_ordinal as usize];
                if let Some(reading_list) =
                    indexes.reading_by_kanji_id.get(&kanji_row.id.to_native())
                {
                    for &reading_ordinal in reading_list {
                        let reading_row = &tables.readings[reading_ordinal as usize];
                        if reading_row.text.as_str() == reading
                            && reading_row.reading_type.as_str() == reading_type
                        {
                            out.push((
                                reading_row.stat_common.to_native(),
                                kanji_row.stat_common.to_native(),
                                kanji_row.grade.as_ref().map(|value| value.to_native()),
                            ));
                        }
                    }
                }
            }
        }
        Ok(out)
    }

    async fn kanji_reading_pairs(
        &self,
        text: &str,
        typeset: &[String],
    ) -> Result<Vec<(String, String)>, sqlx::Error> {
        let tables = self.tables();
        let indexes = self.idx();
        let mut rows: Vec<(i32, String, String)> = Vec::new();
        if let Some(kanji_list) = indexes.kanji_by_text.get(text) {
            for &kanji_ordinal in kanji_list {
                let kanji_id = tables.kanji[kanji_ordinal as usize].id.to_native();
                if let Some(reading_list) = indexes.reading_by_kanji_id.get(&kanji_id) {
                    for &reading_ordinal in reading_list {
                        let reading_row = &tables.readings[reading_ordinal as usize];
                        let reading_type = reading_row.reading_type.as_str();
                        if typeset.iter().any(|excluded| excluded == reading_type) {
                            continue;
                        }
                        rows.push((
                            reading_row.id.to_native(),
                            reading_row.text.as_str().to_string(),
                            reading_type.to_string(),
                        ));
                    }
                }
            }
        }
        rows.sort_by_key(|(id, _, _)| *id);
        Ok(rows
            .into_iter()
            .map(|(_, reading_text, reading_type)| (reading_text, reading_type))
            .collect())
    }
}

impl KaniRkyvBackend {
    /// `string_agg(gloss.text, '; ' ORDER BY gloss.ord)` for one sense:
    /// `None` when the sense has no gloss rows (SQL aggregate over an
    /// empty set is NULL).
    fn joined_gloss(&self, sense_id: i32) -> Option<String> {
        let tables = self.tables();
        let mut glosses: Vec<(i32, &str)> = self
            .idx()
            .gloss_by_sense_id
            .get(&sense_id)
            .map(|list| {
                list.iter()
                    .map(|&ordinal| {
                        let row = &tables.glosses[ordinal as usize];
                        (row.ord.to_native(), row.text.as_str())
                    })
                    .collect()
            })
            .unwrap_or_default();
        if glosses.is_empty() {
            return None;
        }
        glosses.sort_by_key(|(gloss_ord, _)| *gloss_ord);
        Some(
            glosses
                .into_iter()
                .map(|(_, text)| text)
                .collect::<Vec<&str>>()
                .join("; "),
        )
    }
}
