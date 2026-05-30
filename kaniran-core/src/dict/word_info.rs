//! Port of the dict.lisp word-info layer — the word-info class +
//! word-info-from-{segment,segment-list,text}, simple-word-info,
//! simplify-reading-list, word-info-json, word-info-gloss-json,
//! def-reader-for-json macro, register-item, score-base, word-type,
//! word-conj-data.

use crate::characters::char_classes::{count_char_class, CharClass};
use crate::conn::kani_context::KaniranContext;
use crate::dict::best_path::SEGMENT_SCORE_CUTOFF;
use crate::dict::best_text::{get_kana, get_text, true_text, word_info_reading};
use crate::dict::calc_score::gen_score;
use crate::dict::conj_data::{
    conj_info_json, get_conj_data, ConjData, FilterPropsText, FromOrConjIds,
};
use crate::dict::counters::dispatchers::{seq, text, value_string, word_conjugations};
use crate::dict::find_word::{find_word_full, CounterArg};
use crate::dict::grammar::suffix_init::get_suffix_description;
use crate::dict::kani::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use crate::dict::segment::{PathElement, Segment, SegmentList, TopArray, TopArrayItem};
use crate::dict::senses::get_senses_json;
use crate::dict::text_classes::{CompoundText, ProxyText, WordConjugations};
use serde_json::{Map, Number, Value};
use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WordInfoType {
    Kanji,
    Kana,
    #[default]
    Gap,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WordInfoKana {
    Single(String),
    Multi(Vec<Option<WordInfoKana>>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WordInfoSeq {
    Single(i32),
    Multi(Vec<Option<WordInfoSeq>>),
}

#[derive(Debug, Clone)]
pub struct WordInfo {
    pub kind: WordInfoType,
    pub text: String,
    pub true_text: Option<String>,
    pub kana: Option<WordInfoKana>,
    pub seq: Option<WordInfoSeq>,
    pub conjugations: Option<WordConjugations>,
    pub score: Option<i32>,
    pub components: Vec<WordInfo>,
    pub alternative: bool,
    pub primary: bool,
    pub start: Option<usize>,
    pub end: Option<usize>,
    pub counter: Option<(String, bool)>,
    pub skipped: i32,
}

impl Default for WordInfo {
    fn default() -> Self {
        // Mirrors the upstream slot `:initform`s:
        //   score :initform 0, primary :initform t, skipped :initform 0.
        // The remaining slots (true_text, kana, seq, conjugations,
        // components, alternative, start, end, counter) initform to nil.
        // `:initform 0` only fires when the `:score` initarg is absent;
        // a caller supplying `:score nil` (e.g. word-info-from-segment
        // with a scoreless segment) overrides via `..Default::default()`.
        Self {
            kind: WordInfoType::default(),
            text: String::new(),
            true_text: None,
            kana: None,
            seq: None,
            conjugations: None,
            score: Some(0),
            components: Vec::new(),
            alternative: false,
            primary: true,
            start: None,
            end: None,
            counter: None,
            skipped: 0,
        }
    }
}

pub async fn word_info_from_segment(
    ctx: &KaniranContext,
    segment: &mut Segment,
) -> Result<WordInfo, sqlx::Error> {
    // dict.lisp:1330 (:text (get-text segment)) — lazy memoization via segment.text
    let text = segment.get_text().to_string();
    // dict.lisp:1347-1348 (:score / :start / :end) — read before re-borrowing word
    let score = segment.score;
    let start = segment.start;
    let end = segment.end;
    let word = &segment.word;

    // dict.lisp:1329 (:type (word-type word))
    let kind = word_info_type_from(word_type(word));

    // dict.lisp:1331 (:kana (get-kana word))
    let kana = get_kana(ctx, word).await?.map(WordInfoKana::Single);

    // dict.lisp:1332 (:seq (seq word))
    let seq_value = seq(word);

    // dict.lisp:1333-1334 (:conjugations / :true-text — gated on simple-text)
    let (true_text_v, conjugations_v) = match word {
        KaniWordDispatchEnum::Kanji(_)
        | KaniWordDispatchEnum::Kana(_)
        | KaniWordDispatchEnum::Proxy(_) => {
            (Some(true_text(word).into_owned()), word_conjugations(word))
        }
        _ => (None, None),
    };

    // dict.lisp:1335-1345 (:components — gated on compound-text)
    let components = if let KaniWordDispatchEnum::Compound(c) = word {
        compound_components(ctx, c).await?
    } else {
        Vec::new()
    };

    // dict.lisp:1346 (:counter — gated on counter-text)
    let counter = if let KaniWordDispatchEnum::Counter(c) = word {
        Some((value_string(c), c.base().ordinalp))
    } else {
        None
    };

    Ok(WordInfo {
        kind,
        text,
        true_text: true_text_v,
        kana,
        seq: seq_value,
        conjugations: conjugations_v,
        score,
        components,
        counter,
        start: Some(start),
        end: Some(end),
        ..Default::default()
    })
}

async fn compound_components(
    ctx: &KaniranContext,
    c: &CompoundText,
) -> Result<Vec<WordInfo>, sqlx::Error> {
    // dict.lisp:1336 (with primary-seq = (seq (primary word))) — bound once.
    // Lisp's `(= int int)` is the only branch that returns a bool; any
    // non-int operand raises TYPE-ERROR. The Rust port panics on the
    // first non-Single encounter to mirror that.
    let primary_seq = match seq(&c.primary) {
        Some(WordInfoSeq::Single(s)) => s,
        other => panic!(
            "compound-text primary seq must be Single int — Lisp `(= … {:?})` would type-error",
            other
        ),
    };
    let mut out = Vec::with_capacity(c.words.len());
    for wrd in &c.words {
        let wrd_seq = seq(wrd);
        let wrd_seq_int = match wrd_seq.as_ref() {
            Some(WordInfoSeq::Single(s)) => *s,
            other => panic!(
                "compound child seq must be Single int — Lisp `(= {:?} {})` would type-error",
                other, primary_seq
            ),
        };
        let child_kana = get_kana(ctx, wrd).await?.map(WordInfoKana::Single);
        out.push(WordInfo {
            // dict.lisp:1339 (:type (word-type wrd))
            kind: word_info_type_from(word_type(wrd)),
            // dict.lisp:1340 (:text (get-text wrd))
            text: get_text(wrd).into_owned(),
            // dict.lisp:1341 (:true-text (true-text wrd))
            true_text: Some(true_text(wrd).into_owned()),
            // dict.lisp:1342 (:kana (get-kana wrd))
            kana: child_kana,
            // dict.lisp:1343 (:seq (seq wrd))
            seq: wrd_seq,
            // dict.lisp:1344 (:conjugations (word-conjugations wrd))
            conjugations: word_conjugations(wrd),
            // dict.lisp:1345 (:primary (= (seq wrd) primary-seq))
            primary: wrd_seq_int == primary_seq,
            ..Default::default()
        });
    }
    Ok(out)
}

fn word_info_type_from(word_type: WordType) -> WordInfoType {
    match word_type {
        WordType::Kanji => WordInfoType::Kanji,
        WordType::Kana => WordInfoType::Kana,
        WordType::Gap => WordInfoType::Gap,
    }
}

pub async fn word_info_from_segment_list(
    ctx: &KaniranContext,
    segment_list: &mut SegmentList,
) -> Result<WordInfo, sqlx::Error> {
    // dict.lisp:1354-1355 ((segments ...) (wi-list* ...)) — map over segments
    let mut wi_list_star: Vec<WordInfo> = Vec::with_capacity(segment_list.segments.len());
    for seg in segment_list.segments.iter_mut() {
        wi_list_star.push(word_info_from_segment(ctx, seg).await?);
    }

    // dict.lisp:1356 (wi1 (car wi-list*)) — bound BEFORE the score filter;
    // every "return wi1 fields" reference below resolves against this.
    let wi1 = wi_list_star
        .first()
        .expect("segment-list has zero segments")
        .clone();
    let matches = segment_list.matches as i32;

    // dict.lisp:1357-1361 (max-score / wi-list = remove-if score < cutoff*max-score)
    // — Lisp `(* 2/3 nil)` and `(< nil _)` both raise TYPE-ERROR; the
    // Rust port panics in the same situation rather than silently
    // substituting 0 (which would change the surviving set).
    let max_int = wi1.score.expect(
        "word-info-from-segment-list: wi1.score is nil — Lisp `(* 2/3 nil)` would type-error",
    ) as i64;
    let (num, den) = SEGMENT_SCORE_CUTOFF;
    let wi_list: Vec<WordInfo> = wi_list_star
        .into_iter()
        .filter(|wi| {
            let s = wi.score.expect(
                "word-info-from-segment-list: wi.score is nil during cutoff filter — Lisp `(< nil _)` would type-error",
            ) as i64;
            den * s >= num * max_int
        })
        .collect();

    // dict.lisp:1363-1365 ((if (= (length wi-list) 1) (prog1 wi1 (setf skipped ...))))
    // — `prog1` returns wi1 (the pre-filter binding); we mutate skipped
    // on the wi1 we already cloned above.
    if wi_list.len() == 1 {
        let mut result = wi1;
        result.skipped = matches - 1;
        return Ok(result);
    }

    // dict.lisp:1366-1380 (multi-branch)
    // dict.lisp:1367-1368 — collect kana / seq per child, position-aligned.
    let kana_list: Vec<Option<WordInfoKana>> = wi_list.iter().map(|wi| wi.kana.clone()).collect();
    let seq_list: Vec<Option<WordInfoSeq>> = wi_list.iter().map(|wi| wi.seq.clone()).collect();

    // dict.lisp:1372 (remove-duplicates kana-list :test 'equal :from-end t)
    let kana_dedup = dedup_keep_first(&kana_list);

    let kept = wi_list.len() as i32;
    Ok(WordInfo {
        // dict.lisp:1370 (:type (word-info-type wi1))
        kind: wi1.kind,
        // dict.lisp:1371 (:text (word-info-text wi1))
        text: wi1.text.clone(),
        kana: Some(WordInfoKana::Multi(kana_dedup)),
        seq: Some(WordInfoSeq::Multi(seq_list)),
        components: wi_list,
        alternative: true,
        // dict.lisp:1376 (:score (word-info-score wi1))
        score: wi1.score,
        start: Some(segment_list.start),
        end: Some(segment_list.end),
        skipped: matches - kept,
        ..Default::default()
    })
}

// dict.lisp:1372 (remove-duplicates kana-list :test 'equal :from-end t)
// — :from-end t keeps the FIRST occurrence in left-to-right order. The
// list is heterogeneous (Single / Multi / None entries), so dedup runs
// on the full Option<WordInfoKana> value via PartialEq.
fn dedup_keep_first(items: &[Option<WordInfoKana>]) -> Vec<Option<WordInfoKana>> {
    let mut out: Vec<Option<WordInfoKana>> = Vec::with_capacity(items.len());
    for item in items {
        if !out.iter().any(|seen| seen == item) {
            out.push(item.clone());
        }
    }
    out
}

pub async fn word_info_from_text(
    ctx: &KaniranContext,
    text: &str,
) -> Result<WordInfo, sqlx::Error> {
    // dict.lisp:1384 (readings (find-word-full text :counter :auto))
    let readings = find_word_full(ctx, text, false, Some(CounterArg::Auto)).await?;
    // dict.lisp:1385 (segments (loop for r in readings collect (gen-score (make-segment …))))
    let text_len = text.chars().count();
    let mut segments: Vec<Segment> = Vec::with_capacity(readings.len());
    for r in readings {
        let mut segment = Segment {
            start: 0,
            end: text_len,
            word: r,
            score: None,
            info: None,
            top: None,
            text: Some(text.to_string()),
        };
        gen_score(ctx, &mut segment, false, &[]).await?;
        segments.push(segment);
    }
    // dict.lisp:1386-1387 (segment-list (make-segment-list :segments segments :start 0 :end (length text) :matches (length segments)))
    let matches = segments.len();
    let mut segment_list = SegmentList {
        segments,
        start: 0,
        end: text_len,
        top: None,
        matches,
    };
    // dict.lisp:1388 (word-info-from-segment-list segment-list)
    word_info_from_segment_list(ctx, &mut segment_list).await
}

/// The `:as` keyword — selects [`simple_word_info`]'s return shape.
#[derive(Debug, Clone, Copy)]
pub enum SimpleWordInfoAs {
    Object,
    Json,
}

/// Tagged return for [`simple_word_info`]: `:object` yields the [`WordInfo`],
/// `:json` its [`word_info_json`] serialization.
#[derive(Debug)]
pub enum KaniSimpleWordInfo {
    Object(WordInfo),
    Json(Value),
}

pub fn simple_word_info(
    seq: Option<WordInfoSeq>,
    text: &str,
    reading: Option<WordInfoKana>,
    kind: WordInfoType,
    as_: SimpleWordInfoAs,
) -> KaniSimpleWordInfo {
    let obj = WordInfo {
        kind,
        text: text.to_owned(),
        true_text: Some(text.to_owned()),
        seq,
        kana: reading,
        ..WordInfo::default()
    };
    match as_ {
        SimpleWordInfoAs::Object => KaniSimpleWordInfo::Object(obj),
        SimpleWordInfoAs::Json => KaniSimpleWordInfo::Json(word_info_json(&obj)),
    }
}

/// Mirrors `(remove-duplicates positions)`. Order is unobservable here
/// (callers either sort the result or use it only for `count`), so this
/// keeps first occurrences.
fn remove_duplicates(positions: &[usize]) -> Vec<usize> {
    let mut out: Vec<usize> = Vec::new();
    for &position in positions {
        if !out.contains(&position) {
            out.push(position);
        }
    }
    out
}

pub fn simplify_reading_list(reading_list: &[String]) -> Vec<String> {
    // assoc entries are `(can cnt . spos)`: de-spaced text, count of
    // readings that mapped to it, and the accumulated boundary positions
    // (each reading's positions concatenated, cross-reading duplicates
    // kept so `count` below can measure agreement). Kept in encounter
    // order (Lisp builds the reverse and `nreverse`s; see the push arm).
    let mut assoc: Vec<(String, i32, Vec<usize>)> = Vec::new();
    for reading in reading_list {
        let mut can = String::new();
        let mut spos: Vec<usize> = Vec::new();
        let mut off: usize = 0;
        for (i, char) in reading.chars().enumerate() {
            if char == ' ' {
                spos.push(i - off);
                off += 1;
            } else {
                can.push(char);
            }
        }
        let spos = remove_duplicates(&spos);
        match assoc.iter_mut().find(|(entry_can, _, _)| *entry_can == can) {
            // dict.lisp:1713 — (setf (cddr a) (nconc spos (cddr a))) (incf (cadr a)).
            Some(entry) => {
                entry.2.extend(spos);
                entry.1 += 1;
            }
            // dict.lisp:1714 — (push (list* can 1 spos) assoc). Lisp pushes
            // onto the front and `nreverse`s at the end; appending here builds
            // the same encounter order directly.
            None => assoc.push((can, 1, spos)),
        }
    }
    let mut out: Vec<String> = Vec::with_capacity(assoc.len());
    for (can, cnt, spos) in &assoc {
        let mut sspos = remove_duplicates(spos);
        sspos.sort_unstable();
        let mut sspos = sspos.into_iter().peekable();
        let mut s = String::new();
        for (i, char) in can.chars().enumerate() {
            if sspos.peek() == Some(&i) {
                let count = spos.iter().filter(|&&position| position == i).count() as i32;
                s.push(if count == *cnt { ' ' } else { '\u{00B7}' });
                sspos.next();
            }
            s.push(char);
        }
        out.push(s);
    }
    out
}

/// jsown renders CL `nil` as `[]`; shared empty-array sentinel.
fn nil() -> Value {
    Value::Array(Vec::new())
}

/// `(symbol-name type)` — the keyword's print name.
fn type_name(t: WordInfoType) -> &'static str {
    match t {
        WordInfoType::Kanji => "KANJI",
        WordInfoType::Kana => "KANA",
        WordInfoType::Gap => "GAP",
    }
}

fn kana_json(kana: &WordInfoKana) -> Value {
    match kana {
        WordInfoKana::Single(s) => Value::String(s.clone()),
        WordInfoKana::Multi(items) => Value::Array(
            items
                .iter()
                .map(|item| item.as_ref().map_or_else(nil, kana_json))
                .collect(),
        ),
    }
}

fn seq_json(seq: &WordInfoSeq) -> Value {
    match seq {
        WordInfoSeq::Single(n) => Value::Number(Number::from(*n)),
        WordInfoSeq::Multi(items) => Value::Array(
            items
                .iter()
                .map(|item| item.as_ref().map_or_else(nil, seq_json))
                .collect(),
        ),
    }
}

pub fn word_info_json(word_info: &WordInfo) -> Value {
    let mut js = Map::new();
    js.insert(
        "type".to_owned(),
        Value::String(type_name(word_info.kind).to_owned()),
    );
    js.insert("text".to_owned(), Value::String(word_info.text.clone()));
    js.insert(
        "truetext".to_owned(),
        word_info
            .true_text
            .as_ref()
            .map_or_else(nil, |t| Value::String(t.clone())),
    );
    js.insert(
        "kana".to_owned(),
        word_info.kana.as_ref().map_or_else(nil, kana_json),
    );
    js.insert(
        "seq".to_owned(),
        word_info.seq.as_ref().map_or_else(nil, seq_json),
    );
    js.insert(
        "conjugations".to_owned(),
        match &word_info.conjugations {
            None => nil(),
            Some(WordConjugations::Root) => Value::String("ROOT".to_owned()),
            Some(WordConjugations::Ids(ids)) => Value::Array(
                ids.iter()
                    .map(|id| Value::Number(Number::from(*id)))
                    .collect(),
            ),
        },
    );
    js.insert(
        "score".to_owned(),
        word_info
            .score
            .map_or_else(nil, |n| Value::Number(Number::from(n))),
    );
    js.insert(
        "components".to_owned(),
        Value::Array(word_info.components.iter().map(word_info_json).collect()),
    );
    js.insert(
        "alternative".to_owned(),
        if word_info.alternative {
            Value::Bool(true)
        } else {
            nil()
        },
    );
    js.insert(
        "primary".to_owned(),
        if word_info.primary {
            Value::Bool(true)
        } else {
            nil()
        },
    );
    js.insert(
        "start".to_owned(),
        word_info
            .start
            .map_or_else(nil, |n| Value::Number(Number::from(n))),
    );
    js.insert(
        "end".to_owned(),
        word_info
            .end
            .map_or_else(nil, |n| Value::Number(Number::from(n))),
    );
    js.insert(
        "counter".to_owned(),
        match &word_info.counter {
            None => nil(),
            Some((value_string, ordinalp)) => Value::Array(vec![
                Value::String(value_string.clone()),
                if *ordinalp { Value::Bool(true) } else { nil() },
            ]),
        },
    );
    js.insert(
        "skipped".to_owned(),
        Value::Number(Number::from(word_info.skipped)),
    );
    Value::Object(js)
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

fn inner<'a>(
    ctx: &'a KaniranContext,
    word_info: &'a WordInfo,
    suffix: bool,
    root_only: bool,
) -> Pin<Box<dyn Future<Output = Result<Value, sqlx::Error>> + 'a>> {
    Box::pin(async move {
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
                components.push(inner(ctx, component, !component.primary, root_only).await?);
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
                    Some(word_info_reading(ctx, word_info)),
                )
                .await?;
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
                // (return-from inner (extend-js js ("gloss" (get-senses-json seq :reading-getter …))))
                let gloss = match seq {
                    Some(seq) => {
                        get_senses_json(
                            ctx,
                            seq,
                            &[],
                            None,
                            Some(word_info_reading(ctx, word_info)),
                        )
                        .await?
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
                            Some(word_info_reading(ctx, word_info)),
                        )
                        .await?;
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
                        .await?;
                js.insert("conj".to_owned(), Value::Array(conj));
            }
        }
        Ok(Value::Object(js))
    })
}

pub async fn word_info_gloss_json(
    ctx: &KaniranContext,
    word_info: &WordInfo,
    root_only: bool,
) -> Result<Value, sqlx::Error> {
    if word_info.alternative {
        // (jsown:new-js ("alternative" (mapcar #'inner (word-info-components word-info))))
        let mut alternatives = Vec::with_capacity(word_info.components.len());
        for component in &word_info.components {
            alternatives.push(inner(ctx, component, false, root_only).await?);
        }
        let mut js = Map::new();
        js.insert("alternative".to_owned(), Value::Array(alternatives));
        Ok(Value::Object(js))
    } else {
        inner(ctx, word_info, false, root_only).await
    }
}

pub fn def_reader_for_json<'a>(obj: &'a Value, slot: &str) -> &'a Value {
    obj.get(slot)
        .unwrap_or_else(|| panic!("jsown:val: key {slot:?} not present in object"))
}

pub fn register_item(obj: &mut TopArray, score: i32, payload: Vec<PathElement>) {
    // dict.lisp:1151 (let ((item (make-top-array-item :score score :payload payload)) (len ...)))
    let mut item: Option<TopArrayItem> = Some(TopArrayItem { score, payload });
    let len = obj.array.len();
    // dict.lisp:1153 (loop for idx from (min count len) downto 0 ...)
    let start = obj.count.min(len);
    let mut idx = start;
    loop {
        // dict.lisp:1154 (for prev-item = (when (> idx 0) (aref array (1- idx))))
        // The slot value is itself an Option (initialize-instance fills with nil);
        // both "idx == 0" and "slot is nil" collapse to None here.
        let prev_score = if idx > 0 {
            obj.array[idx - 1].as_ref().map(|prev| prev.score)
        } else {
            None
        };
        // dict.lisp:1155 (for done = (or (not prev-item) (>= (tai-score prev-item) score)))
        let done = match prev_score {
            None => true,
            Some(prev) => prev >= score,
        };
        // dict.lisp:1156 (when (< idx len) do (setf (aref array idx) (if done item prev-item)))
        if idx < len {
            obj.array[idx] = if done {
                item.take()
            } else {
                obj.array[idx - 1].take()
            };
        }
        // dict.lisp:1157 (until done)
        if done {
            break;
        }
        idx -= 1;
    }
    // dict.lisp:1158 (incf count)
    obj.count += 1;
}

pub fn score_base(word: &CompoundText) -> &KaniWordDispatchEnum {
    word.score_base.as_deref().unwrap_or(&*word.primary)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordType {
    Kanji,
    Kana,
    Gap,
}

pub fn word_type(obj: &KaniWordDispatchEnum) -> WordType {
    match obj {
        KaniWordDispatchEnum::Kanji(_) => WordType::Kanji,
        KaniWordDispatchEnum::Kana(_) => WordType::Kana,
        KaniWordDispatchEnum::Counter(_) => {
            let t = text(obj);
            if count_char_class(&t, CharClass::KanjiChar) > 0 {
                WordType::Kanji
            } else {
                WordType::Kana
            }
        }
        KaniWordDispatchEnum::Proxy(p) => word_type_simple(&p.source),
        KaniWordDispatchEnum::Compound(c) => word_type(&c.primary),
    }
}

fn word_type_simple(obj: &KaniSimpleTextDispatchEnum) -> WordType {
    match obj {
        KaniSimpleTextDispatchEnum::Kanji(_) => WordType::Kanji,
        KaniSimpleTextDispatchEnum::Kana(_) => WordType::Kana,
        KaniSimpleTextDispatchEnum::Proxy(p) => word_type_simple(&p.source),
    }
}

pub async fn word_conj_data(
    ctx: &KaniranContext,
    word: &KaniWordDispatchEnum,
) -> Result<Vec<ConjData>, sqlx::Error> {
    match word {
        // dict-counters.lisp:87 (defmethod word-conj-data ((obj counter-text)) nil)
        KaniWordDispatchEnum::Counter(_) => Ok(Vec::new()),

        // dict.lisp:660-661 (defmethod word-conj-data ((word compound-text)))
        KaniWordDispatchEnum::Compound(c) => {
            let last = c
                .words
                .last()
                .expect("compound-text always has at least one word (adjoin-word ctor)");
            Box::pin(word_conj_data(ctx, last)).await
        }

        // dict.lisp:657-658 (defmethod word-conj-data ((word simple-text)))
        KaniWordDispatchEnum::Kanji(k) => {
            simple_text_arm(ctx, k.seq, &k.state.conjugations, &k.text).await
        }
        KaniWordDispatchEnum::Kana(k) => {
            simple_text_arm(ctx, k.seq, &k.state.conjugations, &k.text).await
        }
        KaniWordDispatchEnum::Proxy(p) => {
            // dict.lisp:657-658 — proxy's seq / word-conjugations / true-text gfs
            // each delegate to (source obj); the leaf is a kanji-text or kana-text
            // and supplies all three slot reads.
            let leaf = leaf_of(p);
            let (seq, conj, text) = leaf_slots(leaf);
            simple_text_arm(ctx, seq, conj, text).await
        }
    }
}

async fn simple_text_arm(
    ctx: &KaniranContext,
    seq: i32,
    conjugations: &Option<WordConjugations>,
    true_text: &str,
) -> Result<Vec<ConjData>, sqlx::Error> {
    let from_or_conj_ids = match conjugations {
        None => FromOrConjIds::All,
        Some(WordConjugations::Root) => FromOrConjIds::Root,
        Some(WordConjugations::Ids(ids)) => FromOrConjIds::ConjIds(ids.clone()),
    };
    get_conj_data(ctx, seq, from_or_conj_ids, &[true_text]).await
}

fn leaf_of(p: &ProxyText) -> &KaniSimpleTextDispatchEnum {
    let mut current: &KaniSimpleTextDispatchEnum = &p.source;
    loop {
        match current {
            KaniSimpleTextDispatchEnum::Proxy(inner) => current = &inner.source,
            _ => return current,
        }
    }
}

fn leaf_slots(leaf: &KaniSimpleTextDispatchEnum) -> (i32, &Option<WordConjugations>, &str) {
    match leaf {
        KaniSimpleTextDispatchEnum::Kanji(k) => (k.seq, &k.state.conjugations, &k.text),
        KaniSimpleTextDispatchEnum::Kana(k) => (k.seq, &k.state.conjugations, &k.text),
        KaniSimpleTextDispatchEnum::Proxy(_) => unreachable!("leaf_of strips proxies"),
    }
}

#[cfg(test)]
mod test_word_info_from_segment {
    //! Unit tests against the real .103 PG via `KaniranContext::from_env()`.
    //! Each test exercises one branch of the segment-word dispatch and
    //! confirms the slot mapping against REPL-captured ground truth.
    use super::*;
    use crate::dict::counters::find_counter::find_counter;
    use crate::dict::find_word::{find_word, FindWordRows};

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    async fn first_reading(ctx: &KaniranContext, word: &str) -> KaniWordDispatchEnum {
        let rows = find_word(ctx, word, false).await.unwrap();
        match rows {
            FindWordRows::Kanji(v) => v
                .into_iter()
                .next()
                .map(KaniWordDispatchEnum::Kanji)
                .unwrap_or_else(|| panic!("no kanji rows for {word:?}")),
            FindWordRows::Kana(v) => v
                .into_iter()
                .next()
                .map(KaniWordDispatchEnum::Kana)
                .unwrap_or_else(|| panic!("no kana rows for {word:?}")),
        }
    }

    fn segment(word: KaniWordDispatchEnum, score: i32, start: usize, end: usize) -> Segment {
        Segment {
            start,
            end,
            word,
            score: Some(score),
            info: None,
            top: None,
            text: None,
        }
    }

    #[tokio::test]
    async fn kana_text_segment_populates_simple_text_slots() {
        // REPL: (find-word-full "ねこ") → KANA-TEXT seq=1467640
        //   word-info-from-segment with score=16, end=2 →
        //   type=KANA text=ねこ kana=ねこ true-text=ねこ primary=T counter=NIL
        let ctx = ctx_from_env().await;
        let word = first_reading(&ctx, "ねこ").await;
        let mut seg = segment(word, 16, 0, 2);
        let wi = word_info_from_segment(&ctx, &mut seg).await.unwrap();
        assert_eq!(wi.kind, WordInfoType::Kana);
        assert_eq!(wi.text, "ねこ");
        assert_eq!(wi.kana, Some(WordInfoKana::Single("ねこ".into())));
        assert_eq!(wi.seq, Some(WordInfoSeq::Single(1467640)));
        assert_eq!(wi.score, Some(16));
        assert_eq!(wi.start, Some(0));
        assert_eq!(wi.end, Some(2));
        assert_eq!(wi.true_text.as_deref(), Some("ねこ"));
        assert!(wi.counter.is_none());
        assert!(wi.primary);
        assert!(!wi.alternative);
        assert_eq!(wi.skipped, 0);
        assert!(wi.components.is_empty());
    }

    #[tokio::test]
    async fn kanji_text_segment_returns_text_and_seq() {
        // KANJI-TEXT branch — get-kana goes through best-kana-conj /
        // get-kanji-kana-old / hint dispatch, all live against the DB.
        let ctx = ctx_from_env().await;
        let word = first_reading(&ctx, "猫").await;
        let mut seg = segment(word, 3, 0, 1);
        let wi = word_info_from_segment(&ctx, &mut seg).await.unwrap();
        assert_eq!(wi.kind, WordInfoType::Kanji);
        assert_eq!(wi.text, "猫");
        assert_eq!(wi.seq, Some(WordInfoSeq::Single(2698030)));
        assert_eq!(wi.score, Some(3));
        assert_eq!(wi.true_text.as_deref(), Some("猫"));
        assert!(wi.counter.is_none());
        assert!(matches!(wi.kana, Some(WordInfoKana::Single(_))));
    }

    #[tokio::test]
    async fn counter_text_segment_populates_counter_pair_and_null_true_text() {
        // REPL: (word-info-from-segment) on a COUNTER-TEXT "5個":
        //   type=KANJI text=5個 counter=("Value: 5" NIL) true-text=NIL
        let ctx = ctx_from_env().await;
        let counter = find_counter(&ctx, "5", "個", None)
            .into_iter()
            .next()
            .expect("find_counter(5, 個) returned no counters");
        let word = KaniWordDispatchEnum::Counter(counter);
        let mut seg = segment(word, 40, 0, 2);
        let wi = word_info_from_segment(&ctx, &mut seg).await.unwrap();
        assert_eq!(wi.kind, WordInfoType::Kanji);
        assert_eq!(wi.text, "5個");
        assert_eq!(wi.counter, Some(("Value: 5".into(), false)));
        assert!(wi.true_text.is_none()); // counter-text is not simple-text
        assert!(wi.conjugations.is_none());
        assert_eq!(wi.score, Some(40));
    }

    #[tokio::test]
    async fn segment_with_no_score_passes_none_through() {
        // The upstream slot is unset (initform 0 only fires when :score
        // initarg is absent — but every callsite supplies :score from
        // (segment-score segment), even if nil).
        let ctx = ctx_from_env().await;
        let word = first_reading(&ctx, "ねこ").await;
        let mut seg = Segment {
            start: 0,
            end: 2,
            word,
            score: None,
            info: None,
            top: None,
            text: None,
        };
        let wi = word_info_from_segment(&ctx, &mut seg).await.unwrap();
        assert_eq!(wi.score, None);
    }

    #[tokio::test]
    async fn compound_text_segment_builds_components_with_primary_flag() {
        // dict.lisp:1335-1345 — compound-text branch:
        //   components = each child word-info with primary set iff
        //   (= (seq wrd) (seq (primary word))).
        use crate::dict::text_classes::{CompoundText, ScoreMod};
        let ctx = ctx_from_env().await;
        let w1 = first_reading(&ctx, "ねこ").await; // seq=1467640
        let w2 = first_reading(&ctx, "いぬ").await; // seq=1258330
        let compound = CompoundText {
            text: "ねこいぬ".into(),
            kana: "ねこいぬ".into(),
            primary: Box::new(w1.clone()),
            words: vec![w1, w2],
            score_base: None,
            score_mod: ScoreMod::Single(0),
        };
        let mut seg = segment(KaniWordDispatchEnum::Compound(compound), 5, 0, 4);
        let wi = word_info_from_segment(&ctx, &mut seg).await.unwrap();
        assert_eq!(wi.text, "ねこいぬ");
        assert_eq!(
            wi.seq,
            Some(WordInfoSeq::Multi(vec![
                Some(WordInfoSeq::Single(1467640)),
                Some(WordInfoSeq::Single(1258330)),
            ]))
        );
        assert_eq!(wi.components.len(), 2);
        assert!(wi.components[0].primary); // matches primary's seq
        assert!(!wi.components[1].primary); // different seq
    }
}

#[cfg(test)]
mod test_word_info_from_segment_list {
    //! Unit tests against the real .103 PG via `KaniranContext::from_env()`.
    //!
    //! Per-branch coverage:
    //! - single-survivor returns `wi1` with `skipped = matches - 1`;
    //! - multi-survivor builds the synthetic with `alternative=true`,
    //!   `score = wi1.score`, `start/end` from the segment-list, and
    //!   per-child `kana` / `seq` preserved (no flattening);
    //! - the score cutoff anchors on `wi1.score` (the FIRST pre-filter
    //!   wi) — even when wi1 itself survives by tie, callers see the
    //!   wi1 binding.
    //! - `dedup_keep_first` keeps first occurrence on a heterogeneous
    //!   `Vec<Option<WordInfoKana>>` (logic-only, no DB).

    use super::*;
    use crate::dict::find_word::{find_word, FindWordRows};
    use crate::dict::kani::KaniWordDispatchEnum;
    use crate::dict::segment::Segment;
    use crate::dict::word_info::WordInfoType;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    async fn one_kana_reading(ctx: &KaniranContext, word: &str) -> KaniWordDispatchEnum {
        let rows = find_word(ctx, word, false).await.unwrap();
        match rows {
            FindWordRows::Kanji(v) => v
                .into_iter()
                .next()
                .map(KaniWordDispatchEnum::Kanji)
                .expect("at least one kanji-text row"),
            FindWordRows::Kana(v) => v
                .into_iter()
                .next()
                .map(KaniWordDispatchEnum::Kana)
                .expect("at least one kana-text row"),
        }
    }

    fn seg(word: KaniWordDispatchEnum, score: i32, start: usize, end: usize) -> Segment {
        Segment {
            start,
            end,
            word,
            score: Some(score),
            info: None,
            top: None,
            text: None,
        }
    }

    fn seg_list(segments: Vec<Segment>, start: usize, end: usize, matches: usize) -> SegmentList {
        SegmentList {
            segments,
            start,
            end,
            top: None,
            matches,
        }
    }

    #[tokio::test]
    async fn single_survivor_returns_wi1_with_skipped() {
        // 1 segment, matches=1 → single branch, skipped = matches - 1 = 0.
        let ctx = ctx_from_env().await;
        let word = one_kana_reading(&ctx, "ねこ").await;
        let mut sl = seg_list(vec![seg(word, 16, 0, 2)], 0, 2, 1);
        let wi = word_info_from_segment_list(&ctx, &mut sl).await.unwrap();
        assert_eq!(wi.kind, WordInfoType::Kana);
        assert_eq!(wi.text, "ねこ");
        assert_eq!(wi.seq, Some(WordInfoSeq::Single(1467640)));
        assert!(!wi.alternative);
        assert_eq!(wi.skipped, 0);
        assert!(wi.components.is_empty());
        assert_eq!(wi.score, Some(16));
        assert_eq!(wi.start, Some(0));
        assert_eq!(wi.end, Some(2));
    }

    #[tokio::test]
    async fn single_survivor_skipped_eq_matches_minus_one() {
        // After cull-segments, matches > len(wi-list)==1 → skipped = matches - 1.
        let ctx = ctx_from_env().await;
        let word = one_kana_reading(&ctx, "ねこ").await;
        let mut sl = seg_list(vec![seg(word, 16, 0, 2)], 0, 2, 7);
        let wi = word_info_from_segment_list(&ctx, &mut sl).await.unwrap();
        assert_eq!(wi.skipped, 6);
    }

    #[tokio::test]
    async fn multi_survivor_builds_synthetic_from_wi1() {
        // Two surviving segments (scores 5, 5; max=5, cutoff = 2*5/3
        // — both pass with cross-mul 3*5 >= 2*5). Synthetic wi.kind /
        // .text / .score all come from wi1 (the first pre-filter wi).
        let ctx = ctx_from_env().await;
        let neko = one_kana_reading(&ctx, "ねこ").await;
        let inu = one_kana_reading(&ctx, "いぬ").await;
        let mut sl = seg_list(vec![seg(neko, 5, 0, 2), seg(inu, 5, 0, 2)], 0, 2, 2);
        let wi = word_info_from_segment_list(&ctx, &mut sl).await.unwrap();
        assert!(wi.alternative);
        assert_eq!(wi.text, "ねこ"); // wi1.text (first segment's wi)
        assert_eq!(wi.score, Some(5));
        assert_eq!(wi.components.len(), 2);
        assert_eq!(wi.start, Some(0));
        assert_eq!(wi.end, Some(2));
        assert_eq!(wi.skipped, 0);
        assert_eq!(
            wi.seq,
            Some(WordInfoSeq::Multi(vec![
                Some(WordInfoSeq::Single(1467640)),
                Some(WordInfoSeq::Single(1258330)),
            ]))
        );
    }

    #[tokio::test]
    async fn multi_branch_filters_below_two_thirds_of_wi1_score() {
        // wi1.score = 9, cutoff = (2*9)/3 = 6. Score-5 and score-3
        // segments fail; only wi1 survives → falls back to single branch.
        let ctx = ctx_from_env().await;
        let a = one_kana_reading(&ctx, "ねこ").await;
        let b = one_kana_reading(&ctx, "いぬ").await;
        let c = one_kana_reading(&ctx, "とり").await;
        let mut sl = seg_list(
            vec![seg(a, 9, 0, 1), seg(b, 5, 0, 1), seg(c, 3, 0, 1)],
            0,
            1,
            3,
        );
        let wi = word_info_from_segment_list(&ctx, &mut sl).await.unwrap();
        assert!(!wi.alternative);
        assert_eq!(wi.text, "ねこ");
        assert_eq!(wi.skipped, 2);
    }

    #[tokio::test]
    async fn multi_branch_anchors_kind_text_score_on_pre_filter_wi1() {
        // Constructed scenario: wi1 has the highest score; second
        // segment also passes the 2/3 cutoff. Confirms wi.text and
        // wi.score follow wi1 even when later survivors have different
        // scores.
        let ctx = ctx_from_env().await;
        let a = one_kana_reading(&ctx, "ねこ").await;
        let b = one_kana_reading(&ctx, "いぬ").await;
        let mut sl = seg_list(vec![seg(a, 9, 0, 1), seg(b, 7, 0, 1)], 0, 1, 2);
        let wi = word_info_from_segment_list(&ctx, &mut sl).await.unwrap();
        assert!(wi.alternative);
        assert_eq!(wi.text, "ねこ"); // wi1.text
        assert_eq!(wi.score, Some(9)); // wi1.score
    }

    #[test]
    fn dedup_keep_first_handles_heterogeneous_options() {
        // remove-duplicates :from-end t keeps first occurrence; the
        // collection is heterogeneous (Single, Multi, None).
        let a = Some(WordInfoKana::Single("a".into()));
        let b = Some(WordInfoKana::Single("b".into()));
        let nested = Some(WordInfoKana::Multi(vec![
            Some(WordInfoKana::Single("x".into())),
            Some(WordInfoKana::Single("y".into())),
        ]));
        let none: Option<WordInfoKana> = None;
        let result = dedup_keep_first(&[
            a.clone(),
            b.clone(),
            a.clone(),
            none.clone(),
            nested.clone(),
            none.clone(),
            nested.clone(),
        ]);
        assert_eq!(result, vec![a, b, none, nested]);
    }
}

#[cfg(test)]
mod test_word_info_from_text {
    //! Unit tests against the real .103 PG via `KaniranContext::from_env()`.
    //!
    //! Every case is a single-survivor result (`skipped = 0`, i.e.
    //! `find-word-full` returned exactly one reading), so the outcome is
    //! independent of `find-word`'s unordered row order. Multi-survivor
    //! inputs (where `wi1` is order-dependent) are exercised
    //! deterministically by `word_info_from_segment_list`'s own tests.
    use super::*;
    use crate::dict::word_info::{WordInfoKana, WordInfoSeq, WordInfoType};

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    /// REPL (.103, word-info-from-text "図書館"): single KANJI reading.
    #[tokio::test]
    async fn simple_kanji_noun() {
        let ctx = ctx_from_env().await;
        let wi = word_info_from_text(&ctx, "図書館").await.unwrap();
        assert_eq!(wi.kind, WordInfoType::Kanji);
        assert_eq!(wi.text, "図書館");
        assert_eq!(wi.kana, Some(WordInfoKana::Single("としょかん".into())));
        assert_eq!(wi.seq, Some(WordInfoSeq::Single(1370420)));
        assert_eq!(wi.score, Some(952));
        assert!(!wi.alternative);
        assert_eq!(wi.skipped, 0);
        assert_eq!(wi.start, Some(0));
        assert_eq!(wi.end, Some(3));
        assert!(wi.components.is_empty());
        assert!(wi.counter.is_none());
        assert_eq!(wi.true_text.as_deref(), Some("図書館"));
        assert!(wi.conjugations.is_none());
        assert!(wi.primary);
    }

    /// REPL (.103, word-info-from-text "オレら"): single KANA reading
    /// (seq 1576880). `end = 3` confirms character (not byte) length.
    #[tokio::test]
    async fn simple_kana_pronoun() {
        let ctx = ctx_from_env().await;
        let wi = word_info_from_text(&ctx, "オレら").await.unwrap();
        assert_eq!(wi.kind, WordInfoType::Kana);
        assert_eq!(wi.text, "オレら");
        assert_eq!(wi.kana, Some(WordInfoKana::Single("オレら".into())));
        assert_eq!(wi.seq, Some(WordInfoSeq::Single(1576880)));
        assert_eq!(wi.score, Some(24));
        assert!(!wi.alternative);
        assert_eq!(wi.end, Some(3));
        assert!(wi.components.is_empty());
        assert_eq!(wi.true_text.as_deref(), Some("オレら"));
        assert!(wi.conjugations.is_none());
        assert!(wi.primary);
    }

    /// REPL (.103, word-info-from-text "食べてる"): single COMPOUND
    /// (食べて + いる) — the suffix-teiru expansion. seq is the
    /// per-child list; two components carry the part readings.
    #[tokio::test]
    async fn compound_teiru() {
        let ctx = ctx_from_env().await;
        let wi = word_info_from_text(&ctx, "食べてる").await.unwrap();
        assert_eq!(wi.kind, WordInfoType::Kanji);
        assert_eq!(wi.text, "食べてる");
        assert_eq!(wi.kana, Some(WordInfoKana::Single("たべてる".into())));
        assert_eq!(
            wi.seq,
            Some(WordInfoSeq::Multi(vec![
                Some(WordInfoSeq::Single(10092233)),
                Some(WordInfoSeq::Single(1577980)),
            ]))
        );
        assert_eq!(wi.score, Some(434));
        assert_eq!(wi.end, Some(4));
        assert_eq!(wi.components.len(), 2);
        assert_eq!(wi.components[0].text, "食べて");
        assert_eq!(wi.components[0].seq, Some(WordInfoSeq::Single(10092233)));
        assert_eq!(wi.components[1].text, "いる");
        assert_eq!(wi.components[1].seq, Some(WordInfoSeq::Single(1577980)));
        // compound-text is not simple-text → true-text / conjugations nil.
        assert!(wi.true_text.is_none());
        assert!(wi.conjugations.is_none());
        assert!(wi.primary);
    }

    /// REPL (.103, word-info-from-text "5万100"): the `:counter :auto`
    /// branch resolves a COUNTER reading — counter pair populated,
    /// `seq` nil. `end = 5` is the character count (byte length is 7).
    #[tokio::test]
    async fn counter_auto_number() {
        let ctx = ctx_from_env().await;
        let wi = word_info_from_text(&ctx, "5万100").await.unwrap();
        assert_eq!(wi.text, "5万100");
        assert_eq!(wi.counter, Some(("Value: 50100".into(), false)));
        assert_eq!(wi.seq, None);
        assert_eq!(wi.score, Some(780));
        assert_eq!(wi.end, Some(5));
        assert!(wi.components.is_empty());
        // counter-text is not simple-text → true-text / conjugations nil.
        assert!(wi.true_text.is_none());
        assert!(wi.conjugations.is_none());
        assert!(wi.primary);
    }

    /// REPL (.103, word-info-from-text "三羽"): `:counter :auto` yields
    /// the 三羽 counter reading (value 3).
    #[tokio::test]
    async fn counter_auto_kanji_number() {
        let ctx = ctx_from_env().await;
        let wi = word_info_from_text(&ctx, "三羽").await.unwrap();
        assert_eq!(wi.text, "三羽");
        assert_eq!(wi.kana, Some(WordInfoKana::Single("さんば".into())));
        assert_eq!(wi.seq, Some(WordInfoSeq::Single(1607310)));
        assert_eq!(wi.counter, Some(("Value: 3".into(), false)));
        assert_eq!(wi.score, Some(286));
        assert_eq!(wi.end, Some(2));
        // counter-text is not simple-text → true-text / conjugations nil.
        assert!(wi.true_text.is_none());
        assert!(wi.conjugations.is_none());
        assert!(wi.primary);
    }
}

#[cfg(test)]
mod test_simple_word_info {
    use super::*;

    /// REPL fixture (.103, `simple-word-info ... :as :json`), 2026-05-25.
    #[test]
    fn simple_word_info_json() {
        let out = simple_word_info(
            Some(WordInfoSeq::Single(1234567)),
            "テスト",
            Some(WordInfoKana::Single("てすと".to_owned())),
            WordInfoType::Kana,
            SimpleWordInfoAs::Json,
        );
        let KaniSimpleWordInfo::Json(v) = out else {
            panic!("expected Json variant");
        };
        assert_eq!(
            serde_json::to_string(&v).unwrap(),
            r#"{"type":"KANA","text":"テスト","truetext":"テスト","kana":"てすと","seq":1234567,"conjugations":[],"score":0,"components":[],"alternative":[],"primary":true,"start":[],"end":[],"counter":[],"skipped":0}"#
        );
    }

    /// `:as :object` (the default) returns the constructed word-info;
    /// `:true-text` mirrors `text` and the unset slots take their initforms.
    #[test]
    fn simple_word_info_object() {
        let out = simple_word_info(
            Some(WordInfoSeq::Single(10092229)),
            "食べた",
            Some(WordInfoKana::Single("たべた".to_owned())),
            WordInfoType::Kanji,
            SimpleWordInfoAs::Object,
        );
        let KaniSimpleWordInfo::Object(wi) = out else {
            panic!("expected Object variant");
        };
        assert_eq!(wi.text, "食べた");
        assert_eq!(wi.true_text.as_deref(), Some("食べた"));
        assert_eq!(wi.kind, WordInfoType::Kanji);
        assert_eq!(wi.score, Some(0));
        assert!(wi.primary);
    }
}

#[cfg(test)]
mod test_simplify_reading_list {
    use super::*;

    fn srl(readings: &[&str]) -> Vec<String> {
        let owned: Vec<String> = readings.iter().map(|reading| reading.to_string()).collect();
        simplify_reading_list(&owned)
    }

    #[test]
    fn simplify_reading_list_fixtures() {
        // REPL fixtures (.103, ichiran/dict::simplify-reading-list), 2026-05-23.
        let cases: &[(&[&str], &[&str])] = &[
            (&[], &[]),
            (&["aru"], &["aru"]),
            (&["tokoro ga"], &["tokoro ga"]),
            // two boundaries, single reading -> both agree -> spaces.
            (&["a b c"], &["a b c"]),
            // consecutive spaces collapse to one boundary (per-reading dedup).
            (&["a  b"], &["a b"]),
            // same de-spaced text, boundary disagrees -> MIDDLE_DOT.
            (&["tokoroga", "tokoro ga"], &["tokoro\u{00B7}ga"]),
            // same de-spaced text, boundary agrees -> space.
            (&["tokoro ga", "tokoro ga"], &["tokoro ga"]),
            // distinct de-spaced texts -> two outputs.
            (&["hito", "kuni"], &["hito", "kuni"]),
            // 2 of 3 readings split at the boundary (count<cnt) -> MIDDLE_DOT.
            (&["a b", "ab", "a b"], &["a\u{00B7}b"]),
            // leading space -> boundary at position 0.
            (&[" ab"], &[" ab"]),
            // trailing space -> boundary at length, never emitted.
            (&["ab "], &["ab"]),
            (&["a b c", "a b c", "a b c"], &["a b c"]),
            // same de-spaced "abc", two different boundaries, neither shared.
            (&["a bc", "ab c"], &["a\u{00B7}b\u{00B7}c"]),
        ];
        for (readings, expected) in cases {
            let actual = srl(readings);
            let expected: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
            assert_eq!(actual, expected, "readings={readings:?}");
        }
    }
}

#[cfg(test)]
mod test_word_info_json {
    use super::*;

    fn single_kana(s: &str) -> Option<WordInfoKana> {
        Some(WordInfoKana::Single(s.to_owned()))
    }

    /// REPL fixtures (.103, `jsown:to-json` of `word-info-json` on
    /// `simple-segment` output), 2026-05-25. serde_json emits raw UTF-8 where
    /// jsown emitted `\uXXXX`; the values/order are identical JSON.
    #[test]
    fn word_info_json_fixtures() {
        // 食べた — plain kanji word: single kana/seq, nil conjugations/counter,
        // primary t, alternative/truetext present.
        let tabeta = WordInfo {
            kind: WordInfoType::Kanji,
            text: "食べた".to_owned(),
            true_text: Some("食べた".to_owned()),
            kana: single_kana("たべた"),
            seq: Some(WordInfoSeq::Single(10092229)),
            score: Some(336),
            start: Some(0),
            end: Some(3),
            ..WordInfo::default()
        };
        assert_eq!(
            serde_json::to_string(&word_info_json(&tabeta)).unwrap(),
            r#"{"type":"KANJI","text":"食べた","truetext":"食べた","kana":"たべた","seq":10092229,"conjugations":[],"score":336,"components":[],"alternative":[],"primary":true,"start":0,"end":3,"counter":[],"skipped":0}"#
        );

        // 5番目 — ordinal counter: counter array [value-string, t], nil truetext.
        let go_banme = WordInfo {
            kind: WordInfoType::Kanji,
            text: "5番目".to_owned(),
            true_text: None,
            kana: single_kana("ごばんめ"),
            seq: Some(WordInfoSeq::Single(1482410)),
            score: Some(667),
            start: Some(0),
            end: Some(3),
            counter: Some(("Value: 5th".to_owned(), true)),
            ..WordInfo::default()
        };
        assert_eq!(
            serde_json::to_string(&word_info_json(&go_banme)).unwrap(),
            r#"{"type":"KANJI","text":"5番目","truetext":[],"kana":"ごばんめ","seq":1482410,"conjugations":[],"score":667,"components":[],"alternative":[],"primary":true,"start":0,"end":3,"counter":["Value: 5th",true],"skipped":0}"#
        );

        // 走っている — compound: Multi seq, recursive components, conjugations as
        // an id list (走って) and the :root sentinel (いる, primary nil→[]),
        // component start/end nil→[].
        let hashitteiru = WordInfo {
            kind: WordInfoType::Kanji,
            text: "走っている".to_owned(),
            true_text: None,
            kana: single_kana("はしっている"),
            seq: Some(WordInfoSeq::Multi(vec![
                Some(WordInfoSeq::Single(10063379)),
                Some(WordInfoSeq::Single(1577980)),
            ])),
            score: Some(406),
            components: vec![
                WordInfo {
                    kind: WordInfoType::Kanji,
                    text: "走って".to_owned(),
                    true_text: Some("走って".to_owned()),
                    kana: single_kana("はしって"),
                    seq: Some(WordInfoSeq::Single(10063379)),
                    conjugations: Some(WordConjugations::Ids(vec![63591])),
                    score: Some(0),
                    start: None,
                    end: None,
                    ..WordInfo::default()
                },
                WordInfo {
                    kind: WordInfoType::Kana,
                    text: "いる".to_owned(),
                    true_text: Some("いる".to_owned()),
                    kana: single_kana("いる"),
                    seq: Some(WordInfoSeq::Single(1577980)),
                    conjugations: Some(WordConjugations::Root),
                    score: Some(0),
                    primary: false,
                    start: None,
                    end: None,
                    ..WordInfo::default()
                },
            ],
            start: Some(0),
            end: Some(5),
            ..WordInfo::default()
        };
        assert_eq!(
            serde_json::to_string(&word_info_json(&hashitteiru)).unwrap(),
            r#"{"type":"KANJI","text":"走っている","truetext":[],"kana":"はしっている","seq":[10063379,1577980],"conjugations":[],"score":406,"components":[{"type":"KANJI","text":"走って","truetext":"走って","kana":"はしって","seq":10063379,"conjugations":[63591],"score":0,"components":[],"alternative":[],"primary":true,"start":[],"end":[],"counter":[],"skipped":0},{"type":"KANA","text":"いる","truetext":"いる","kana":"いる","seq":1577980,"conjugations":"ROOT","score":0,"components":[],"alternative":[],"primary":[],"start":[],"end":[],"counter":[],"skipped":0}],"alternative":[],"primary":true,"start":0,"end":5,"counter":[],"skipped":0}"#
        );

        // 何 — alternative branch: Multi kana (string list), Multi seq (int list),
        // alternative t, two components.
        let nani = WordInfo {
            kind: WordInfoType::Kanji,
            text: "何".to_owned(),
            true_text: None,
            kana: Some(WordInfoKana::Multi(vec![
                Some(WordInfoKana::Single("なに".to_owned())),
                Some(WordInfoKana::Single("なん".to_owned())),
            ])),
            seq: Some(WordInfoSeq::Multi(vec![
                Some(WordInfoSeq::Single(1577100)),
                Some(WordInfoSeq::Single(2846738)),
            ])),
            score: Some(24),
            components: vec![
                WordInfo {
                    kind: WordInfoType::Kanji,
                    text: "何".to_owned(),
                    true_text: Some("何".to_owned()),
                    kana: single_kana("なに"),
                    seq: Some(WordInfoSeq::Single(1577100)),
                    score: Some(24),
                    start: Some(0),
                    end: Some(1),
                    ..WordInfo::default()
                },
                WordInfo {
                    kind: WordInfoType::Kanji,
                    text: "何".to_owned(),
                    true_text: Some("何".to_owned()),
                    kana: single_kana("なん"),
                    seq: Some(WordInfoSeq::Single(2846738)),
                    score: Some(16),
                    start: Some(0),
                    end: Some(1),
                    ..WordInfo::default()
                },
            ],
            alternative: true,
            start: Some(0),
            end: Some(1),
            ..WordInfo::default()
        };
        assert_eq!(
            serde_json::to_string(&word_info_json(&nani)).unwrap(),
            r#"{"type":"KANJI","text":"何","truetext":[],"kana":["なに","なん"],"seq":[1577100,2846738],"conjugations":[],"score":24,"components":[{"type":"KANJI","text":"何","truetext":"何","kana":"なに","seq":1577100,"conjugations":[],"score":24,"components":[],"alternative":[],"primary":true,"start":0,"end":1,"counter":[],"skipped":0},{"type":"KANJI","text":"何","truetext":"何","kana":"なん","seq":2846738,"conjugations":[],"score":16,"components":[],"alternative":[],"primary":true,"start":0,"end":1,"counter":[],"skipped":0}],"alternative":true,"primary":true,"start":0,"end":1,"counter":[],"skipped":0}"#
        );
    }
}

#[cfg(test)]
mod test_word_info_gloss_json {
    //! Ground truth captured from `(jsown:to-json (word-info-gloss-json …))`
    //! and `find-word-info-json` on .103 (2026-05-25) after `(init-suffixes t t)`.
    //! jsown emits `\uXXXX`; the expected strings below are the
    //! raw-UTF-8 round-trip serde_json produces (identical JSON). Tests run
    //! against the local DB per project policy.
    use super::*;
    use crate::dict::find_word::find_word_info;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    fn json(value: &Value) -> String {
        serde_json::to_string(value).unwrap()
    }

    /// Drives word-info-gloss-json through find-word-info (non-root). One row
    /// per cond branch: simple noun (`(t)` arm, top-level gloss + has-gloss →
    /// empty conj), conjugated verb (conjs is a list → no top-level gloss; conj
    /// carries it), ordinal counter (counter object, `ordinal:true`), and
    /// compound (compound list + recursive components, the non-primary いる
    /// reaching the suffix arm).
    #[tokio::test]
    async fn word_info_gloss_json_branches() {
        let ctx = ctx_from_env().await;
        // (text, expected single-object json)
        let cases: &[(&str, &str)] = &[
            // simple noun — no conjs → top-level gloss, has-gloss → empty conj.
            (
                "政府",
                r#"{"reading":"政府 【せいふ】","text":"政府","kana":"せいふ","score":325,"seq":1376070,"gloss":[{"pos":"[n]","gloss":"government; administration; ministry"}],"conj":[]}"#,
            ),
            // conjugated verb — conjs is a list (not nil/:root) → no top-level
            // gloss; conj-info-json (has-gloss nil) carries the gloss.
            (
                "書いた",
                r#"{"reading":"書いた 【かいた】","text":"書いた","kana":"かいた","score":336,"seq":10526928,"conj":[{"prop":[{"pos":"v5k","type":"Past (~ta)"}],"reading":"書く 【かく】","gloss":[{"pos":"[v5k,vt]","gloss":"to write; to compose; to pen"},{"pos":"[vt,v5k]","gloss":"to draw; to paint"}],"readok":true}]}"#,
            ),
            // ordinal counter — counter object with ordinal:true.
            (
                "5番目",
                r#"{"reading":"5番目 【ごばんめ】","text":"5番目","kana":"ごばんめ","score":667,"counter":{"value":"Value: 5th","ordinal":true},"seq":1482410,"gloss":[{"pos":"[ctr]","gloss":"the nth ...","info":"indicates position in a sequence"}]}"#,
            ),
            // compound — compound list + components, the non-primary いる
            // component reaches the suffix arm (suffix t, has a description).
            (
                "食べてる",
                r#"{"reading":"食べてる 【たべてる】","text":"食べてる","kana":"たべてる","score":434,"compound":["食べて","いる"],"components":[{"reading":"食べて 【たべて】","text":"食べて","kana":"たべて","score":0,"seq":10092233,"conj":[{"prop":[{"pos":"v1","type":"Conjunctive (~te)"}],"reading":"食べる 【たべる】","gloss":[{"pos":"[v1,vt]","gloss":"to eat"},{"pos":"[vt,v1]","gloss":"to live on (e.g. a salary); to live off; to subsist on"}],"readok":true}]},{"reading":"いる","text":"いる","kana":"いる","score":0,"seq":1577980,"suffix":"indicates continuing action (to be ...ing)","conj":[]}]}"#,
            ),
        ];
        for (text, expected) in cases {
            let wis = find_word_info(&ctx, text, None, false).await.unwrap();
            assert!(!wis.is_empty(), "text={text}");
            let result = word_info_gloss_json(&ctx, &wis[0], false).await.unwrap();
            assert_eq!(json(&result), *expected, "text={text}");
        }
    }

    /// `root_only=true` on a non-root word-info: the `(t)` arm adds gloss
    /// unconditionally and returns before the conj key. 政府 yields a
    /// populated gloss; 書いた's conjugated seq has no direct senses, so the
    /// gloss is the empty list (still emitted, unlike the guarded non-root arm).
    #[tokio::test]
    async fn root_only_t_arm() {
        let ctx = ctx_from_env().await;
        let cases: &[(&str, &str)] = &[
            (
                "政府",
                r#"{"reading":"政府 【せいふ】","text":"政府","kana":"せいふ","score":325,"seq":1376070,"gloss":[{"pos":"[n]","gloss":"government; administration; ministry"}]}"#,
            ),
            (
                "書いた",
                r#"{"reading":"書いた 【かいた】","text":"書いた","kana":"かいた","score":336,"seq":10526928,"gloss":[]}"#,
            ),
        ];
        for (text, expected) in cases {
            let wis = find_word_info(&ctx, text, None, false).await.unwrap();
            assert!(!wis.is_empty(), "text={text}");
            let result = word_info_gloss_json(&ctx, &wis[0], true).await.unwrap();
            assert_eq!(json(&result), *expected, "text={text}");
        }
    }

    /// 5個 → two counter readings (ごこ / ごか); each carries a non-ordinal
    /// counter object (`ordinal:[]`) and a pos-list `("ctr")`-filtered gloss.
    #[tokio::test]
    async fn counter_two_readings() {
        let ctx = ctx_from_env().await;
        let expected = [
            r#"{"reading":"5個 【ごこ】","text":"5個","kana":"ごこ","score":128,"counter":{"value":"Value: 5","ordinal":[]},"seq":1264740,"gloss":[{"pos":"[ctr]","gloss":"counter for (small) things or pieces","info":"also written as ヶ"},{"pos":"[ctr]","gloss":"counter for military units"}]}"#,
            r#"{"reading":"5個 【ごか】","text":"5個","kana":"ごか","score":40,"counter":{"value":"Value: 5","ordinal":[]},"seq":2220320,"gloss":[{"pos":"[ctr]","gloss":"counter used with Sino-Japanese words","info":"precedes the thing being counted; e.g. ３か月, ５か所; also written as ヶ or ケ"}]}"#,
        ];
        let wis = find_word_info(&ctx, "5個", None, false).await.unwrap();
        assert_eq!(wis.len(), 2);
        for (wi, expected) in wis.iter().zip(expected.iter()) {
            let result = word_info_gloss_json(&ctx, wi, false).await.unwrap();
            assert_eq!(json(&result), *expected);
        }
    }

    /// Alternative branch: `{"alternative": [inner(c) …]}`. Built from a real
    /// alternative word-info (the segmenter shape: alternative=t, components =
    /// the surviving word-infos). find-word-info never produces alternatives,
    /// so the components are assembled from its 何 (なに / なん) word-infos.
    #[tokio::test]
    async fn alternative_branch() {
        let ctx = ctx_from_env().await;
        let components = find_word_info(&ctx, "何", None, false).await.unwrap();
        assert_eq!(components.len(), 2);
        let alt = WordInfo {
            kind: crate::dict::word_info::WordInfoType::Kanji,
            text: "何".to_owned(),
            alternative: true,
            components,
            start: Some(0),
            end: Some(1),
            ..WordInfo::default()
        };
        let result = word_info_gloss_json(&ctx, &alt, false).await.unwrap();
        let expected = r#"{"alternative":[{"reading":"何 【なに】","text":"何","kana":"なに","score":24,"seq":1577100,"gloss":[{"pos":"[pn]","gloss":"what"},{"pos":"[pn]","gloss":"you-know-what; that thing"},{"pos":"[pn]","gloss":"whatsit; whachamacallit; what's-his-name; what's-her-name"},{"pos":"[n]","gloss":"penis; (one's) thing; dick","info":"esp. ナニ"},{"pos":"[adv]","gloss":"(not) at all; (not) in the slightest","info":"with neg. sentence"},{"pos":"[int]","gloss":"what?; huh?","info":"indicates surprise"},{"pos":"[int]","gloss":"hey!; come on!","info":"indicates anger or irritability"},{"pos":"[int]","gloss":"oh, no (it's fine); why (it's nothing); oh (certainly not)","info":"used to dismiss someone's worries, concerns, etc."}],"conj":[]},{"reading":"何 【なん】","text":"何","kana":"なん","score":16,"seq":2846738,"gloss":[{"pos":"[pn]","gloss":"what"},{"pos":"[pref]","gloss":"how many","info":"followed by a counter"},{"pos":"[pref]","gloss":"many; a lot of","info":"followed by (optional number), counter and も"},{"pos":"[pref]","gloss":"several; a few; some","info":"followed by a counter and か"}],"conj":[]}]}"#;
        assert_eq!(json(&result), expected);
    }
}

#[cfg(test)]
mod test_def_reader_for_json_macro {
    use super::*;
    use serde_json::json;

    // REPL fixtures (.103, `(jsown:val (word-info-json (word-info-from-text W)) slot)`
    // after `(init-suffixes t t)`), 2026-05-25. jsown renders CL nil as `[]`, so the
    // serde_json object `word-info-json` builds matches the captured shapes; each
    // generated reader returns the value pinned here.

    #[test]
    fn reads_each_slot() {
        // 薔薇 — single-seq noun: scalar slots, every nil slot serialized as [].
        let bara = json!({
            "type":"KANJI","text":"薔薇","truetext":"薔薇","kana":"ばら",
            "seq":1571760,"conjugations":[],"score":143,"components":[],
            "alternative":[],"primary":true,"start":0,"end":2,"counter":[],"skipped":0
        });
        let cases: &[(&str, Value)] = &[
            ("text", json!("薔薇")),
            ("truetext", json!("薔薇")),
            ("kana", json!("ばら")),
            ("seq", json!(1571760)),
            ("score", json!(143)),
            ("components", json!([])),
            ("alternative", json!([])),
            ("primary", json!(true)),
            ("start", json!(0)),
            ("end", json!(2)),
            ("counter", json!([])),
            ("skipped", json!(0)),
        ];
        for &(slot, ref expected) in cases {
            assert_eq!(def_reader_for_json(&bara, slot), expected, "slot={slot}");
        }
    }

    #[test]
    fn reads_multi_value_slots() {
        // 一人 — alternative reading: kana/seq are arrays, components is the
        // two-child array (counter on the second child), alternative t,
        // truetext nil→[].
        let hitori = json!({
            "type":"KANJI","text":"一人","truetext":[],"kana":["ひとり"],
            "seq":[1576150,2149890],"conjugations":[],"score":312,
            "components":[
                {"type":"KANJI","text":"一人","truetext":"一人","kana":"ひとり","seq":1576150,
                 "conjugations":[],"score":312,"components":[],"alternative":[],"primary":true,
                 "start":0,"end":2,"counter":[],"skipped":0},
                {"type":"KANJI","text":"一人","truetext":[],"kana":"ひとり","seq":2149890,
                 "conjugations":[],"score":208,"components":[],"alternative":[],"primary":true,
                 "start":0,"end":2,"counter":["Value: 1",[]],"skipped":0}
            ],
            "alternative":true,"primary":true,"start":0,"end":2,"counter":[],"skipped":0
        });
        assert_eq!(def_reader_for_json(&hitori, "kana"), &json!(["ひとり"]));
        assert_eq!(
            def_reader_for_json(&hitori, "seq"),
            &json!([1576150, 2149890])
        );
        assert_eq!(def_reader_for_json(&hitori, "alternative"), &json!(true));
        assert_eq!(def_reader_for_json(&hitori, "truetext"), &json!([]));
        let components = def_reader_for_json(&hitori, "components");
        let second_child = &components.as_array().expect("components is an array")[1];
        assert_eq!(
            def_reader_for_json(second_child, "counter"),
            &json!(["Value: 1", []])
        );
    }

    #[test]
    #[should_panic(expected = "not present")]
    fn panics_on_missing_key() {
        // jsown:val errors on an absent key.
        let obj = json!({"text": "x"});
        def_reader_for_json(&obj, "nonexistent");
    }
}

#[cfg(test)]
mod test_register_item {
    use super::*;
    use crate::dict::segment::SegmentList;

    fn payload(tag: i32) -> Vec<PathElement> {
        // Use a sentinel SegmentList with `matches` carrying the tag —
        // tests only need a discriminable payload, not real data.
        vec![PathElement::SegmentList(SegmentList {
            segments: vec![],
            start: 0,
            end: 0,
            top: None,
            matches: tag as usize,
        })]
    }

    fn tag_of(item: &TopArrayItem) -> usize {
        match &item.payload[0] {
            PathElement::SegmentList(sl) => sl.matches,
            _ => panic!("unexpected payload variant"),
        }
    }

    fn scores_and_tags(obj: &TopArray) -> Vec<(i32, usize)> {
        obj.array
            .iter()
            .filter_map(|slot| slot.as_ref().map(|item| (item.score, tag_of(item))))
            .collect()
    }

    // REPL probes (`/tmp/probe_register_item.lisp` on .103, 2026-05-19).
    // Tags below correspond to the Lisp keyword payloads :A=1, :B=2,
    // :C=3, :Z=99 — only the structural relationships matter.

    #[test]
    fn a_empty_register_lands_at_index_0() {
        // REPL A: empty limit=3, register score=10 payload=(A).
        // count=1, [0]=(10,A), [1..]=NIL.
        let mut ta = TopArray::new(3);
        register_item(&mut ta, 10, payload(1));
        assert_eq!(ta.count, 1);
        assert_eq!(scores_and_tags(&ta), vec![(10, 1)]);
        assert!(ta.array[1].is_none());
        assert!(ta.array[2].is_none());
    }

    #[test]
    fn b_higher_score_shifts_existing_down() {
        // REPL B: register 10/A then 20/B with limit=3.
        // count=2, [0]=(20,B), [1]=(10,A).
        let mut ta = TopArray::new(3);
        register_item(&mut ta, 10, payload(1));
        register_item(&mut ta, 20, payload(2));
        assert_eq!(ta.count, 2);
        assert_eq!(scores_and_tags(&ta), vec![(20, 2), (10, 1)]);
    }

    #[test]
    fn c_lower_score_appends_after() {
        // REPL C: register 20/A then 10/B with limit=3.
        // count=2, [0]=(20,A), [1]=(10,B).
        let mut ta = TopArray::new(3);
        register_item(&mut ta, 20, payload(1));
        register_item(&mut ta, 10, payload(2));
        assert_eq!(ta.count, 2);
        assert_eq!(scores_and_tags(&ta), vec![(20, 1), (10, 2)]);
    }

    #[test]
    fn d_equal_score_new_lands_below_existing() {
        // REPL D: register 20/A then 20/B — the `>=` test stops descent
        // at the equal slot, new item lands one index lower.
        // count=2, [0]=(20,A), [1]=(20,B).
        let mut ta = TopArray::new(3);
        register_item(&mut ta, 20, payload(1));
        register_item(&mut ta, 20, payload(2));
        assert_eq!(ta.count, 2);
        assert_eq!(scores_and_tags(&ta), vec![(20, 1), (20, 2)]);
    }

    #[test]
    fn e_middle_insert_shifts_lower_down() {
        // REPL E: limit=5, register 30/C, 10/A, 20/B.
        // count=3, [0]=(30,C), [1]=(20,B), [2]=(10,A), [3]=NIL, [4]=NIL.
        let mut ta = TopArray::new(5);
        register_item(&mut ta, 30, payload(3));
        register_item(&mut ta, 10, payload(1));
        register_item(&mut ta, 20, payload(2));
        assert_eq!(ta.count, 3);
        assert_eq!(scores_and_tags(&ta), vec![(30, 3), (20, 2), (10, 1)]);
        assert!(ta.array[3].is_none());
        assert!(ta.array[4].is_none());
    }

    #[test]
    fn f_overflow_at_bottom_is_dropped_silently() {
        // REPL F: limit=3, register 30/C, 20/B, 10/A, 5/Z.
        // 5/Z's idx starts at min(3,3)=3 which fails the `(< idx len)`
        // guard; done is true (prev>=5) → break with no write.
        // count=4, [0..2]=(30,C),(20,B),(10,A); Z dropped.
        let mut ta = TopArray::new(3);
        register_item(&mut ta, 30, payload(3));
        register_item(&mut ta, 20, payload(2));
        register_item(&mut ta, 10, payload(1));
        register_item(&mut ta, 5, payload(99));
        assert_eq!(ta.count, 4);
        assert_eq!(scores_and_tags(&ta), vec![(30, 3), (20, 2), (10, 1)]);
    }

    #[test]
    fn g_full_array_highest_replaces_top_evicts_lowest() {
        // REPL G: limit=3, register 30/C, 20/B, 10/A, then 99/Z.
        // 99/Z walks down through all three; the lowest (10/A) falls off
        // the bottom because idx=3 skip-writes (no out-of-bounds).
        // count=4, [0]=(99,Z), [1]=(30,C), [2]=(20,B).
        let mut ta = TopArray::new(3);
        register_item(&mut ta, 30, payload(3));
        register_item(&mut ta, 20, payload(2));
        register_item(&mut ta, 10, payload(1));
        register_item(&mut ta, 99, payload(99));
        assert_eq!(ta.count, 4);
        assert_eq!(scores_and_tags(&ta), vec![(99, 99), (30, 3), (20, 2)]);
    }

    #[test]
    fn h_limit_one() {
        // REPL H: limit=1. After register 5/A: count=1, [0]=(5,A).
        // After register 10/B: B shifts A out → count=2, [0]=(10,B).
        // After register 3/C: 3 < 10, idx=1 skip-writes, done → count=3,
        // [0] still (10,B).
        let mut ta = TopArray::new(1);
        register_item(&mut ta, 5, payload(1));
        assert_eq!(ta.count, 1);
        assert_eq!(scores_and_tags(&ta), vec![(5, 1)]);
        register_item(&mut ta, 10, payload(2));
        assert_eq!(ta.count, 2);
        assert_eq!(scores_and_tags(&ta), vec![(10, 2)]);
        register_item(&mut ta, 3, payload(3));
        assert_eq!(ta.count, 3);
        assert_eq!(scores_and_tags(&ta), vec![(10, 2)]);
    }

    #[test]
    fn i_limit_zero_increments_count_only() {
        // REPL I: limit=0. No slot can be written; count still increments.
        // count=1, array empty.
        let mut ta = TopArray::new(0);
        register_item(&mut ta, 5, payload(1));
        assert_eq!(ta.count, 1);
        assert_eq!(ta.array.len(), 0);
    }

    #[test]
    fn j_empty_payload() {
        // REPL J: find-best-path's initial call `(register-item top
        // (gap-penalty 0 str-length) nil)` — empty payload is the
        // structural zero element.
        let mut ta = TopArray::new(3);
        register_item(&mut ta, -1000, vec![]);
        assert_eq!(ta.count, 1);
        let item = ta.array[0].as_ref().unwrap();
        assert_eq!(item.score, -1000);
        assert!(item.payload.is_empty());
    }
}

#[cfg(test)]
mod test_word_type {
    use super::*;
    use crate::dict::counters::classes::{Common, Counter, CounterSource, CounterText};
    use crate::dict::dao::{KanaText, KanjiText};
    use crate::dict::text_classes::{CompoundText, ProxyText, ScoreMod, SimpleText};

    fn kanji(seq: i32, text: &str) -> KanjiText {
        KanjiText {
            id: 0,
            seq,
            text: text.into(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: true,
            nokanji: false,
            best_kana: None,
            state: SimpleText::default(),
        }
    }

    fn kana(seq: i32, text: &str) -> KanaText {
        KanaText {
            id: 0,
            seq,
            text: text.into(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        }
    }

    fn counter(number_text: &str, text: &str, source: Option<CounterSource>) -> Counter {
        Counter::Base(CounterText {
            text: text.into(),
            kana: String::new(),
            number_text: number_text.into(),
            number: 0,
            source,
            ordinalp: false,
            suffix: None,
            accepts_suffixes: Vec::new(),
            suffix_descriptions: Vec::new(),
            digit_opts: Vec::new(),
            common: Common::Inherit,
            allowed: Vec::new(),
            foreign: false,
        })
    }

    #[test]
    fn kanji_variant_is_kanji() {
        assert_eq!(
            word_type(&KaniWordDispatchEnum::Kanji(kanji(1, "犬"))),
            WordType::Kanji,
        );
    }

    #[test]
    fn kana_variant_is_kana() {
        assert_eq!(
            word_type(&KaniWordDispatchEnum::Kana(kana(1, "いぬ"))),
            WordType::Kana,
        );
    }

    #[test]
    fn counter_with_kanji_in_concatenated_text() {
        // `text` gf concatenates "1" + "人" → "1人"; contains kanji.
        let c = counter("1", "人", None);
        assert_eq!(
            word_type(&KaniWordDispatchEnum::Counter(c)),
            WordType::Kanji,
        );
    }

    #[test]
    fn counter_with_only_kana_text() {
        // "1" + "つ" → "1つ"; no kanji.
        let c = counter("1", "つ", None);
        assert_eq!(word_type(&KaniWordDispatchEnum::Counter(c)), WordType::Kana,);
    }

    #[test]
    fn proxy_recurses_through_source_chain() {
        let leaf = KaniSimpleTextDispatchEnum::Kana(kana(1, "ねこ"));
        let inner = KaniSimpleTextDispatchEnum::Proxy(ProxyText {
            text: String::new(),
            kana: String::new(),
            source: Box::new(leaf),
            state: SimpleText::default(),
        });
        let outer = ProxyText {
            text: String::new(),
            kana: String::new(),
            source: Box::new(inner),
            state: SimpleText::default(),
        };
        assert_eq!(
            word_type(&KaniWordDispatchEnum::Proxy(outer)),
            WordType::Kana,
        );
    }

    #[test]
    fn compound_dispatches_on_primary() {
        let primary = Box::new(KaniWordDispatchEnum::Kanji(kanji(1, "犬")));
        let words = vec![
            KaniWordDispatchEnum::Kanji(kanji(1, "犬")),
            KaniWordDispatchEnum::Kana(kana(2, "ねこ")),
        ];
        let c = CompoundText {
            text: String::new(),
            kana: String::new(),
            primary,
            words,
            score_base: None,
            score_mod: ScoreMod::Single(0),
        };
        assert_eq!(
            word_type(&KaniWordDispatchEnum::Compound(c)),
            WordType::Kanji,
        );
    }
}
