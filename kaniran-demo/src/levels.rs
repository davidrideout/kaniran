//! In-memory difficulty/grade lookups loaded once at startup.
//!
//! Two static data sets are embedded into the binary and parsed into
//! HashMaps when the server boots — no database, no runtime files:
//!
//! - `kanji-jouyou.json` (kanjidic-derived, one record per kanji) gives
//!   per-kanji Joyo `grade`, WaniKani `wk_level`, and JLPT `jlpt_new`.
//! - The cumulative JLPT word lists give per-*word* JLPT level by which
//!   list a surface first appears in.

use std::collections::HashMap;

use serde::Deserialize;

/// Per-kanji difficulty facts surfaced to the client for histograms.
///
/// Example — 暖: `grade: Some(6)`, `wk_level: Some(34)`,
/// `jlpt_new: Some(3)` (Joyo grade 6, WaniKani level 34, JLPT N3).
#[derive(Debug, Clone, Copy)]
pub struct KanjiStats {
    /// Joyo grade 1–8 (`None` for non-Joyo kanji).
    pub grade: Option<i32>,
    /// WaniKani level 1–60 (`None` if the kanji is not taught on WaniKani).
    pub wk_level: Option<i32>,
    /// Modern JLPT level as N-number: 5 = N5 … 1 = N1 (`None` if unlisted).
    pub jlpt_new: Option<i32>,
}

/// One record as it appears in `kanji-jouyou.json`. Only the four fields
/// the demo needs are deserialized; the rest of each record is ignored.
#[derive(Debug, Deserialize)]
struct KanjiRecord {
    grade: Option<i32>,
    wk_level: Option<i32>,
    jlpt_new: Option<i32>,
}

/// Loaded difficulty tables. Cheap to share read-only across requests.
pub struct Levels {
    kanji: HashMap<char, KanjiStats>,
    /// surface → JLPT N-number (5 = N5 … 1 = N1).
    jlpt_words: HashMap<String, u8>,
}

const KANJI_JSON: &str = include_str!("../data/kanji-jouyou.json");

// Cumulative JLPT lists, easiest first. Each file contains every word
// from the easier levels, so the smallest file a word appears in fixes
// its level. N-number: N5 = 5 (easiest) … N1 = 1 (hardest).
const JLPT_LISTS: &[(&str, u8)] = &[
    (include_str!("../data/jlpt/n5.txt"), 5),
    (include_str!("../data/jlpt/n5_n4.txt"), 4),
    (include_str!("../data/jlpt/n5_n4_n3.txt"), 3),
    (include_str!("../data/jlpt/n5_n4_n3_n2.txt"), 2),
    (include_str!("../data/jlpt/n5_n4_n3_n2_n1.txt"), 1),
];

impl Levels {
    /// Parse the embedded data sets. Panics on malformed embedded data —
    /// it ships with the binary, so a parse failure is a build mistake.
    pub fn load() -> Self {
        let records: HashMap<String, KanjiRecord> =
            serde_json::from_str(KANJI_JSON).expect("kanji-jouyou.json: malformed embedded JSON");
        let mut kanji = HashMap::with_capacity(records.len());
        for (key, record) in records {
            // Keys are single-character kanji strings; skip any stray
            // multi-char key rather than guess.
            let mut chars = key.chars();
            if let (Some(character), None) = (chars.next(), chars.next()) {
                kanji.insert(
                    character,
                    KanjiStats {
                        grade: record.grade,
                        wk_level: record.wk_level,
                        jlpt_new: record.jlpt_new,
                    },
                );
            }
        }

        // Insert hardest-first so that the easiest (highest N-number)
        // list a word appears in wins the final assignment.
        let mut jlpt_words: HashMap<String, u8> = HashMap::new();
        for (contents, level) in JLPT_LISTS.iter().rev() {
            for line in contents.lines() {
                // A line may hold several surface forms: "足; 脚".
                for form in line.split(';') {
                    let form = form.trim();
                    if !form.is_empty() {
                        jlpt_words.insert(form.to_owned(), *level);
                    }
                }
            }
        }

        Self { kanji, jlpt_words }
    }

    /// Per-kanji stats for one character, if known.
    pub fn kanji(&self, character: char) -> Option<KanjiStats> {
        self.kanji.get(&character).copied()
    }

    /// Word-level JLPT N-number for a surface form, trying the dictionary
    /// surface then the kana reading (so a kana-written entry still hits).
    pub fn jlpt_word(&self, surface: &str, reading: &str) -> Option<u8> {
        self.jlpt_words
            .get(surface)
            .or_else(|| self.jlpt_words.get(reading))
            .copied()
    }
}
