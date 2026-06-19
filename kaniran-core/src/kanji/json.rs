use crate::conn::kani_backend::KaniBackend;
use super::dao::{Kanji, Meaning, Reading};
use super::matching::{match_readings, MatchedSegment};
use super::readings::{get_original_reading, ReadingTag};
use super::stats::{calculate_perc, get_reading_stats};
use crate::characters::constants::{KANJI_CHAR_REGEX, KANJI_REGEX};
use crate::conn::kani_context::KaniranContext;
use crate::core::methods::hepburn_basic;
use crate::core::methods::RomanizationMethod;
use crate::core::romanize::romanize_word;
use fancy_regex::Regex;
use serde_json::{Map, Value};
use std::sync::OnceLock;

/// Port of `ichiran/kanji:to-json` (`kanji.lisp:369`), the sole method
/// of the `to-json` generic (`kanji.lisp:7`).
///
/// Builds a JSON object describing one `kanji` row: its text, radical
/// and stroke fields, common/irregular stat counts and percentage, the
/// non-nanori readings (each via [`reading_info_json`]), the meanings,
/// and `freq`/`grade` when present.
pub fn to_json(ctx: &KaniranContext, kanji: &Kanji) -> Result<Value, crate::conn::KaniDbError> {
    let total = kanji.stat_common;
    let mut js = Map::new();
    js.insert("text".to_owned(), Value::String(kanji.text.clone()));
    js.insert("rc".to_owned(), Value::Number(kanji.radical_c.into()));
    js.insert("rn".to_owned(), Value::Number(kanji.radical_n.into()));
    js.insert("strokes".to_owned(), Value::Number(kanji.strokes.into()));
    js.insert("total".to_owned(), Value::Number(kanji.stat_common.into()));
    js.insert("irr".to_owned(), Value::Number(kanji.stat_irregular.into()));
    js.insert(
        "irr_perc".to_owned(),
        Value::String(calculate_perc(kanji.stat_irregular, total)),
    );
    // kanji.lisp:379-383 ((select-dao 'reading (:and (:= 'kanji-id (id kanji)) (:not (:= 'type "ja_na"))) (:desc 'type) (:desc 'stat-common)))
    let readings: Vec<Reading> = ctx.store.readings_non_nanori_by_kanji_id(kanji.id)?;
    let mut readings_json = Vec::with_capacity(readings.len());
    for reading in &readings {
        readings_json.push(reading_info_json(ctx, reading, total)?);
    }
    js.insert("readings".to_owned(), Value::Array(readings_json));
    // kanji.lisp:384 ((mapcar 'text (select-dao 'meaning (:= 'kanji-id (id kanji)) 'id)))
    let meanings: Vec<Meaning> = ctx.store.meanings_by_kanji_id(kanji.id)?;
    js.insert(
        "meanings".to_owned(),
        Value::Array(
            meanings
                .iter()
                .map(|m| Value::String(m.text.clone()))
                .collect(),
        ),
    );
    // kanji.lisp:386-389 ((when (freq kanji) ...) / (when (grade kanji) ...)) — the slot reads
    // :null for a SQL NULL, which is truthy in Lisp, so both keys are always emitted; a NULL
    // column becomes JSON null.
    js.insert(
        "freq".to_owned(),
        match kanji.freq {
            Some(freq) => Value::Number(freq.into()),
            None => Value::Null,
        },
    );
    js.insert(
        "grade".to_owned(),
        match kanji.grade {
            Some(grade) => Value::Number(grade.into()),
            None => Value::Null,
        },
    );
    Ok(Value::Object(js))
}

/// Port of `ichiran/kanji:reading-info-json` (`kanji.lisp:354`).
///
/// Builds a JSON object describing one `reading` row: its text,
/// romanized form, type, distinct okurigana list, corpus sample count
/// and percentage, plus `prefixp`/`suffixp` flags when set.
pub fn reading_info_json(
    ctx: &KaniranContext,
    reading: &Reading,
    total: i32,
) -> Result<Value, crate::conn::KaniDbError> {
    let mut js = Map::new();
    js.insert("text".to_owned(), Value::String(reading.text.clone()));
    // kanji.lisp:358 ((romanize-word (text reading) :method *hepburn-basic* :original-spelling ""))
    js.insert(
        "rtext".to_owned(),
        Value::String(romanize_word(
            &reading.text,
            RomanizationMethod::GenericHepburn(hepburn_basic()),
            Some(""),
            true,
        )),
    );
    js.insert(
        "type".to_owned(),
        Value::String(reading.reading_type.clone()),
    );
    // kanji.lisp:360 ((query (:select 'text :distinct :from 'okurigana :where (:= 'reading-id (id reading))) :column))
    let okuri: Vec<String> = ctx
        .store
        .okurigana_texts_by_reading_id(reading.id)?
        .into_iter()
        .map(|text| text.into_owned())
        .collect();
    js.insert(
        "okuri".to_owned(),
        Value::Array(okuri.into_iter().map(Value::String).collect()),
    );
    js.insert(
        "sample".to_owned(),
        Value::Number(reading.stat_common.into()),
    );
    js.insert(
        "perc".to_owned(),
        Value::String(calculate_perc(reading.stat_common, total)),
    );
    if reading.prefixp {
        js.insert("prefixp".to_owned(), Value::Bool(true));
    }
    if reading.suffixp {
        js.insert("suffixp".to_owned(), Value::Bool(true));
    }
    Ok(Value::Object(js))
}

/// Port of `ichiran/kanji:kanji-info-json` (`kanji.lisp:392`).
///
/// Looks up the `kanji` row whose `text` equals `char` and returns its
/// [`Kanji::to_json`] object, or [`None`] when no row matches.
pub fn kanji_info_json(
    ctx: &KaniranContext,
    char: &str,
) -> Result<Option<Value>, crate::conn::KaniDbError> {
    let str = char;
    // kanji.lisp:395 ((car (select-dao 'kanji (:= 'text str))))
    let kanji: Option<Kanji> = ctx.store.kanji_by_text(str)?.into_iter().next();
    match kanji {
        Some(kanji) => Ok(Some(to_json(ctx, &kanji)?)),
        None => Ok(None),
    }
}

/// Port of `ichiran/kanji:kanji-reading-json` (`kanji.lisp:410`).
///
/// Builds a JSON object describing one kanji/reading/type triple, adding
/// `link`, `rendaku`, `geminated`, and corpus `stats` fields when they
/// apply.
fn kanji_reading_json_kanji_char_scanner() -> &'static Regex {
    static KANJI_CHAR_SCANNER: OnceLock<Regex> = OnceLock::new();
    KANJI_CHAR_SCANNER
        .get_or_init(|| Regex::new(KANJI_CHAR_REGEX).expect("*kanji-char-regex* must compile"))
}

pub fn kanji_reading_json(
    ctx: &KaniranContext,
    kanji: &str,
    reading: &str,
    r#type: &str,
    rendaku: bool,
    geminated: Option<&str>,
) -> Result<Value, crate::conn::KaniDbError> {
    let mut js = Map::new();
    js.insert("kanji".to_owned(), Value::String(kanji.to_owned()));
    js.insert("reading".to_owned(), Value::String(reading.to_owned()));
    js.insert("type".to_owned(), Value::String(r#type.to_owned()));
    // kanji.lisp:412 ((ppcre:scan *kanji-char-regex* kanji))
    if kanji_reading_json_kanji_char_scanner()
        .is_match(kanji)
        .expect("scan over fixed *kanji-char-regex* pattern cannot fail")
    {
        js.insert("link".to_owned(), Value::Bool(true));
    }
    if rendaku {
        // kanji.lisp:415 ((jsown:extend-js js ("rendaku" rendaku))) — jsown renders :rendaku as "RENDAKU"
        js.insert("rendaku".to_owned(), Value::String("RENDAKU".to_owned()));
    }
    if let Some(geminated) = geminated {
        js.insert("geminated".to_owned(), Value::String(geminated.to_owned()));
    }
    // kanji.lisp:418 ((get-reading-stats kanji (get-original-reading reading rendaku geminated) type))
    let stats = get_reading_stats(
        ctx,
        kanji,
        &get_original_reading(reading, rendaku, geminated),
        r#type,
    )
    ?;
    if let Some((sample, total, perc, grade)) = stats {
        js.insert("stats".to_owned(), Value::Bool(true));
        js.insert("sample".to_owned(), Value::Number(sample.into()));
        js.insert("total".to_owned(), Value::Number(total.into()));
        js.insert("perc".to_owned(), Value::String(perc));
        // kanji.lisp:423 ((when (not (eql grade :null)) ...))
        if let Some(grade) = grade {
            js.insert("grade".to_owned(), Value::Number(grade.into()));
        }
    }
    Ok(Value::Object(js))
}

/// Port of `ichiran/kanji:process-match-json` (`kanji.lisp:428`).
///
/// Walks the [`MatchedSegment`] list from [`super::match_readings`]:
/// each non-`irr` kanji segment becomes a [`super::kanji_reading_json`]
/// object, each non-kanji run a `{"text": …}` object, and consecutive
/// `irr` kanji segments coalesce into one `{"kanji", "reading", "type":
/// "irr"}` object whose texts are the concatenated segments, gaining
/// `"link": true` when the concatenation holds a CJK ideograph.
fn process_match_json_kanji_char_scanner() -> &'static Regex {
    static KANJI_CHAR_SCANNER: OnceLock<Regex> = OnceLock::new();
    KANJI_CHAR_SCANNER
        .get_or_init(|| Regex::new(KANJI_CHAR_REGEX).expect("*kanji-char-regex* must compile"))
}

// kanji.lisp:430-438 (empty-bag closure) — flush the accumulated irr segments
// into one "irr" object. Segments accumulate in original order here (Lisp
// pushes then `nreverse`s), so no reversal is needed.
fn empty_bag(irrbag: &mut Vec<(&str, &str)>, result: &mut Vec<Value>) {
    let kanji: String = irrbag.iter().map(|(kanji, _)| *kanji).collect();
    let reading: String = irrbag.iter().map(|(_, reading)| *reading).collect();
    let link = process_match_json_kanji_char_scanner()
        .is_match(&kanji)
        .expect("scan over fixed *kanji-char-regex* pattern cannot fail");
    let mut js = Map::new();
    js.insert("kanji".to_owned(), Value::String(kanji));
    js.insert("reading".to_owned(), Value::String(reading));
    js.insert("type".to_owned(), Value::String("irr".to_owned()));
    if link {
        js.insert("link".to_owned(), Value::Bool(true));
    }
    result.push(Value::Object(js));
    irrbag.clear();
}

pub fn process_match_json(
    ctx: &KaniranContext,
    match_: &[MatchedSegment],
) -> Result<Vec<Value>, crate::conn::KaniDbError> {
    let mut irrbag: Vec<(&str, &str)> = Vec::new();
    let mut result: Vec<Value> = Vec::new();
    for item in match_ {
        match item {
            // kanji.lisp:440-445 ((listp item) cond)
            MatchedSegment::Kanji { kanji, reading } => {
                if reading.r#type == "irr" {
                    irrbag.push((kanji.as_str(), reading.reading.as_str()));
                } else {
                    if !irrbag.is_empty() {
                        empty_bag(&mut irrbag, &mut result);
                    }
                    // kanji.lisp:445 ((apply 'kanji-reading-json item))
                    result.push(
                        kanji_reading_json(
                            ctx,
                            kanji,
                            &reading.reading,
                            &reading.r#type,
                            matches!(reading.tag, Some(ReadingTag::Rendaku)),
                            reading.gem.as_deref(),
                        )
                        ?,
                    );
                }
            }
            // kanji.lisp:446-448 (else do (when irrbag (funcall empty-bag)) (push (jsown:new-js ("text" item)) result))
            MatchedSegment::NonKanji(text) => {
                if !irrbag.is_empty() {
                    empty_bag(&mut irrbag, &mut result);
                }
                let mut js = Map::new();
                js.insert("text".to_owned(), Value::String(text.clone()));
                result.push(Value::Object(js));
            }
        }
    }
    // kanji.lisp:449 (finally (when irrbag (funcall empty-bag)))
    if !irrbag.is_empty() {
        empty_bag(&mut irrbag, &mut result);
    }
    Ok(result)
}

/// Port of `ichiran/kanji:match-readings-json` (`kanji.lisp:452`).
///
/// Returns [`None`] when `str` holds no kanji-ish character, or when
/// [`super::match_readings`] cannot align `reading` to it; otherwise the
/// per-segment JSON list from [`super::process_match_json`].
fn kanji_scanner() -> &'static Regex {
    static KANJI_SCANNER: OnceLock<Regex> = OnceLock::new();
    KANJI_SCANNER.get_or_init(|| Regex::new(KANJI_REGEX).expect("*kanji-regex* must compile"))
}

pub fn match_readings_json(
    ctx: &KaniranContext,
    str: &str,
    reading: &str,
) -> Result<Option<Vec<Value>>, crate::conn::KaniDbError> {
    // kanji.lisp:453 ((ppcre:scan *kanji-regex* str))
    if !kanji_scanner()
        .is_match(str)
        .expect("scan over fixed *kanji-regex* pattern cannot fail")
    {
        return Ok(None);
    }
    // kanji.lisp:454-456 ((let ((match (match-readings str reading))) (when match (process-match-json match))))
    match match_readings(ctx, str, reading)? {
        Some(match_) => Ok(Some(process_match_json(ctx, &match_)?)),
        None => Ok(None),
    }
}

/// Looks up each kanji literal in `texts`, maps every matching row
/// through [`to_json`], and extends each resulting object with the
/// caller's extra fields.
///
/// Divergence from `ichiran/kanji:query-kanji-json` (`kanji.lisp:458`):
/// upstream is a macro taking a caller-supplied SQL string passed to
/// `(query-dao 'kanji query)`. That raw-SQL form is Postgres-only — no
/// other backend can serve it — yet every real query is a lookup by
/// kanji text (upstream's own `kanji-info-json` uses the structured
/// `(select-dao 'kanji (:= 'text str))`). Keying on text instead lets
/// both the Postgres and rkyv backends serve it via the shared
/// [`kanji_by_text`](crate::conn::kani_backend::KaniBackend::kanji_by_text)
/// method.
pub fn query_kanji_json(
    ctx: &KaniranContext,
    texts: &[&str],
    extra_fields: impl Fn(&Kanji) -> Vec<(String, Value)>,
) -> Result<Vec<Value>, crate::conn::KaniDbError> {
    let mut result = Vec::new();
    for text in texts {
        // (mapcar (lambda (var) (let ((js (to-json var))) (jsown:extend-js js …))) …)
        for var in &ctx.store.kanji_by_text(text)? {
            let mut js = to_json(ctx, var)?;
            if let Value::Object(map) = &mut js {
                for (key, value) in extra_fields(var) {
                    map.insert(key, value);
                }
            }
            result.push(js);
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests;
