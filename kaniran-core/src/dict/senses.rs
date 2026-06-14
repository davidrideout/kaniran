use crate::conn::kani_backend::KaniBackend;
use crate::characters::text::join;
use crate::conn::kani_context::KaniranContext;
use crate::dict::accessors::{word_type, WordType};
use crate::dict::counters::methods::{nokanji, text};
use crate::dict::dao::{KanaText, KanjiText};
use crate::dict::kani_word::KaniWordDispatchEnum;
use serde_json::{Map, Value};
use std::fmt::Write;

/// Transliteration of `ichiran/dict:get-senses-raw` (`dict.lisp:1458`).
///
/// Returns one [`RawSense`] per `sense` row attached to `seq`, ordered
/// by `sense.ord`, carrying the joined gloss string and the `(tag,
/// texts)` props (pos / s_inf / stagk / stagr / field).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSense {
    pub ord: i32,
    pub gloss: String,
    pub props: Vec<(String, Vec<String>)>,
}

const TAGS: &[&str] = &["pos", "s_inf", "stagk", "stagr", "field"];

pub fn get_senses_raw(ctx: &KaniranContext, seq: i32) -> Result<Vec<RawSense>, crate::conn::KaniDbError> {
    let gloss_rows = ctx.store.sense_gloss_rows(seq)?;

    let mut sense_list: Vec<RawSense> = Vec::with_capacity(gloss_rows.len());
    for (ord, gloss) in gloss_rows {
        sense_list.push(RawSense {
            ord,
            gloss: gloss.unwrap_or_default(),
            props: Vec::new(),
        });
    }

    let prop_rows = ctx.store.sense_prop_rows_tagged(seq, TAGS)?;

    let mut cur_sord: Option<i32> = None;
    let mut cur_tag: Option<String> = None;
    let mut cur_idx: Option<usize> = None;
    let mut bag: Vec<String> = Vec::new();

    for (sord, tag, text) in prop_rows {
        let changed = cur_sord != Some(sord) || cur_tag.as_deref() != Some(tag.as_str());
        if changed {
            // dict.lisp:1479 (in-loop transition) — emit prior bag in
            // upstream insertion order (Lisp `(reverse bag)` flips
            // `push`-prepended order; Rust `Vec::push` is already in
            // insertion order so no reverse is applied).
            if let Some(idx) = cur_idx {
                let prev_tag = cur_tag.take().unwrap_or_default();
                let prev_bag = std::mem::take(&mut bag);
                sense_list[idx].props.insert(0, (prev_tag, prev_bag));
            }
            cur_sord = Some(sord);
            cur_tag = Some(tag);
            bag.clear();
            cur_idx = sense_list.iter().position(|s| s.ord == sord);
        }
        bag.push(text);
    }
    // dict.lisp:1483 (finally clause) — upstream emits `(cons curtag
    // bag)` without `reverse`, leaving the final group's texts in
    // reverse insertion order. The Rust `Vec::push` produced
    // insertion order, so reverse here to mirror the asymmetry.
    if let Some(idx) = cur_idx {
        let prev_tag = cur_tag.take().unwrap_or_default();
        bag.reverse();
        sense_list[idx].props.insert(0, (prev_tag, bag));
    }

    Ok(sense_list)
}

/// Port of `ichiran/dict:get-senses` (`dict.lisp:1487`).
///
/// Turns each [`get_senses_raw`] sense into a `(pos-str, gloss, props)`
/// tuple, where `pos-str` is the comma-joined pos values bracketed as
/// `[...]`.
pub type SenseEntry = (String, String, Vec<(String, Vec<String>)>);

pub fn get_senses(ctx: &KaniranContext, seq: i32) -> Result<Vec<SenseEntry>, crate::conn::KaniDbError> {
    let raw = get_senses_raw(ctx, seq)?;
    let mut out: Vec<SenseEntry> = Vec::with_capacity(raw.len());
    for sense in raw {
        let pos_str = {
            let pos: &[String] = sense
                .props
                .iter()
                .find(|(tag, _)| tag == "pos")
                .map(|(_, vals)| vals.as_slice())
                .unwrap_or(&[]);
            format!("[{}]", pos.join(","))
        };
        out.push((pos_str, sense.gloss, sense.props));
    }
    Ok(out)
}

/// Port of `ichiran/dict:get-senses-str` (`dict.lisp:1495`).
///
/// Renders an entry's senses as a numbered, newline-separated string,
/// each line showing the pos, optional field/info, and gloss.
pub fn get_senses_str(ctx: &KaniranContext, seq: i32) -> Result<String, crate::conn::KaniDbError> {
    let senses = get_senses(ctx, seq)?;
    let mut out = String::new();
    let mut rpos: &str = "";
    for (i, (pos, gloss, props)) in senses.iter().enumerate() {
        // dict.lisp:1499 (loop for rpos = pos then …) — first iter seeds rpos,
        // later iters keep the prior rpos when the current pos is "[]".
        if i == 0 {
            rpos = pos.as_str();
        } else {
            if pos != "[]" {
                rpos = pos.as_str();
            }
            out.push('\n');
        }
        let inf = props
            .iter()
            .find(|(tag, _)| tag == "s_inf")
            .map(|(_, vals)| join("; ", vals));
        let field = props
            .iter()
            .find(|(tag, _)| tag == "field")
            .map(|(_, vals)| join(",", vals));
        write!(out, "{}. {} ", i + 1, rpos).unwrap();
        if let Some(f) = &field {
            write!(out, "{{{}}} ", f).unwrap();
        }
        if let Some(s) = &inf {
            write!(out, "《{}》 ", s).unwrap();
        }
        out.push_str(gloss);
    }
    Ok(out)
}

/// Port of `ichiran/dict:match-kana-kanji` (`dict.lisp:1507`).
///
/// Tests whether a kana reading and kanji reading are compatible given
/// the `(reading, text)` rows of `restricted-readings`: yields `Yes`
/// when the kana carries no restriction, `Found(s)` when a restricted
/// kanji surface matches, else `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchKanaKanjiResult {
    /// Lisp `t` — the kana reading carries no restricted-reading
    /// constraint, so any kanji pairs.
    Yes,
    /// Lisp `(find (text kanji-reading) restr :test 'equal)` — the
    /// matched kanji surface.
    Found(String),
}

pub fn match_kana_kanji(
    kana_reading: &KaniWordDispatchEnum,
    kanji_reading: &KaniWordDispatchEnum,
    restricted: &[(String, String)],
) -> Option<MatchKanaKanjiResult> {
    // dict.lisp:1508 ((nokanji kana-reading) nil) — `nokanji` has no
    // method for compound-text upstream (no-applicable-method); the
    // dispatcher returns None there and `.expect` surfaces that error.
    if nokanji(kana_reading).expect("nokanji: no method for compound-text") {
        return None;
    }
    // dict.lisp:1509 (kana-text (text kana-reading))
    let kana_text = text(kana_reading);
    let kana_text = kana_text.as_ref();
    // dict.lisp:1510 (restr (loop for (rt kt) in restricted when (equal kana-text rt) collect kt))
    let restr: Vec<&str> = restricted
        .iter()
        .filter(|(rt, _kt)| rt.as_str() == kana_text)
        .map(|(_rt, kt)| kt.as_str())
        .collect();
    // dict.lisp:1511-1513 (if restr (find (text kanji-reading) restr :test 'equal) t)
    if !restr.is_empty() {
        let kanji_text = text(kanji_reading);
        if restr.iter().any(|kt| *kt == kanji_text.as_ref()) {
            Some(MatchKanaKanjiResult::Found(kanji_text.into_owned()))
        } else {
            None
        }
    } else {
        Some(MatchKanaKanjiResult::Yes)
    }
}

/// Port of `ichiran/dict:match-sense-restrictions` (`dict.lisp:1515`).
///
/// Tests whether a sense (given its `stagk`/`stagr` restriction tags in
/// `props`) applies to `reading`: true when there are no restrictions or
/// the reading is listed, false when the wrong word-type is restricted
/// away, otherwise a `restricted-readings` lookup paired with
/// [`crate::dict::match_kana_kanji`].
pub fn match_sense_restrictions(
    ctx: &KaniranContext,
    seq: i32,
    props: &[(String, Vec<String>)],
    reading: &KaniWordDispatchEnum,
) -> Result<Option<MatchKanaKanjiResult>, crate::conn::KaniDbError> {
    // dict.lisp:1516-1517 (stagk/stagr (cdr (assoc … props :test 'equal)))
    let stagk: &[String] = props
        .iter()
        .find(|(tag, _)| tag == "stagk")
        .map_or(&[], |(_, texts)| texts.as_slice());
    let stagr: &[String] = props
        .iter()
        .find(|(tag, _)| tag == "stagr")
        .map_or(&[], |(_, texts)| texts.as_slice());
    // dict.lisp:1518 (wtype (word-type reading))
    let wtype = word_type(reading);

    // dict.lisp:1519 ((and (not stagk) (not stagr)) t)
    if stagk.is_empty() && stagr.is_empty() {
        return Ok(Some(MatchKanaKanjiResult::Yes));
    }
    // dict.lisp:1520-1521 ((or (member (text reading) stagk) (member (text reading) stagr)) t)
    let reading_text = text(reading);
    let reading_text = reading_text.as_ref();
    if stagk.iter().any(|t| t.as_str() == reading_text)
        || stagr.iter().any(|t| t.as_str() == reading_text)
    {
        return Ok(Some(MatchKanaKanjiResult::Yes));
    }
    // dict.lisp:1522 ((and (not stagr) (eql wtype :kanji)) nil)
    if stagr.is_empty() && wtype == WordType::Kanji {
        return Ok(None);
    }
    // dict.lisp:1523 ((and (not stagk) (eql wtype :kana)) nil)
    if stagk.is_empty() && wtype == WordType::Kana {
        return Ok(None);
    }
    // dict.lisp:1524 (restricted (query (:select 'reading 'text :from 'restricted-readings :where (:= 'seq seq))))
    let restricted: Vec<(String, String)> = ctx.store.restricted_readings_by_seq(seq)?;
    // dict.lisp:1525-1532 (case wtype …)
    match wtype {
        WordType::Kanji => {
            // dict.lisp:1527 (rkana (select-dao 'kana-text (:and (:= 'seq seq) (:in 'text (:set stagr)))))
            let rkana: Vec<KanaText> = ctx.store.kana_texts_by_seq_and_text_any(seq, stagr)?;
            // dict.lisp:1528 (some (lambda (rk) (match-kana-kanji rk reading restricted)) rkana)
            Ok(rkana.into_iter().find_map(|rk| {
                match_kana_kanji(&KaniWordDispatchEnum::Kana(rk), reading, &restricted)
            }))
        }
        WordType::Kana => {
            // dict.lisp:1530 (rkanji (select-dao 'kanji-text (:and (:= 'seq seq) (:in 'text (:set stagk)))))
            let rkanji: Vec<KanjiText> = ctx
                .store
                .kanji_texts_by_seq_and_text_any(seq, stagk)
                ?;
            // dict.lisp:1531 (some (lambda (rk) (match-kana-kanji reading rk restricted)) rkanji)
            Ok(rkanji.into_iter().find_map(|rk| {
                match_kana_kanji(reading, &KaniWordDispatchEnum::Kanji(rk), &restricted)
            }))
        }
        // (case wtype …) has no :gap clause → nil
        WordType::Gap => Ok(None),
    }
}

/// Port of `ichiran/dict:split-pos` (`dict.lisp:1535`).
///
/// Splits the bracketed part-of-speech string on commas, excluding the
/// enclosing `[` / `]`. Empty subsequences are kept, so `"[]"` yields
/// `[""]`; offsets index by code point.
pub fn split_pos(pos_str: &str) -> Vec<&str> {
    // :start 1 :end (1- (length pos-str)) — code-point offsets.
    let char_count = pos_str.chars().count();
    let start = pos_str
        .char_indices()
        .nth(1)
        .map_or(pos_str.len(), |(byte, _)| byte);
    let end = pos_str
        .char_indices()
        .nth(char_count - 1)
        .map_or(pos_str.len(), |(byte, _)| byte);
    pos_str[start..end].split(',').collect()
}

/// Port of `ichiran/dict:get-senses-json` (`dict.lisp:1537`).
///
/// Builds the per-sense JSON objects (`pos` / `gloss` plus optional
/// `field` and `info`) for an entry, filtering by `pos_list` and, when
/// a reading is supplied, by sense restrictions. The `reading_getter`
/// thunk is awaited at most once across the loop.
pub fn get_senses_json<Getter>(
    ctx: &KaniranContext,
    seq: i32,
    pos_list: &[String],
    reading: Option<KaniWordDispatchEnum>,
    reading_getter: Option<Getter>,
) -> Result<Vec<Value>, crate::conn::KaniDbError>
where
    Getter: FnOnce() -> Result<Option<KaniWordDispatchEnum>, crate::conn::KaniDbError>,
{
    let has_reading_getter = reading_getter.is_some();
    let mut reading_getter = reading_getter;
    let mut reading = reading;
    let mut readp = false;
    let mut rpos = String::new();
    let mut lpos: Vec<String> = Vec::new();
    let mut first = true;
    let mut out: Vec<Value> = Vec::new();

    for (pos, gloss, props) in get_senses(ctx, seq)? {
        let emptypos = pos == "[]";
        // for rpos / lpos = … then (if emptypos … …): first iteration uses
        // the raw value, later iterations keep the prior on an empty pos.
        if first || !emptypos {
            rpos = pos.clone();
            lpos = split_pos(&pos).into_iter().map(str::to_owned).collect();
            first = false;
        }
        let rinf = props
            .iter()
            .find(|(tag, _)| tag == "s_inf")
            .map(|(_, inf)| join("; ", inf));
        let rfield = props
            .iter()
            .find(|(tag, _)| tag == "field")
            .map(|(_, field)| format!("{{{}}}", field.join(",")));

        // (or (not pos-list) (intersection lpos pos-list :test 'equal))
        let cond1 = pos_list.is_empty() || lpos.iter().any(|lp| pos_list.iter().any(|q| q == lp));
        let collect_this = if !cond1 {
            false
        } else if !(has_reading_getter || reading.is_some()) {
            // (not (or reading-getter reading))
            true
        } else if !props
            .iter()
            .any(|(tag, _)| tag == "stagk" || tag == "stagr")
        {
            // (not (or (assoc "stagk" props) (assoc "stagr" props)))
            true
        } else {
            // (let ((rr (or reading (and (not readp) (setf readp t reading (funcall reading-getter)))))) …)
            if reading.is_none() && !readp {
                readp = true;
                reading = match reading_getter.take() {
                    Some(getter) => getter()?,
                    None => None,
                };
            }
            // (if rr (match-sense-restrictions seq props rr) t)
            match &reading {
                Some(rr) => match_sense_restrictions(ctx, seq, &props, rr)
                    ?
                    .is_some(),
                None => true,
            }
        };

        if collect_this {
            let mut js = Map::new();
            js.insert("pos".to_owned(), Value::String(rpos.clone()));
            js.insert("gloss".to_owned(), Value::String(gloss));
            if let Some(rfield) = rfield {
                js.insert("field".to_owned(), Value::String(rfield));
            }
            if let Some(rinf) = rinf {
                js.insert("info".to_owned(), Value::String(rinf));
            }
            out.push(Value::Object(js));
        }
    }
    Ok(out)
}

/// Port of `ichiran/dict:short-sense-str` (`dict.lisp:1562`).
///
/// Returns the joined gloss string of the first sense (lowest `ord`)
/// for `seq`, optionally restricted to senses tagged with the given
/// part of speech.
pub fn short_sense_str(
    ctx: &KaniranContext,
    seq: i32,
    with_pos: Option<&str>,
) -> Result<Option<String>, crate::conn::KaniDbError> {
    // dict.lisp:1562 — `,@(if with-pos …)` splices the sense-prop join
    // only when with-pos is supplied.
    let single: Option<Option<String>> = match with_pos {
        Some(with_pos) => ctx.store.first_sense_gloss_with_pos(seq, with_pos)?,
        None => ctx.store.first_sense_gloss(seq)?,
    };
    Ok(single.flatten())
}

#[cfg(test)]
mod tests;
