use crate::conn::kani_backend::KaniBackend;
use super::kani_kanji_reading::KanjiReading;
use crate::characters::voicing::{geminate, rendaku as voice_rendaku, unrendaku, Voicing};
use crate::conn::kani_context::KaniranContext;
use std::collections::HashSet;

/// Port of `ichiran/kanji:get-readings-cache` (`kanji.lisp:201`).
pub fn get_readings_cache(
    ctx: &KaniranContext,
    text: &str,
    typeset: &[String],
) -> Result<Vec<(String, String)>, crate::conn::KaniDbError> {
    let key = (text.to_string(), typeset.to_vec());
    {
        let cache = ctx.reading_cache.lock().unwrap();
        if let Some(val) = cache.get(&key) {
            return Ok(val.clone());
        }
    }
    let result: Vec<(String, String)> = if typeset.is_empty() {
        Vec::new()
    } else {
        // kanji.lisp:206 ((:select 'r.text 'r.type :from (:as 'kanji 'k) ...))
        // The store method's ORDER BY r.id diverges from upstream's
        // unordered SELECT: it returns each kanji's readings in
        // load_readings insertion order (= kanjidic2 order), so
        // get_normal_readings' first-occurrence dedup breaks
        // ambiguous-gemination ties deterministically. Without it the
        // JOIN order is unstable and reading.stat_common drifts
        // run-to-run.
        ctx.store
            .kanji_reading_pairs(text, typeset)?
            .into_iter()
            .map(|(reading, reading_type)| (reading.into_owned(), reading_type.into_owned()))
            .collect()
    };
    {
        let mut cache = ctx.reading_cache.lock().unwrap();
        cache.insert(key, result.clone());
    }
    Ok(result)
}

/// Port of `ichiran/kanji:get-readings` (`kanji.lisp:213`).
///
/// Looks up the kanjidic2 readings of `char`, defaulting to everything
/// except `ja_na` (named-reading) entries. With `names` set the typeset
/// filter is empty and the call returns an empty `Vec`.
pub fn get_readings(
    ctx: &KaniranContext,
    char: char,
    names: bool,
) -> Result<Vec<(String, String)>, crate::conn::KaniDbError> {
    let str: String = char.into();
    let typeset: Vec<String> = if names {
        Vec::new()
    } else {
        vec!["ja_na".to_string()]
    };
    get_readings_cache(ctx, &str, &typeset)
}

/// Port of `ichiran/kanji:get-reading-alternatives` (`kanji.lisp:218`).
///
/// Expands a single `(reading, type)` kanjidic2 reading into its variant
/// readings. The base entry is always emitted; a trailing-mora geminated
/// (`っ`) entry is added when `type` is `"ja_on"` and the last character
/// is one of つ/く/き/ち; with `rendaku` set, every entry above is also
/// duplicated dakuten- and handakuten-voiced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadingTag {
    Plain,
    Rendaku,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadingAlternative {
    pub reading: String,
    pub r#type: String,
    pub tag: ReadingTag,
    pub gem: Option<String>,
}

pub fn get_reading_alternatives(
    reading: &str,
    r#type: &str,
    rendaku: bool,
) -> Vec<ReadingAlternative> {
    let chars: Vec<char> = reading.chars().collect();
    let mut lst: Vec<ReadingAlternative> = vec![ReadingAlternative {
        reading: reading.to_string(),
        r#type: r#type.to_string(),
        tag: ReadingTag::Plain,
        gem: None,
    }];

    if chars.len() > 1 && r#type == "ja_on" {
        let last = chars[chars.len() - 1];
        if matches!(last, 'つ' | 'く' | 'き' | 'ち') {
            let mut copy = reading.to_string();
            geminate(&mut copy);
            lst.push(ReadingAlternative {
                reading: copy,
                r#type: r#type.to_string(),
                tag: ReadingTag::Plain,
                gem: Some(last.to_string()),
            });
        }
    }

    if rendaku {
        let snapshot = lst.clone();
        for entry in &snapshot {
            let mut rd = entry.reading.clone();
            voice_rendaku(&mut rd, Voicing::Dakuten);
            lst.push(ReadingAlternative {
                reading: rd,
                r#type: r#type.to_string(),
                tag: ReadingTag::Rendaku,
                gem: entry.gem.clone(),
            });
            let mut rd_h = entry.reading.clone();
            voice_rendaku(&mut rd_h, Voicing::Handakuten);
            lst.push(ReadingAlternative {
                reading: rd_h,
                r#type: r#type.to_string(),
                tag: ReadingTag::Rendaku,
                gem: entry.gem.clone(),
            });
        }
    }

    lst
}

/// Port of `ichiran/kanji:get-normal-readings` (`kanji.lisp:231`).
///
/// Looks up the kun/on readings of `char` (excluding `ja_na`
/// named-reading rows), expands each into geminate / rendaku variants,
/// then deduplicates by reading text keeping the first occurrence.
pub fn get_normal_readings(
    ctx: &KaniranContext,
    char: char,
    rendaku: bool,
) -> Result<Vec<KanjiReading>, crate::conn::KaniDbError> {
    let str: String = char.into();
    let typeset = vec!["ja_na".to_string()];
    let readings = get_readings_cache(ctx, &str, &typeset)?;

    let mut main_readings: Vec<KanjiReading> = Vec::new();
    let mut alt_readings: Vec<KanjiReading> = Vec::new();
    for (reading, r#type) in &readings {
        let alternatives = get_reading_alternatives(reading, r#type, rendaku);
        // kanji.lisp:235 (loop ... for (main . rest) = ...)
        let mut iter = alternatives.into_iter();
        if let Some(main) = iter.next() {
            main_readings.push(main.into());
        }
        for alt in iter {
            alt_readings.push(alt.into());
        }
    }

    let mut combined: Vec<KanjiReading> = main_readings;
    combined.extend(alt_readings);

    // kanji.lisp:239 (remove-duplicates ... :test 'equal :key 'car :from-end t)
    // — keep the first occurrence in original order (verified empirically).
    let mut seen: HashSet<String> = HashSet::new();
    let mut deduped: Vec<KanjiReading> = Vec::with_capacity(combined.len());
    for entry in combined {
        if seen.insert(entry.reading.clone()) {
            deduped.push(entry);
        }
    }
    Ok(deduped)
}

/// Port of `ichiran/kanji:get-original-reading` (`kanji.lisp:308`).
///
/// Recovers the underlying kun/on dictionary form from a reading
/// variant: strips dakuten/handakuten when `rendaku` is set, and
/// replaces the trailing character with the supplied `geminated` glyph
/// when present.
pub fn get_original_reading(rtext: &str, rendaku: bool, geminated: Option<&str>) -> String {
    let mut s = rtext.to_string();
    if rendaku {
        unrendaku(&mut s);
    }
    if let Some(g) = geminated {
        // kanji.lisp:311 ((setf (char rtext (1- (length rtext))) (char geminated 0)))
        let new_first = g
            .chars()
            .next()
            .expect("geminated is non-empty when present");
        let last_pos = s
            .char_indices()
            .last()
            .expect("rtext is non-empty when geminated is set")
            .0;
        let mut buf = [0u8; 4];
        let new_str = new_first.encode_utf8(&mut buf);
        s.replace_range(last_pos.., new_str);
    }
    s
}

#[cfg(test)]
mod tests;
