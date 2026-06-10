use crate::characters::char_class::get_char_class;
use crate::characters::kani_kana_class::KanaClass;
use crate::conn::kani_context::KaniranContext;
use crate::dict::accessors::{
    get_kana, get_text, true_text, word_conjugations, word_type, WordType,
};
use crate::dict::counters::methods::{seq, value_string};
use crate::dict::dao::{KanaText, KanjiText, WordConjugations};
use crate::dict::kani_word::KaniWordDispatchEnum;
use crate::dict::path::{
    find_best_path, find_word_full, join_substring_words, CounterArg, PathElement, SegmentList,
};
use crate::dict::scoring::score::{gen_score, Segment, SEGMENT_SCORE_CUTOFF};
use crate::dict::text_classes::CompoundText;
use serde_json::{Map, Number, Value};
use std::collections::HashMap;

/// Port of `ichiran/dict:*suffix-map-temp*` (`dict.lisp:1049`).
///
/// Caller-scoped suffix lookup cache: character end-position →
/// `(substr, keyword, kf)` suffix candidates ending there, letting
/// `find-word-suffix` skip recomputing them via `get-suffixes`.
pub type SuffixMapTemp = HashMap<usize, Vec<(String, String, Option<KanaText>)>>;

/// Port of `ichiran/dict:*suffix-next-end*` (`dict.lisp:1050`).
///
/// Caller-scoped current character end-position used as the lookup key
/// into `*suffix-map-temp*`. Signed: the `find-word-suffix` recursion
/// subtracts the suffix length and can go negative, and a negative key
/// simply misses the map.

/// Port of `ichiran/dict:word-info` (`dict.lisp:1245`).
///
/// The runtime descriptor the segmenter produces for each word in a
/// tokenized sentence (a plain CLOS class, not a DAO).
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

/// Port of `ichiran/dict:word-info-json` (`dict.lisp:1262`).
///
/// Serializes every [`WordInfo`] slot into a JSON object. jsown
/// renders CL `nil` as `[]`, so every absent/false slot serializes as
/// an empty array; `:root` serializes as `"ROOT"`.
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

/// Port of `ichiran/dict:simple-word-info` (`dict.lisp:1282`).
///
/// Builds a [`WordInfo`] from the given seq/text/reading/type (with
/// `true_text` = `text`), returned either as the object or its JSON.
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

/// Port of `ichiran/dict:def-reader-for-json` (`dict.lisp:1292`).
///
/// Reads the value at `slot` from a `word-info-json` object, panicking
/// on an absent key like `jsown:val`'s error.
pub fn def_reader_for_json<'a>(obj: &'a Value, slot: &str) -> &'a Value {
    obj.get(slot)
        .unwrap_or_else(|| panic!("jsown:val: key {slot:?} not present in object"))
}

/// Port of `ichiran/dict:word-info-from-segment` (`dict.lisp:1327`).
///
/// Lifts a scored [`Segment`] into a [`WordInfo`], branching on the
/// segment's word: simple-text fills `true_text` / `conjugations`,
/// compound-text fills `components`, counter-text fills `counter`.
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

/// Port of `ichiran/dict:word-info-from-segment-list` (`dict.lisp:1353`).
///
/// Maps [`word_info_from_segment`] across a segment-list and drops
/// scores below `2/3` of the first wi's score, returning either the
/// lone survivor or a synthetic alternative-marked word-info collecting
/// every survivor's kana / seq.
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

/// Port of `ichiran/dict:word-info-from-text` (`dict.lisp:1382`).
///
/// Builds a one-span segment-list over `text` (looking up its full
/// readings and scoring each) and collapses it into a single
/// [`WordInfo`] via [`word_info_from_segment_list`].
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

/// Port of `ichiran/dict:fill-segment-path` (`dict.lisp:1390`).
///
/// Walks a `find-best-path` result and builds the flat [`WordInfo`]
/// sequence: gap-typed word-infos fill the runs between segment-list
/// slices, each segment-list lifts via [`word_info_from_segment_list`],
/// and synergy elements are filtered out. Character offsets are
/// char-indexed, not byte-indexed.
pub async fn fill_segment_path(
    ctx: &KaniranContext,
    str: &str,
    path: &mut [PathElement],
) -> Result<Vec<WordInfo>, sqlx::Error> {
    let str_char_len = str.chars().count();
    let mut idx: usize = 0;
    let mut result: Vec<WordInfo> = Vec::new();

    // dict.lisp:1396-1403 (loop ... for segment-list in path
    //   when (typep segment-list 'segment-list) ...)
    for element in path.iter_mut() {
        let PathElement::SegmentList(sl) = element else {
            continue;
        };
        // dict.lisp:1399-1400 (when start > idx, push gap)
        if sl.start > idx {
            result.push(make_substr_gap(str, idx, sl.start));
        }
        // dict.lisp:1402 (push (word-info-from-segment-list segment-list) result)
        let wi = word_info_from_segment_list(ctx, sl).await?;
        // dict.lisp:1403 (setf idx (segment-list-end segment-list))
        idx = sl.end;
        result.push(wi);
    }

    // dict.lisp:1404-1406 (finally — trailing gap if idx < length)
    if idx < str_char_len {
        result.push(make_substr_gap(str, idx, str_char_len));
    }

    // dict.lisp:1407 (return (process-word-info (nreverse result)))
    // — we built `result` forward, so no nreverse; process_word_info
    //   takes ownership and returns the transformed Vec.
    Ok(process_word_info(result))
}

// dict.lisp:1391-1395 (flet make-substr-gap)
fn make_substr_gap(str: &str, start: usize, end: usize) -> WordInfo {
    // (subseq str start end) — char-indexed in SBCL (CONVENTIONS §4.5)
    let substr: String = str.chars().skip(start).take(end - start).collect();
    WordInfo {
        kind: WordInfoType::Gap,
        text: substr.clone(),
        kana: Some(WordInfoKana::Single(substr)),
        start: Some(start),
        end: Some(end),
        ..Default::default()
    }
}

/// Port of `ichiran/dict:word-info-rec-find` (`dict.lisp:1409`).
///
/// Walks `wi-list` and each word-info's `components` recursively,
/// returning every `(matched, following)` pair where `matched`
/// satisfies `test-fn`. The `following` word-info is the one after
/// `matched` in its list, falling back to the parent's next for a
/// matched last component, or `None` at the very end.
pub fn word_info_rec_find<'a, F>(
    wi_list: &'a [WordInfo],
    test_fn: &F,
) -> Vec<(&'a WordInfo, Option<&'a WordInfo>)>
where
    F: Fn(&WordInfo) -> bool,
{
    let mut result = Vec::new();
    // dict.lisp:1411 (loop for (wi wi-next) on wi-list)
    for (idx, wi) in wi_list.iter().enumerate() {
        let wi_next = wi_list.get(idx + 1);
        // dict.lisp:1412 (for components = (word-info-components wi))
        let components = &wi.components;
        // dict.lisp:1413 (if (funcall test-fn wi) nconc (list (cons wi wi-next)))
        if test_fn(wi) {
            result.push((wi, wi_next));
        }
        // dict.lisp:1414-1415 (nconc (loop for (wf . wf-next) in (word-info-rec-find components test-fn)
        //                                collect (cons wf (or wf-next wi-next))))
        for (wf, wf_next) in word_info_rec_find(components, test_fn) {
            result.push((wf, wf_next.or(wi_next)));
        }
    }
    result
}

/// Port of `ichiran/dict:process-word-info` (`dict.lisp:1417`).
///
/// Post-processes a word-info sequence to fix the 何 (what) reading
/// based on the next word's first kana, picking `"なん"` when every
/// leading kana falls in the dental / voiced / `n` / `r` bracket and
/// `"なに"` otherwise (empty kana strings leave the reading unchanged).
pub fn process_word_info(mut wi_list: Vec<WordInfo>) -> Vec<WordInfo> {
    for i in 0..wi_list.len() {
        if wi_list[i].text != "何" {
            continue;
        }
        let Some(next) = wi_list.get(i + 1) else {
            continue;
        };
        // dict.lisp:1421-1438 — `(unless (listp kn) (setf kn (list kn)))`
        // wraps a non-list `kn` in a singleton; the inner loop then
        // iterates `kn` at one level. `(char kana 0)` errors with a
        // type-error on a non-string element; we mirror that by
        // panicking on a nested `Multi` entry. `None` entries become
        // length-0 and are skipped via `(when (> (length kana) 0) ...)`.
        // Iterate kn at one level. Lisp's `(unless (listp kn) (setf kn (list kn)))`
        // wraps a non-list element into a singleton; equivalent here: a
        // `Single`/`None` slot wraps to a one-element iteration.
        let singleton: Option<WordInfoKana>;
        let kn_iter: &[Option<WordInfoKana>] = match &next.kana {
            Some(WordInfoKana::Multi(items)) => items.as_slice(),
            other => {
                singleton = other.clone();
                std::slice::from_ref(&singleton)
            }
        };
        let mut nani = false;
        let mut nan = false;
        for entry in kn_iter {
            let kana: &str = match entry {
                Some(WordInfoKana::Single(s)) => s.as_str(),
                None => "",
                Some(WordInfoKana::Multi(_)) => {
                    panic!(
                        "process-word-info: nested Multi inside kana list — upstream `(char list 0)` would type-error"
                    );
                }
            };
            let Some(first_char) = kana.chars().next() else {
                continue;
            };
            let fc_class = get_char_class(first_char);
            if matches!(fc_class, Some(c) if is_nan_class(c)) {
                nan = true;
            } else {
                nani = true;
            }
        }
        let nani_kana = match (nan, nani) {
            (true, true) => Some("なに"),
            (true, false) => Some("なん"),
            (false, true) => Some("なに"),
            (false, false) => None,
        };
        if let Some(s) = nani_kana {
            wi_list[i].kana = Some(WordInfoKana::Single(s.to_string()));
        }
    }
    wi_list
}

fn is_nan_class(c: KanaClass) -> bool {
    use KanaClass::*;
    matches!(
        c,
        Ba | Bi
            | Bu
            | Be
            | Bo
            | Pa
            | Pi
            | Pu
            | Pe
            | Po
            | Da
            | Dji
            | Dzu
            | De
            | Do
            | Za
            | Ji
            | Zu
            | Ze
            | Zo
            | Ta
            | Chi
            | Tsu
            | Te
            | To
            | Na
            | Nu
            | Ne
            | No
            | Ra
            | Ri
            | Ru
            | Re
            | Ro
    )
}

/// Port of `ichiran/dict:word-info-reading` (`dict.lisp:1445`).
///
/// Looks up the reading DAO backing a [`WordInfo`]: the first
/// `kanji_text` row for a `:kanji` word-info, the first `kana_text`
/// row for a `:kana` one, matched on `text = true-text`. Returns
/// `None` when the type is `:gap`, `true-text` is nil, or no row
/// matches.
pub async fn word_info_reading(
    ctx: &KaniranContext,
    word_info: &WordInfo,
) -> Result<Option<KaniWordDispatchEnum>, sqlx::Error> {
    // (true-text (word-info-true-text word-info)) — the `(and table true-text)`
    // guard fails outright when true-text is nil.
    let true_text = match &word_info.true_text {
        Some(true_text) => true_text,
        None => return Ok(None),
    };
    // (case (word-info-type word-info) (:kanji 'kanji-text) (:kana 'kana-text))
    // then (car (select-dao table (:= 'text true-text)))
    match word_info.kind {
        WordInfoType::Kanji => {
            let row: Option<KanjiText> = ctx
                .store
                .kanji_texts_by_text(true_text)
                .await?
                .into_iter()
                .next();
            Ok(row.map(KaniWordDispatchEnum::Kanji))
        }
        WordInfoType::Kana => {
            let row: Option<KanaText> = ctx
                .store
                .kana_texts_by_text(true_text)
                .await?
                .into_iter()
                .next();
            Ok(row.map(KaniWordDispatchEnum::Kana))
        }
        // (case …) has no :gap clause → table nil → guard fails → nil.
        WordInfoType::Gap => Ok(None),
    }
}

/// Port of `ichiran/dict:dict-segment` (`dict.lisp:1450`).
///
/// Segments `str` into the top-scoring paths and pairs each resulting
/// word-info list with its score.
pub async fn dict_segment(
    ctx: &KaniranContext,
    str: &str,
    limit: Option<usize>,
) -> Result<Vec<(Vec<WordInfo>, i32)>, sqlx::Error> {
    let limit = limit.unwrap_or(5);

    // (find-best-path (join-substring-words str) (length str) :limit limit)
    let mut segment_lists = join_substring_words(ctx, str).await?;
    let best_paths =
        find_best_path(ctx, &mut segment_lists, str.chars().count(), Some(limit)).await?;

    // (loop for (path . score) in ... collect (cons (fill-segment-path str path) score))
    let mut result = Vec::with_capacity(best_paths.len());
    for (mut path, score) in best_paths {
        let word_info_list = fill_segment_path(ctx, str, &mut path).await?;
        result.push((word_info_list, score));
    }
    Ok(result)
}

/// Port of `ichiran/dict:simple-segment` (`dict.lisp:1455`).
///
/// Returns the word-info list of the best (first) path from
/// [`dict_segment`] (empty when there is no path).
pub async fn simple_segment(
    ctx: &KaniranContext,
    str: &str,
    limit: Option<usize>,
) -> Result<Vec<WordInfo>, sqlx::Error> {
    let limit = limit.unwrap_or(5);
    // (caar (dict-segment str :limit limit))
    let segments = dict_segment(ctx, str, Some(limit)).await?;
    Ok(segments
        .into_iter()
        .next()
        .map(|(word_info_list, _score)| word_info_list)
        .unwrap_or_default())
}

#[cfg(test)]
mod tests;
