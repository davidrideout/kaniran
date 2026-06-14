use crate::conn::kani_backend::KaniBackend;
use crate::characters::text::join;
use crate::conn::kani_context::KaniranContext;
use crate::dict::conj::{conj_info_json, print_conj_info, simplify_reading_list, FilterPropsText};
use crate::dict::dao::WordConjugations;
use crate::dict::grammar::suffix::init::get_suffix_description;
use crate::dict::senses::{get_senses_json, get_senses_str, short_sense_str};
use crate::dict::word_info::{
    word_info_reading, WordInfo, WordInfoKana, WordInfoSeq, WordInfoType,
};
use serde_json::{Map, Value};
use std::borrow::Cow;
use std::fmt::Write;

/// Port of `ichiran/dict:reading-str*` (`dict.lisp:1580`).
///
/// Formats a `(kanji, kana)` pair as `"kanji 【kana】"`, or the bare kana
/// when there's no kanji.
pub fn reading_str_star_(kanji: Option<&str>, kana: Option<&str>) -> Option<String> {
    match kanji {
        // ~a of nil prints "NIL"
        Some(kanji) => Some(format!("{} 【{}】", kanji, kana.unwrap_or("NIL"))),
        None => kana.map(str::to_string),
    }
}

/// Port of `ichiran/dict:reading-str-seq` (`dict.lisp:1584`).
///
/// Looks up the ord-0 kanji and kana surface forms for `seq` and formats
/// them via `reading-str*`.
pub fn reading_str_seq(
    ctx: &KaniranContext,
    seq: i32,
) -> Result<Option<String>, crate::conn::KaniDbError> {
    let kanji_text: Option<String> = ctx.store.headword_kanji_text(seq)?;
    let kana_text: Option<String> = ctx.store.headword_kana_text(seq)?;
    Ok(reading_str_star_(
        kanji_text.as_deref(),
        kana_text.as_deref(),
    ))
}

/// Port of `ichiran/dict:entry-info-short` (`dict.lisp:1595`).
///
/// Formats a seq as `"<reading> : <short sense str>"`.
pub fn entry_info_short(
    ctx: &KaniranContext,
    seq: i32,
    with_pos: Option<&str>,
) -> Result<String, crate::conn::KaniDbError> {
    let sense_str = short_sense_str(ctx, seq, with_pos)?;
    // ~a of a nil reading prints "NIL"
    let mut s = format!(
        "{} : ",
        reading_str_seq(ctx, seq)?.as_deref().unwrap_or("NIL")
    );
    if let Some(sense_str) = sense_str {
        s.push_str(&sense_str);
    }
    Ok(s)
}

/// Port of `ichiran/dict:entry-info-long` (`dict.lisp:1601`).
///
/// Formats a seq as its number, reading line, and full sense text;
/// the reading line is omitted when the reading is nil.
pub fn entry_info_long(ctx: &KaniranContext, seq: i32) -> Result<String, crate::conn::KaniDbError> {
    let mut out = seq.to_string();
    // dict.lisp:1601 (~@[ ~a~%~]) — " <reading>\n" only when reading-str-seq
    // is non-nil; nil prints nothing (unlike entry-info-short's bare ~a).
    if let Some(reading) = reading_str_seq(ctx, seq)? {
        writeln!(out, " {}", reading).unwrap();
    }
    out.push_str(&get_senses_str(ctx, seq)?);
    Ok(out)
}

/// Port of `ichiran/dict:map-word-info-kana` (`dict.lisp:1727`).
///
/// Apply `fn_` to a word-info's kana. When the kana is a list, map `fn_`
/// over each element, simplify the readings, and join with `separator`;
/// when it is a single string, return `fn_` of it. A nil kana takes the
/// list branch and yields the empty string.
pub fn map_word_info_kana<F>(fn_: F, word_info: &WordInfo, separator: &str) -> String
where
    F: Fn(&Option<WordInfoKana>) -> String,
{
    let wkana = &word_info.kana;
    match wkana {
        // (listp wkana) is nil for a string -> (funcall fn wkana).
        Some(WordInfoKana::Single(_)) => fn_(wkana),
        // (listp wkana) is t for a list -> (mapcar fn wkana).
        Some(WordInfoKana::Multi(elements)) => {
            let mapped: Vec<String> = elements.iter().map(|element| fn_(element)).collect();
            join(separator, &simplify_reading_list(&mapped))
        }
        // (listp nil) is t -> (mapcar fn nil) = nil -> join over empty = "".
        None => join(separator, &simplify_reading_list(&[])),
    }
}

/// Port of `ichiran/dict:word-info-reading-str` (`dict.lisp:1733`).
///
/// Renders a [`WordInfo`]'s text/kana into a reading string via
/// [`reading_str_star_`] — kanji (or a counter with a seq) gets both
/// sides; everything else passes only the text as the kana side.
pub fn word_info_reading_str(word_info: &WordInfo) -> Option<String> {
    if word_info.kind == WordInfoType::Kanji
        || (word_info.counter.is_some() && word_info.seq.is_some())
    {
        let kana = word_info.kana.as_ref().map(princ_kana);
        reading_str_star_(Some(&word_info.text), kana.as_deref())
    } else {
        reading_str_star_(None, Some(&word_info.text))
    }
}

// `~a`/princ rendering of a kana value: a string prints verbatim, a list
// prints `(elem ...)` with nil → NIL and nested lists recursing.
fn princ_kana(kana: &WordInfoKana) -> Cow<'_, str> {
    match kana {
        WordInfoKana::Single(reading) => Cow::Borrowed(reading),
        WordInfoKana::Multi(readings) => {
            let elems: Vec<String> = readings
                .iter()
                .map(|reading| match reading {
                    None => "NIL".to_string(),
                    Some(reading) => princ_kana(reading).into_owned(),
                })
                .collect();
            Cow::Owned(format!("({})", elems.join(" ")))
        }
    }
}

/// Port of `ichiran/dict:word-info-str` (`dict.lisp:1745`).
///
/// Renders a [`WordInfo`] to its human-readable string form — one
/// numbered block per component for an alternative word-info,
/// otherwise a single block.
pub fn word_info_str(
    ctx: &KaniranContext,
    word_info: &WordInfo,
) -> Result<String, crate::conn::KaniDbError> {
    let mut s = String::new();
    if word_info.alternative {
        // dict.lisp:1775-1779 (loop for wi … for i from 1 when (> i 1) do (terpri s) do (format s "<~a>. " i) (inner wi nil nil))
        for (index, wi) in word_info.components.iter().enumerate() {
            let i = index + 1;
            if i > 1 {
                s.push('\n');
            }
            write!(s, "<{}>. ", i).unwrap();
            word_info_str_inner(ctx, wi, false, false, &mut s)?;
        }
    } else {
        word_info_str_inner(ctx, word_info, false, false, &mut s)?;
    }
    Ok(s)
}

// dict.lisp:1748 (labels word_info_str_inner (word-info &optional suffix marker))
fn word_info_str_inner(
    ctx: &KaniranContext,
    word_info: &WordInfo,
    suffix: bool,
    marker: bool,
    s: &mut String,
) -> Result<(), crate::conn::KaniDbError> {
    if marker {
        s.push_str(" * ");
    }
    // (princ (reading-str word-info) s)
    s.push_str(word_info.reading_str().as_deref().unwrap_or("NIL"));
    if !word_info.components.is_empty() {
        // dict.lisp:1754 (format s " Compound word: ~{~a~^ + ~}" (mapcar #'word-info-text components))
        let texts: Vec<&str> = word_info
            .components
            .iter()
            .map(|comp| comp.text.as_str())
            .collect();
        write!(s, " Compound word: {}", texts.join(" + ")).unwrap();
        // dict.lisp:1755-1757 (dolist (comp components) (terpri s) (inner comp (not (word-info-primary comp)) t))
        for comp in &word_info.components {
            s.push('\n');
            word_info_str_inner(ctx, comp, !comp.primary, true, s)?;
        }
    } else if let Some((value, _ordinal)) = &word_info.counter {
        // dict.lisp:1759-1763 (destructuring-bind (value ordinal) (word-info-counter …) (terpri s) (princ value s) …)
        s.push('\n');
        s.push_str(value);
        if let Some(seq) = word_info_seq_single(word_info) {
            s.push('\n');
            s.push_str(&get_senses_str(ctx, seq)?);
        }
    } else {
        // dict.lisp:1765-1774
        let seq = word_info_seq_single(word_info);
        let conjs = word_info.conjugations.as_ref();
        // (cond ((and suffix (setf desc (get-suffix-description seq))) …)
        //       ((or (not conjs) (eql conjs :root)) …))
        let mut desc: Option<&'static str> = None;
        if suffix {
            if let Some(seq) = seq {
                desc = get_suffix_description(ctx, seq);
            }
        }
        if let Some(desc) = desc {
            write!(s, "  [suffix]: {} ", desc).unwrap();
        } else if conjs.is_none() || matches!(conjs, Some(WordConjugations::Root)) {
            s.push('\n');
            match seq {
                Some(seq) => s.push_str(&get_senses_str(ctx, seq)?),
                None => s.push_str("???"),
            }
        }
        // (when seq (print-conj-info seq :out s :conjugations conjs))
        if let Some(seq) = seq {
            print_conj_info(ctx, seq, conjs, s)?;
        }
    }
    Ok(())
}

// (word-info-seq word-info) is a single int or nil in the counter and default
// branches; a list seq only occurs on a compound/alternative word-info, which
// the components branch and top-level alternative loop handle first.
fn word_info_seq_single(word_info: &WordInfo) -> Option<i32> {
    match &word_info.seq {
        Some(WordInfoSeq::Single(seq)) => Some(*seq),
        None => None,
        Some(WordInfoSeq::Multi(_)) => {
            panic!("word-info-str: non-compound word-info seq is WordInfoSeq::Multi")
        }
    }
}

/// Port of `ichiran/dict:word-info-gloss-json` (`dict.lisp:1782`).
///
/// Builds the gloss JSON object for a [`WordInfo`] — reading / text /
/// kana / score plus, per word kind, compound components, counter
/// value, suffix description, sense gloss, and conjugation info.
// jsown renders CL `nil` as `[]`.
fn nil() -> Value {
    Value::Array(Vec::new())
}

// `(word-info-kana word-info)` handed straight to jsown: a string prints as
// itself, nil as `[]`, a list as a JSON array (mirrors word-info-json's slot).
fn kana_json(kana: &WordInfoKana) -> Value {
    match kana {
        WordInfoKana::Single(reading) => Value::String(reading.clone()),
        WordInfoKana::Multi(readings) => Value::Array(
            readings
                .iter()
                .map(|reading| reading.as_ref().map_or_else(nil, kana_json))
                .collect(),
        ),
    }
}

// The counter and `(t ...)` cond arms run only when components is empty; a
// list seq co-occurs with components (word-info-from-segment-list), handled by
// the compound arm first. Upstream `(get-senses-json (list …))` would raise a
// PostgreSQL `integer = record` error on a list seq reaching here.
fn single_seq(seq: &Option<WordInfoSeq>) -> Option<i32> {
    match seq {
        None => None,
        Some(WordInfoSeq::Single(seq)) => Some(*seq),
        Some(WordInfoSeq::Multi(_)) => panic!(
            "word-info-gloss-json: list seq in a non-compound branch \
             — upstream `(get-senses-json (list …))` raises `integer = record`"
        ),
    }
}

fn word_info_gloss_json_inner<'a>(
    ctx: &'a KaniranContext,
    word_info: &'a WordInfo,
    suffix: bool,
    root_only: bool,
) -> Result<Value, crate::conn::KaniDbError> {
        let mut js = Map::new();
        // ("reading" (reading-str word-info)) / ("text" …) / ("kana" …)
        js.insert(
            "reading".to_owned(),
            word_info.reading_str().map_or_else(nil, Value::String),
        );
        js.insert("text".to_owned(), Value::String(word_info.text.clone()));
        js.insert(
            "kana".to_owned(),
            word_info.kana.as_ref().map_or_else(nil, kana_json),
        );
        // (when (word-info-score word-info) ("score" …)) — 0 is truthy; only nil skips.
        if let Some(score) = word_info.score {
            js.insert("score".to_owned(), Value::from(score));
        }

        if !word_info.components.is_empty() {
            // ((word-info-components word-info) …)
            js.insert(
                "compound".to_owned(),
                Value::Array(
                    word_info
                        .components
                        .iter()
                        .map(|component| Value::String(component.text.clone()))
                        .collect(),
                ),
            );
            let mut components = Vec::with_capacity(word_info.components.len());
            for component in &word_info.components {
                // (inner wi (not (word-info-primary wi)))
                components.push(
                    word_info_gloss_json_inner(ctx, component, !component.primary, root_only)
                        ?,
                );
            }
            js.insert("components".to_owned(), Value::Array(components));
        } else if let Some((value, ordinal)) = &word_info.counter {
            // ((word-info-counter word-info) …)
            let mut counter = Map::new();
            counter.insert("value".to_owned(), Value::String(value.clone()));
            counter.insert(
                "ordinal".to_owned(),
                if *ordinal { Value::Bool(true) } else { nil() },
            );
            js.insert("counter".to_owned(), Value::Object(counter));
            // (let ((seq (word-info-seq word-info))) (when seq …))
            if let Some(seq) = single_seq(&word_info.seq) {
                js.insert("seq".to_owned(), Value::from(seq));
                // (get-senses-json seq :pos-list '("ctr") :reading-getter reading-getter)
                let pos_list = [String::from("ctr")];
                let gloss = get_senses_json(
                    ctx,
                    seq,
                    &pos_list,
                    None,
                    Some(|| word_info_reading(ctx, word_info)),
                )
                ?;
                // (when gloss …)
                if !gloss.is_empty() {
                    js.insert("gloss".to_owned(), Value::Array(gloss));
                }
            }
        } else {
            // (t …)
            let seq = single_seq(&word_info.seq);
            let conjs = word_info.conjugations.as_ref();
            // (when seq ("seq" seq))
            if let Some(seq) = seq {
                js.insert("seq".to_owned(), Value::from(seq));
            }
            let mut has_gloss = false;
            if root_only {
                // (return-from word_info_gloss_json_inner (extend-js js ("gloss" (get-senses-json seq :reading-getter …))))
                let gloss = match seq {
                    Some(seq) => {
                        get_senses_json(
                            ctx,
                            seq,
                            &[],
                            None,
                            Some(|| word_info_reading(ctx, word_info)),
                        )
                        ?
                    }
                    None => Vec::new(),
                };
                js.insert("gloss".to_owned(), Value::Array(gloss));
                return Ok(Value::Object(js));
            }
            // ((and suffix (setf desc (get-suffix-description seq))) …)
            let mut suffix_done = false;
            if suffix {
                if let Some(seq) = seq {
                    if let Some(desc) = get_suffix_description(ctx, seq) {
                        js.insert("suffix".to_owned(), Value::String(desc.to_owned()));
                        suffix_done = true;
                    }
                }
            }
            // ((and seq (or (not conjs) (eql conjs :root))) …)
            if !suffix_done {
                if let Some(seq) = seq {
                    if conjs.is_none() || matches!(conjs, Some(WordConjugations::Root)) {
                        let gloss = get_senses_json(
                            ctx,
                            seq,
                            &[],
                            None,
                            Some(|| word_info_reading(ctx, word_info)),
                        )
                        ?;
                        // (when gloss (setf has-gloss t) ("gloss" gloss))
                        if !gloss.is_empty() {
                            has_gloss = true;
                            js.insert("gloss".to_owned(), Value::Array(gloss));
                        }
                    }
                }
            }
            // (when seq ("conj" (conj-info-json seq :conjugations … :text … :has-gloss …)))
            if let Some(seq) = seq {
                let text = match &word_info.true_text {
                    Some(true_text) => FilterPropsText::One(true_text),
                    None => FilterPropsText::None,
                };
                let conj =
                    conj_info_json(ctx, seq, word_info.conjugations.as_ref(), text, has_gloss)
                        ?;
                js.insert("conj".to_owned(), Value::Array(conj));
            }
        }
        Ok(Value::Object(js))
}

pub fn word_info_gloss_json(
    ctx: &KaniranContext,
    word_info: &WordInfo,
    root_only: bool,
) -> Result<Value, crate::conn::KaniDbError> {
    if word_info.alternative {
        // (jsown:new-js ("alternative" (mapcar #'word_info_gloss_json_inner (word-info-components word-info))))
        let mut alternatives = Vec::with_capacity(word_info.components.len());
        for component in &word_info.components {
            alternatives.push(word_info_gloss_json_inner(ctx, component, false, root_only)?);
        }
        let mut js = Map::new();
        js.insert("alternative".to_owned(), Value::Array(alternatives));
        Ok(Value::Object(js))
    } else {
        word_info_gloss_json_inner(ctx, word_info, false, root_only)
    }
}

/// Port of `ichiran/dict:get-kanji-words` (`dict.lisp:1834`).
///
/// Returns `(seq, kanji-text, kana-text, common)` rows for every root
/// entry whose kanji writing contains `char` as a substring, restricted
/// to the best-kana reading with a non-null `common`.
pub fn get_kanji_words(
    ctx: &KaniranContext,
    char: &str,
) -> Result<Vec<(i32, String, String, i32)>, crate::conn::KaniDbError> {
    ctx.store.kanji_words_containing_char(char)
}

#[cfg(test)]
mod tests;
