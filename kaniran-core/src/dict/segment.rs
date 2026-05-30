//! Port of the dict.lisp segment / top-array layer — segment +
//! segment-list + top-array structs, length-multiplier(-coeff),
//! kanji-break-penalty, gap-penalty, find-sticky-positions,
//! make-slice / subseq-slice, compare-common, cull-segments,
//! get-seg-initial / -splits, expand-segment-list,
//! get-segment-score, dict-segment, simple-segment.

use crate::characters::kana_class::{
    get_char_class, long_vowel_modifier_p, KanaClass, ITERATION_CHARACTERS, KANA_CHARACTERS,
    MODIFIER_CHARACTERS,
};
use crate::characters::text_utils::mora_length;
use crate::conn::kani_context::KaniranContext;
use crate::dict::best_path::{
    fill_segment_path, find_best_path, join_substring_words, IDENTICAL_WORD_SCORE_CUTOFF,
    SCORE_CUTOFF,
};
use crate::dict::calc_score::calc_score;
use crate::dict::conj_data::ConjData;
use crate::dict::errata::NO_KANJI_BREAK_PENALTY;
use crate::dict::grammar::penalty::get_penalties;
use crate::dict::grammar::segfilter::apply_segfilters;
use crate::dict::grammar::suffix_lookup::get_suffixes;
use crate::dict::grammar::synergy::{get_synergies, Synergy};
use crate::dict::kani::{KaniLitePathElement, KaniLiteSegmentList, KaniWordDispatchEnum};
use crate::dict::split::segsplit::get_segsplit;
use crate::dict::text_classes::ScoreMod;
use crate::dict::word_info::WordInfo;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Segment {
    pub start: usize,
    pub end: usize,
    pub word: KaniWordDispatchEnum,
    pub score: Option<i32>,
    pub info: Option<KaniSegmentInfo>,
    pub top: Option<TopArray>,
    pub text: Option<String>,
}

impl Segment {
    /// `get-text` override — `dict.lisp:677-679`:
    ///
    /// ```lisp
    /// (defmethod get-text ((segment segment))
    ///   (or (segment-text segment)
    ///       (setf (segment-text segment) (text (segment-word segment)))))
    /// ```
    ///
    /// Lazy memoization: returns the cached [`Segment::text`] when
    /// already populated, otherwise computes
    /// `text(segment.word)` (via [`crate::dict::counters::dispatchers::text`]),
    /// stores it, and returns a borrow. Mirrors upstream's `setf`
    /// — repeated calls are O(1) after the first.
    pub fn get_text(&mut self) -> &str {
        if self.text.is_none() {
            let t = crate::dict::counters::dispatchers::text(&self.word).into_owned();
            self.text = Some(t);
        }
        self.text.as_deref().unwrap()
    }
}

/// Sidecar (no Lisp FQN). Typed model of the property-list value
/// `calc-score` returns and `gen-score` stores in [`Segment::info`]
/// (`dict.lisp:976-980`). The Lisp slot type is `t`; the port pins
/// the six plist keys calc-score actually writes so downstream
/// consumers (the `def-generic-penalty` / `defsynergy` machinery in
/// `dict-grammar.lisp`, `cull-segments` in `dict.lisp:1024`) get a
/// checked field rather than a `getf` against an untyped list.
///
/// Closed-variant note (CONVENTIONS §4.3): the field set is
/// exhaustive against the current `calc-score` terminal `let`. If a
/// future calc-score change adds a `(setf (getf info :NEW-KEY) …)`
/// or extends the terminal `let`, this struct must grow a matching
/// field — silent drop would corrupt the segment.info contract that
/// every penalty / synergy rule reads.
#[derive(Debug, Clone)]
pub struct KaniSegmentInfo {
    /// `:posi` — part-of-speech tags. `("ctr",)` for counter
    /// readings; otherwise `(get-non-arch-posi seq-set)`.
    pub posi: Vec<String>,
    /// `:seq-set` — JMdict entry-seq plus every `from`-seq the
    /// reading conjugates from. `(cons seq conj-of)` in the
    /// non-counter branch.
    pub seq_set: Vec<i32>,
    /// `:conj` — conjugation records attached to this reading; one
    /// [`ConjData`] per `conj-prop` row. Empty when the reading is
    /// not a conjugated form.
    pub conj: Vec<ConjData>,
    /// `:common` — commonness rank. Lisp value is
    /// `(and common-p common-of)` where `common-of` may be `:null`
    /// (explicit "not common") or an integer. The port collapses
    /// `nil` (no common-p) and `:null` to [`None`]; integer
    /// commonness is [`Some`].
    pub common: Option<i32>,
    /// `:score-info` — `(prop-score, kanji-break, use-length-bonus,
    /// split-info)` telemetry tuple emitted by calc-score. Modeled
    /// as the typed sidecar [`KaniScoreInfo`].
    pub score_info: KaniScoreInfo,
    /// `:kpcl` — `(kanji-or-katakana, primary, common, long)`
    /// destructured by `def-generic-penalty` macro callsites in
    /// `dict-grammar.lisp:759`.
    pub kpcl: (bool, bool, bool, bool),
}

/// Sidecar (no Lisp FQN). The four-element list `calc-score` builds
/// at `dict.lisp:979` and stores under [`KaniSegmentInfo::score_info`].
/// Each field corresponds to one position in the Lisp
/// `(list prop-score kanji-break use-length-bonus split-info)`.
///
/// No `Default` derive — every instance must be the result of a real
/// `calc-score` call. A zero-valued default would let a careless
/// caller fabricate score-info that doesn't correspond to any
/// upstream Lisp state.
#[derive(Debug, Clone)]
pub struct KaniScoreInfo {
    pub prop_score: i32,
    /// `kanji-break` argument to calc-score; a list of character
    /// positions where a kanji boundary falls inside the slice, or
    /// empty when no break applies.
    pub kanji_break: Vec<usize>,
    pub use_length_bonus: i32,
    /// `split-info` — populated by the split-handling branch of
    /// calc-score (`dict.lisp:939-974`): a single integer
    /// (`:score` split), [`None`] (`:pscore` / no split), or the
    /// `(score-mod-split . part-scores)` list (multi-part split).
    pub split_info: KaniSplitInfo,
}

#[derive(Debug, Clone)]
pub enum KaniSplitInfo {
    None,
    Score(i32),
    Parts {
        score_mod: i32,
        part_scores: Vec<i32>,
    },
}

#[derive(Debug, Clone)]
pub struct SegmentList {
    pub segments: Vec<Segment>,
    pub start: usize,
    pub end: usize,
    /// Divergence from Lisp: wrapped in `Arc` so
    /// `SegmentList::clone()` only bumps a refcount instead of
    /// deep-cloning the accumulator. The Lisp slot is a pointer
    /// (every `(copy-segment-list)` shares the slot value), so the
    /// upstream pointer-share semantics are preserved by `Arc`
    /// better than by `Option<TopArray>` with a derived deep
    /// `Clone`. `find_best_path` is the sole mutator and uses
    /// `Arc::make_mut` on its own per-position slot; downstream
    /// readers only need the shared snapshot.
    pub top: Option<Arc<TopArray>>,
    pub matches: usize,
}

#[derive(Debug, Clone)]
pub struct TopArray {
    pub array: Vec<Option<TopArrayItem>>,
    pub count: usize,
}

impl TopArray {
    pub fn new(limit: usize) -> Self {
        Self {
            array: vec![None; limit],
            count: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TopArrayItem {
    pub score: i32,
    pub payload: Vec<PathElement>,
}

/// Sidecar (no Lisp FQN). Closed variant set for the entries
/// `register-item` stores in [`TopArrayItem::payload`]. Per
/// `slot_types.csv`'s two `top-array-item.payload` rows: the
/// `find-best-path` inner loop (`dict.lisp:1208-1226`) pushes
/// [`SegmentList`] elements; `get-seg-splits` (`dict.lisp:1175-1178`)
/// pushes [`Synergy`] elements via `get-penalties` / `get-synergies`.
#[derive(Debug, Clone)]
pub enum PathElement {
    SegmentList(SegmentList),
    Synergy(Synergy),
}

pub fn length_multiplier(length: i64, power: i64, len_lim: i64) -> i64 {
    if length <= len_lim {
        length.pow(power as u32)
    } else {
        length * len_lim.pow((power - 1) as u32)
    }
}

pub fn length_multiplier_coeff(length: i64, class: KaniLengthClass) -> i64 {
    // dict.lisp:693 declares `(integer 0 10000)` for the length
    // parameter. Real `assert!` rather than `debug_assert!` so
    // release-profile audit runs catch a negative input loudly
    // instead of silently extrapolating to a negative coefficient.
    assert!(
        length >= 0,
        "length-multiplier-coeff: length must be ≥ 0 (upstream type (integer 0 10000)), got {length}"
    );
    // dict.lisp:696 — (assoc class *length-coeff-sequences*)
    let coeffs: &[i64] = LENGTH_COEFF_SEQUENCES
        .iter()
        .find(|(c, _)| *c == class)
        .map(|(_, c)| *c)
        .expect("class must be in *length-coeff-sequences*");
    // Upstream `(length coeffs)` includes the keyword head; subtract
    // one to get the count of numeric entries, which is exactly
    // `coeffs.len()` here.
    let n = coeffs.len() as i64;
    // dict.lisp:698 — (< 0 length (length coeffs)) i.e. 0 < length < n+1.
    if 0 < length && length <= n {
        // dict.lisp:699 — (elt coeffs length); upstream index 1 maps to
        // Rust index 0 because the keyword head was sliced off.
        coeffs[(length - 1) as usize]
    } else {
        // dict.lisp:700 — (* length (/ (car (last coeffs)) (1- (length coeffs))))
        // Upstream `(/ a b)` produces a CL rational; the `(the
        // (integer 0 1000) …)` cast at the same line asserts the
        // division is exact. All four current rows satisfy that
        // (60/5, 36/6, 24/4, 24/4); flag a future table edit that
        // breaks parity. Real `assert!` so release-profile audit
        // runs surface the table-edit error rather than silently
        // floor-dividing.
        let last = coeffs[(n - 1) as usize];
        assert!(
            last % n == 0,
            "length-multiplier-coeff: (/ {last} {n}) is not exact — \
             *length-coeff-sequences* table edit broke the upstream \
             `(the (integer 0 1000) …)` assertion at dict.lisp:700"
        );
        length * (last / n)
    }
}

/// Rust-only sidecar tag for the keyword keys in
/// [`LENGTH_COEFF_SEQUENCES`]. Upstream uses bare CL keywords
/// (`:strong`, `:weak`, `:tail`, `:ltail`) inline as `assoc` keys with
/// no named type; the closed `(member …)` ftype declaration at
/// `dict.lisp:693` is the upstream spec for the set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KaniLengthClass {
    Strong,
    Weak,
    Tail,
    Ltail,
}

pub static LENGTH_COEFF_SEQUENCES: &[(KaniLengthClass, &[i64])] = &[
    (KaniLengthClass::Strong, &[1, 8, 24, 40, 60]),
    (KaniLengthClass::Weak, &[1, 4, 9, 16, 25, 36]),
    (KaniLengthClass::Tail, &[4, 9, 16, 24]),
    (KaniLengthClass::Ltail, &[4, 12, 18, 24]),
];

/// The three Lisp `(cond ((cdr kanji-break) :both) ((eql (car kanji-break) 0) :beg) (t :end))`
/// results. `kanji_break` empty → upstream `nil` → falls through to
/// `End` via `(cdr nil) = nil`, `(eql nil 0) = nil`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KanjiBreakEnd {
    Both,
    Beg,
    End,
}

fn classify_end(kanji_break: &[usize]) -> KanjiBreakEnd {
    // dict.lisp:703-705
    if kanji_break.len() > 1 {
        KanjiBreakEnd::Both
    } else if kanji_break.first().copied() == Some(0) {
        KanjiBreakEnd::Beg
    } else {
        KanjiBreakEnd::End
    }
}

pub async fn kanji_break_penalty(
    ctx: &KaniranContext,
    kanji_break: &[usize],
    score: i32,
    info: Option<&KaniSegmentInfo>,
    text: &str,
    use_length: Option<i32>,
    score_mod: Option<&ScoreMod>,
) -> Result<i32, sqlx::Error> {
    // dict.lisp:703-707 (let ((end ...) (bonus 0) (ratio 2) (posi (and info (getf info :posi)))))
    let end = classify_end(kanji_break);
    let mut bonus: i32 = 0;
    let ratio: i32 = 2;
    let posi: &[String] = info.map(|i| i.posi.as_slice()).unwrap_or(&[]);

    // dict.lisp:708 (when info ...)
    if let Some(info) = info {
        // dict.lisp:709-712 — (or (intersection seq-set *no-kanji-break-penalty*)
        //                        (and (eql end :beg) (alexandria:starts-with #\す text)))
        //   → (return-from kanji-break-penalty score)
        let seq_set_intersects = info
            .seq_set
            .iter()
            .any(|s| NO_KANJI_BREAK_PENALTY.contains(s));
        let starts_with_su = end == KanjiBreakEnd::Beg && text.chars().next() == Some('す');
        if seq_set_intersects || starts_with_su {
            return Ok(score);
        }

        // dict.lisp:713-721 ((intersection '("vs-s" "v5s") posi :test 'equal) …)
        //
        // Lisp `cond` semantics: once this clause matches it consumes the
        // dispatch, even when the inner `(when suru-suffix …)` evaluates to
        // nil. The remaining num/suf/pref clauses do NOT fire. The Rust
        // port mirrors that with an explicit `vs-s/v5s ∈ posi` guard on
        // the else-if chain below — without it, a `posi` that contains
        // both "v5s" and "suf" (e.g. dict.lisp:702 row "下す" posi=("suf"
        // "v5s" "vt")) double-fires and adds the +10 suf bonus that
        // Lisp's cond skipped.
        let is_vs_or_v5s = posi.iter().any(|s| s == "vs-s" || s == "v5s");
        if is_vs_or_v5s {
            // dict.lisp:715 (find :suru (get-suffixes text) :key 'second)
            let suffixes = get_suffixes(ctx, text);
            let suru_suffix = suffixes.iter().find(|(_, key, _)| *key == "suru");
            if let Some(&(suffix_text, _key, kf)) = suru_suffix {
                // Upstream `(calc-score (third suru-suffix) …)` feeds the
                // kana-form straight in; on a nil kf it would crash inside
                // calc-score's `(word-type nil)` call. Mirror that
                // panic-on-nil contract here rather than silently skipping.
                // `:SURU` entries are populated exclusively by
                // `load-conjs :suru …` at dict-grammar.lisp:244-248 →
                // `load-kf` → `(get-kana-forms seq)` element which is
                // always non-nil. Only `load-abbr` produces nil-kf cache
                // rows, and it never uses the `:SURU` key.
                let kf = kf.expect(
                    "load-conjs :suru always populates kf — see \
                     dict-grammar.lisp:244-248 / load-kf",
                );
                // dict.lisp:717 (offset = mora-length text - mora-length suffix-text)
                let text_mora = mora_length(text) as i32;
                let suffix_mora = mora_length(suffix_text) as i32;
                let offset = text_mora - suffix_mora;
                // dict.lisp:718-720 (calc-score (third suru-suffix)
                //                     :use-length (and use-length (- use-length offset))
                //                     :score-mod score-mod)
                let use_length_recur = use_length.map(|ul| ul - offset);
                let kf_word: KaniWordDispatchEnum = KaniWordDispatchEnum::Kana((*kf).clone());
                let (suffix_score, _info) = Box::pin(calc_score(
                    ctx,
                    &kf_word,
                    false,
                    use_length_recur,
                    score_mod,
                    &[],
                ))
                .await?;
                // dict.lisp:721 (return-from kanji-break-penalty (min score (+ suffix-score 50)))
                return Ok(score.min(suffix_score + 50));
            }
            // No suru-suffix → bonus stays 0; fall through to the
            // post-cond `(if (>= score *score-cutoff*) …)` arithmetic
            // WITHOUT entering the num/suf/pref clauses. Matches Lisp's
            // cond-consumed-by-this-clause semantics.
        } else if end == KanjiBreakEnd::Beg && posi.iter().any(|s| s == "num") {
            // dict.lisp:722-723 ((and (eql end :beg) (member "num" posi)) (incf bonus 5))
            bonus += 5;
        } else if end == KanjiBreakEnd::Beg && posi.iter().any(|s| s == "suf" || s == "n-suf") {
            // dict.lisp:724-726 ((and (eql end :beg) (intersection '("suf" "n-suf") posi)) (incf bonus 10))
            bonus += 10;
        } else if end == KanjiBreakEnd::End && posi.iter().any(|s| s == "pref") {
            // dict.lisp:727-728 ((and (eql end :end) (member "pref" posi)) (incf bonus 12))
            bonus += 12;
        }
    }

    // dict.lisp:730-732 (if (>= score *score-cutoff*)
    //                       (max *score-cutoff* (+ (ceiling score ratio) bonus))
    //                       score)
    if score >= SCORE_CUTOFF {
        // dict.lisp:731 (ceiling score ratio) — positive-operand round-up.
        let ceiling = (score + ratio - 1) / ratio;
        Ok(SCORE_CUTOFF.max(ceiling + bonus))
    } else {
        Ok(score)
    }
}

pub fn gap_penalty(start: usize, end: usize) -> i64 {
    (end as i64 - start as i64) * GAP_PENALTY
}

pub const GAP_PENALTY: i64 = -500;

pub fn find_sticky_positions(str: &str) -> Vec<usize> {
    let chars: Vec<char> = str.chars().collect();
    let str_len = chars.len();
    let mut out = Vec::new();

    for pos in 0..str_len {
        let ch = chars[pos];
        let char_class = get_char_class(ch);

        if char_class == Some(KanaClass::Sokuon)
            && pos != str_len - 1
            && get_char_class(chars[pos + 1]).is_some_and(is_kana_class)
        {
            out.push(pos + 1);
            continue;
        }

        if let Some(cc) = char_class {
            if is_modifier_or_iter_class(cc) {
                let suppress = pos == str_len - 1
                    && (cc == KanaClass::LongVowel
                        || (pos > 0 && long_vowel_modifier_p(cc, chars[pos - 1])));
                if !suppress {
                    out.push(pos);
                }
            }
        }
    }

    out
}

fn is_kana_class(cc: KanaClass) -> bool {
    KANA_CHARACTERS.iter().any(|(k, _)| *k == cc)
}

fn is_modifier_or_iter_class(cc: KanaClass) -> bool {
    MODIFIER_CHARACTERS.iter().any(|(k, _)| *k == cc)
        || ITERATION_CHARACTERS.iter().any(|(k, _)| *k == cc)
}

pub fn make_slice() -> &'static str {
    ""
}

pub fn subseq_slice<'a>(
    _slice: Option<&str>,
    s: &'a str,
    start: usize,
    end: Option<usize>,
) -> &'a str {
    let total_chars = s.chars().count();
    let end_chars = end.unwrap_or(total_chars);
    assert!(
        end_chars >= start,
        "subseq-slice: end ({}) < start ({})",
        end_chars,
        start,
    );
    // dict.lisp:1016 (adjust-array :displaced-to) — upstream signals
    // ":DISPLACED-TO array is too small" when end > (length str); with
    // end >= start above, this also covers start > (length str).
    assert!(
        end_chars <= total_chars,
        "subseq-slice: end ({}) > (length s) ({})",
        end_chars,
        total_chars,
    );
    let byte_start = nth_char_byte(s, start);
    let byte_end = nth_char_byte(s, end_chars);
    &s[byte_start..byte_end]
}

fn nth_char_byte(s: &str, n: usize) -> usize {
    s.char_indices().nth(n).map(|(b, _)| b).unwrap_or(s.len())
}

/// Faithful image of the three upstream return shapes. Predicate
/// callers consult [`Self::is_truthy`]; fixture replay compares the
/// variant directly to the captured Lisp value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareCommonResult {
    /// Cond fell off without a `t` clause, or a branch evaluated to
    /// `nil` (`(< c1 c2)` returning `nil`, `(and c1 (> c1 0))`
    /// failing, etc.). Maps to Lisp `NIL` / JSON `null`.
    Nil,
    /// First branch — `((not c2) c1)` — fired and returned `c1`
    /// itself (non-`nil` integer). Maps to Lisp integer / JSON
    /// number.
    C1(i64),
    /// Second or third branch returned `T`. Maps to Lisp `T` / JSON
    /// `true`.
    True,
}

impl CompareCommonResult {
    /// Truthiness for the comparator/predicate consumers at
    /// `dict.lisp:867`, `1029`, `1877`. `Nil` is the only falsy
    /// variant — `C1(0)` is truthy because Lisp `0` is truthy.
    pub fn is_truthy(self) -> bool {
        !matches!(self, CompareCommonResult::Nil)
    }
}

pub fn compare_common(c1: Option<i64>, c2: Option<i64>) -> CompareCommonResult {
    // dict.lisp:1023 — ((not c2) c1). Branch returns c1 itself; nil
    // c1 means the branch returns nil.
    if c2.is_none() {
        return match c1 {
            Some(n) => CompareCommonResult::C1(n),
            None => CompareCommonResult::Nil,
        };
    }
    let c2 = c2.unwrap();
    // dict.lisp:1024 — ((= c2 0) (and c1 (> c1 0))). Branch evaluates
    // to T or NIL.
    if c2 == 0 {
        return if matches!(c1, Some(n) if n > 0) {
            CompareCommonResult::True
        } else {
            CompareCommonResult::Nil
        };
    }
    // dict.lisp:1025 — ((and c1 (> c1 0)) (< c1 c2)). Branch only
    // fires when c1 is positive; result is (< c1 c2) which is T or
    // NIL.
    if let Some(n) = c1 {
        if n > 0 {
            return if n < c2 {
                CompareCommonResult::True
            } else {
                CompareCommonResult::Nil
            };
        }
    }
    // cond falls off without a t-clause → nil.
    CompareCommonResult::Nil
}

pub fn cull_segments(mut segments: Vec<Segment>) -> Vec<Segment> {
    if segments.is_empty() {
        return segments;
    }
    // dict.lisp:1029-1030 (stable-sort by compare-common over :common)
    segments.sort_by(|a, b| {
        let ka = a.info.as_ref().and_then(|info| info.common).map(i64::from);
        let kb = b.info.as_ref().and_then(|info| info.common).map(i64::from);
        if compare_common(ka, kb).is_truthy() {
            std::cmp::Ordering::Less
        } else if compare_common(kb, ka).is_truthy() {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    });
    // dict.lisp:1031 (stable-sort by > on segment-score)
    segments.sort_by(|a, b| b.score.cmp(&a.score));
    // dict.lisp:1032-1033 (max-score / cutoff)
    let max_score = i64::from(segments[0].score.expect(
        "cull-segments: segments[0].score is None — gen-score must populate scores before cull-segments",
    ));
    let (num, den) = IDENTICAL_WORD_SCORE_CUTOFF;
    // dict.lisp:1034-1036 (loop while (>= score cutoff) collect)
    let kept = segments
        .iter()
        .position(|seg| {
            let s = i64::from(
                seg.score
                    .expect("cull-segments: segment.score is None — gen-score must populate"),
            );
            den * s < num * max_score
        })
        .unwrap_or(segments.len());
    segments.truncate(kept);
    segments
}

pub fn get_seg_initial(seg: &Arc<KaniLiteSegmentList>) -> Vec<Arc<KaniLiteSegmentList>> {
    apply_segfilters(None, seg)
        .into_iter()
        .map(|(_left, right)| right)
        .collect()
}

pub fn get_seg_splits(
    seg_left: &Arc<KaniLiteSegmentList>,
    seg_right: &Arc<KaniLiteSegmentList>,
) -> Vec<Vec<KaniLitePathElement>> {
    // dict.lisp:1176 (let ((splits (apply-segfilters seg-left seg-right))))
    let splits = apply_segfilters(Some(seg_left), seg_right);
    let mut result: Vec<Vec<KaniLitePathElement>> = Vec::new();
    // dict.lisp:1177-1178 (loop for (seg-left seg-right) in splits
    //                       nconcing (cons (get-penalties seg-left seg-right)
    //                                      (get-synergies seg-left seg-right)))
    for (left_opt, right) in &splits {
        let left = left_opt
            .as_ref()
            .expect("apply_segfilters preserves Some-left when input left is Some");
        result.push(get_penalties(left, right));
        for synergy_path in get_synergies(left, right) {
            result.push(synergy_path);
        }
    }
    result
}

pub async fn expand_segment_list(
    ctx: &KaniranContext,
    segment_list: &mut SegmentList,
) -> Result<(), sqlx::Error> {
    // dict.lisp:1183-1187 — `(loop for segment in segments for segsplit = (get-segsplit segment) collect segment when segsplit collect segsplit and do (incf matches))`.
    // Move the existing segments out so we can hand each one to
    // get_segsplit by reference, then push owned values into the new
    // working list.
    let pre_segments = std::mem::take(&mut segment_list.segments);
    let mut working: Vec<Segment> = Vec::with_capacity(pre_segments.len() * 2);
    for segment in pre_segments {
        let segsplit = get_segsplit(ctx, &segment).await?;
        working.push(segment);
        if let Some(segsplit) = segsplit {
            working.push(segsplit);
            segment_list.matches += 1;
        }
    }
    // dict.lisp:1188 — `(stable-sort … #'> :key #'segment-score)`. Rust
    // slice `sort_by` is stable; gen-score (`dict.lisp:986`) guarantees
    // every segment reaching this point carries `Some(score)` —
    // `cull-segments` (`dict.lisp:1027`) sorts by `segment-score`
    // upstream and would already have crashed on `nil`.
    working.sort_by(|a, b| {
        let a_score = a
            .score
            .expect("expand-segment-list: segment.score must be Some (cull-segments output)");
        let b_score = b
            .score
            .expect("expand-segment-list: segment.score must be Some (cull-segments output)");
        b_score.cmp(&a_score)
    });
    segment_list.segments = working;
    Ok(())
}

#[derive(Debug, Clone)]
pub enum KaniSegmentScoreArg<'a> {
    Segment(&'a Segment),
    SegmentList(&'a SegmentList),
    KaniLiteSegmentList(&'a KaniLiteSegmentList),
    Synergy(&'a Synergy),
}

pub fn get_segment_score(seg: &KaniSegmentScoreArg) -> Option<i32> {
    match seg {
        // dict.lisp:1042-1043 (:method ((seg segment)))
        KaniSegmentScoreArg::Segment(s) => s.score,
        // dict.lisp:1044-1046 (:method ((seg-list segment-list)))
        KaniSegmentScoreArg::SegmentList(sl) => match sl.segments.first() {
            Some(first) => first.score,
            None => Some(0),
        },
        // Same shape as SegmentList arm, but reads the precomputed
        // `score` off the lite segment instead of dereffing into
        // `info` / `Segment.score`.
        KaniSegmentScoreArg::KaniLiteSegmentList(sl) => match sl.segments.first() {
            Some(first) => first.score,
            None => Some(0),
        },
        // dict-grammar.lisp:715-716 (defmethod get-segment-score ((syn synergy)))
        KaniSegmentScoreArg::Synergy(syn) => Some(syn.score),
    }
}

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
mod test_top_array_class {
    use super::*;

    #[test]
    fn new_preallocates_limit_with_nones() {
        let ta = TopArray::new(5);
        assert_eq!(ta.array.len(), 5);
        assert!(ta.array.iter().all(|x| x.is_none()));
        assert_eq!(ta.count, 0);
    }
}

#[cfg(test)]
mod test_length_multiplier {
    use super::*;

    // REPL fixtures (.103, ichiran/dict::length-multiplier), 2026-05-25.
    // `(length, power, len-lim) -> result`; both cond branches and the
    // `length == len-lim` boundary (first branch) are covered.
    #[test]
    fn length_multiplier_fixtures() {
        let cases: &[(i64, i64, i64, i64)] = &[
            // length <= len-lim  → length^power
            (3, 2, 5, 9),
            (5, 2, 5, 25), // boundary: length == len-lim
            (4, 3, 6, 64),
            (3, 1, 5, 3),
            (1, 4, 2, 1),
            // length > len-lim   → length * len-lim^(power-1)
            (7, 2, 5, 35),
            (8, 3, 6, 288),
            (7, 1, 5, 7), // power 1 → len-lim^0 = 1
            (10, 2, 3, 30),
            (6, 4, 4, 384),
        ];
        for &(length, power, len_lim, expected) in cases {
            assert_eq!(
                length_multiplier(length, power, len_lim),
                expected,
                "length={length} power={power} len_lim={len_lim}"
            );
        }
    }
}

#[cfg(test)]
mod test_length_multiplier_coeff {
    use super::*;

    // All assertions REPL-pinned against upstream ichiran.
    #[test]
    fn strong_tabled_range() {
        assert_eq!(length_multiplier_coeff(0, KaniLengthClass::Strong), 0);
        assert_eq!(length_multiplier_coeff(1, KaniLengthClass::Strong), 1);
        assert_eq!(length_multiplier_coeff(2, KaniLengthClass::Strong), 8);
        assert_eq!(length_multiplier_coeff(3, KaniLengthClass::Strong), 24);
        assert_eq!(length_multiplier_coeff(4, KaniLengthClass::Strong), 40);
        assert_eq!(length_multiplier_coeff(5, KaniLengthClass::Strong), 60);
    }

    #[test]
    fn strong_extrapolation() {
        // n = 5, last = 60, last/n = 12. length * 12 outside range.
        assert_eq!(length_multiplier_coeff(6, KaniLengthClass::Strong), 72);
        assert_eq!(length_multiplier_coeff(7, KaniLengthClass::Strong), 84);
        assert_eq!(length_multiplier_coeff(8, KaniLengthClass::Strong), 96);
        assert_eq!(length_multiplier_coeff(10, KaniLengthClass::Strong), 120);
        assert_eq!(length_multiplier_coeff(50, KaniLengthClass::Strong), 600);
    }

    #[test]
    fn weak_tabled_range() {
        assert_eq!(length_multiplier_coeff(0, KaniLengthClass::Weak), 0);
        assert_eq!(length_multiplier_coeff(1, KaniLengthClass::Weak), 1);
        assert_eq!(length_multiplier_coeff(2, KaniLengthClass::Weak), 4);
        assert_eq!(length_multiplier_coeff(3, KaniLengthClass::Weak), 9);
        assert_eq!(length_multiplier_coeff(4, KaniLengthClass::Weak), 16);
        assert_eq!(length_multiplier_coeff(5, KaniLengthClass::Weak), 25);
        assert_eq!(length_multiplier_coeff(6, KaniLengthClass::Weak), 36);
    }

    #[test]
    fn weak_extrapolation() {
        // n = 6, last = 36, last/n = 6.
        assert_eq!(length_multiplier_coeff(7, KaniLengthClass::Weak), 42);
        assert_eq!(length_multiplier_coeff(8, KaniLengthClass::Weak), 48);
        assert_eq!(length_multiplier_coeff(100, KaniLengthClass::Weak), 600);
    }

    #[test]
    fn tail_tabled_range() {
        assert_eq!(length_multiplier_coeff(0, KaniLengthClass::Tail), 0);
        assert_eq!(length_multiplier_coeff(1, KaniLengthClass::Tail), 4);
        assert_eq!(length_multiplier_coeff(2, KaniLengthClass::Tail), 9);
        assert_eq!(length_multiplier_coeff(3, KaniLengthClass::Tail), 16);
        assert_eq!(length_multiplier_coeff(4, KaniLengthClass::Tail), 24);
    }

    #[test]
    fn tail_extrapolation() {
        // n = 4, last = 24, last/n = 6.
        assert_eq!(length_multiplier_coeff(5, KaniLengthClass::Tail), 30);
        assert_eq!(length_multiplier_coeff(6, KaniLengthClass::Tail), 36);
        assert_eq!(length_multiplier_coeff(7, KaniLengthClass::Tail), 42);
        assert_eq!(length_multiplier_coeff(8, KaniLengthClass::Tail), 48);
        assert_eq!(length_multiplier_coeff(1000, KaniLengthClass::Tail), 6000);
    }

    #[test]
    fn ltail_tabled_range() {
        assert_eq!(length_multiplier_coeff(0, KaniLengthClass::Ltail), 0);
        assert_eq!(length_multiplier_coeff(1, KaniLengthClass::Ltail), 4);
        assert_eq!(length_multiplier_coeff(2, KaniLengthClass::Ltail), 12);
        assert_eq!(length_multiplier_coeff(3, KaniLengthClass::Ltail), 18);
        assert_eq!(length_multiplier_coeff(4, KaniLengthClass::Ltail), 24);
    }

    #[test]
    fn ltail_extrapolation() {
        // n = 4, last = 24, last/n = 6.
        assert_eq!(length_multiplier_coeff(5, KaniLengthClass::Ltail), 30);
        assert_eq!(length_multiplier_coeff(6, KaniLengthClass::Ltail), 36);
        assert_eq!(length_multiplier_coeff(7, KaniLengthClass::Ltail), 42);
        assert_eq!(length_multiplier_coeff(8, KaniLengthClass::Ltail), 48);
        assert_eq!(
            length_multiplier_coeff(10000, KaniLengthClass::Ltail),
            60000
        );
    }
}

#[cfg(test)]
mod test__star_length_coeff_sequences_star {
    use super::*;

    // REPL-pinned (.103 SBCL, 2026-05-13):
    //   *length-coeff-sequences* =
    //     ((:STRONG 1 8 24 40 60)
    //      (:WEAK   1 4 9 16 25 36)
    //      (:TAIL   4 9 16 24)
    //      (:LTAIL  4 12 18 24))
    #[test]
    fn matches_introspected_value() {
        assert_eq!(LENGTH_COEFF_SEQUENCES.len(), 4);
        assert_eq!(
            LENGTH_COEFF_SEQUENCES[0],
            (KaniLengthClass::Strong, &[1i64, 8, 24, 40, 60][..])
        );
        assert_eq!(
            LENGTH_COEFF_SEQUENCES[1],
            (KaniLengthClass::Weak, &[1i64, 4, 9, 16, 25, 36][..])
        );
        assert_eq!(
            LENGTH_COEFF_SEQUENCES[2],
            (KaniLengthClass::Tail, &[4i64, 9, 16, 24][..])
        );
        assert_eq!(
            LENGTH_COEFF_SEQUENCES[3],
            (KaniLengthClass::Ltail, &[4i64, 12, 18, 24][..])
        );
    }
}

#[cfg(test)]
mod test_kanji_break_penalty {
    use super::*;

    // ----- pure-arithmetic cases (no info, no DB) -----
    //
    // Every assertion REPL-pinned against upstream ichiran 2026-05-16.

    #[tokio::test]
    async fn no_info_above_cutoff_halves_with_ceiling() {
        // REPL: (kanji-break-penalty '(0) 100) → 50
        // 100 >= 5 → max(5, ceil(100/2) + 0) = max(5, 50) = 50
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = kanji_break_penalty(&ctx, &[0], 100, None, "", None, None)
            .await
            .unwrap();
        assert_eq!(got, 50);
    }

    #[tokio::test]
    async fn no_info_odd_score_rounds_up() {
        // REPL: (kanji-break-penalty '(1) 100) → 50 (same arithmetic; end branch unused without posi)
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = kanji_break_penalty(&ctx, &[1], 100, None, "", None, None)
            .await
            .unwrap();
        assert_eq!(got, 50);
    }

    #[tokio::test]
    async fn no_info_both_branch() {
        // REPL: (kanji-break-penalty '(0 5) 100) → 50
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = kanji_break_penalty(&ctx, &[0, 5], 100, None, "", None, None)
            .await
            .unwrap();
        assert_eq!(got, 50);
    }

    #[tokio::test]
    async fn below_cutoff_returns_unchanged() {
        // REPL: (kanji-break-penalty '(0) 4) → 4 (4 < *score-cutoff* = 5)
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let got = kanji_break_penalty(&ctx, &[0], 4, None, "", None, None)
            .await
            .unwrap();
        assert_eq!(got, 4);
    }

    // ----- info-bearing cases (calc_score + kanji_break_penalty integration) -----
    //
    // The pure-arithmetic cases above exercise the `info=None` arm.
    // These exercise the four cond branches at dict.lisp:709-728 that
    // gate on info contents.

    /// REPL: with seq 1467640 (`猫`, common-rank-7 noun) →
    ///   `(calc-score row)` → 19, info :posi ("n") :seq-set (1467640).
    ///   `(kanji-break-penalty '(0) 19 :info info :text "猫")` → 10.
    ///   Hits the fall-through "penalty applies" branch
    ///   (no seq-set ∩ `*no-kanji-break-penalty*`, no `す` prefix, no
    ///   num/suf/pref bonus). Arithmetic: 19 ≥ 5 → max(5, ceil(19/2) + 0)
    ///   = max(5, 10) = 10.
    #[tokio::test]
    async fn info_fall_through_penalty() {
        use crate::dict::calc_score::calc_score;
        use crate::dict::kani::KaniWordDispatchEnum;
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let rows: Vec<crate::dict::dao::KanjiText> = sqlx::query_as(
            "SELECT * FROM kanji_text WHERE seq = 1467640 AND text = '猫' ORDER BY id LIMIT 1",
        )
        .fetch_all(&ctx.pool)
        .await
        .expect("猫 1467640 row");
        let w = KaniWordDispatchEnum::Kanji(rows.into_iter().next().unwrap());
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[]).await.unwrap();
        assert_eq!(score, 19);
        let info = info.unwrap();
        let got = kanji_break_penalty(&ctx, &[0], score, Some(&info), "猫", None, None)
            .await
            .unwrap();
        assert_eq!(got, 10);
    }

    /// REPL: `飲む` (seq 1169870) is in `*no-kanji-break-penalty*`,
    /// so `kanji-break-penalty` returns `score` unchanged regardless
    /// of arithmetic. Pinned at score=128 (from `(calc-score …)` on
    /// the kanji row).
    #[tokio::test]
    async fn no_penalty_list_short_circuit() {
        use crate::dict::calc_score::calc_score;
        use crate::dict::kani::KaniWordDispatchEnum;
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let rows: Vec<crate::dict::dao::KanjiText> = sqlx::query_as(
            "SELECT * FROM kanji_text WHERE seq = 1169870 AND text = '飲む' ORDER BY id LIMIT 1",
        )
        .fetch_all(&ctx.pool)
        .await
        .expect("飲む 1169870 row");
        let w = KaniWordDispatchEnum::Kanji(rows.into_iter().next().unwrap());
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[]).await.unwrap();
        let info = info.unwrap();
        // dict.lisp:709 — intersection seq-set *no-kanji-break-penalty*
        // returns truthy → return score unchanged.
        let got = kanji_break_penalty(&ctx, &[0], score, Some(&info), "飲む", None, None)
            .await
            .unwrap();
        assert_eq!(got, score);
    }

    /// REPL: `好き` (seq 1277450) is in `*no-kanji-break-penalty*`,
    /// short-circuits regardless of text. Also exercises the
    /// `(eql end :beg) (alexandria:starts-with #\す text)` arm —
    /// even if seq-set didn't short-circuit, the `す`-prefix branch
    /// would. Pinned via the seq-set route.
    #[tokio::test]
    async fn suki_seq_short_circuit() {
        use crate::dict::calc_score::calc_score;
        use crate::dict::kani::KaniWordDispatchEnum;
        let ctx = KaniranContext::from_env().await.expect("ctx");
        let rows: Vec<crate::dict::dao::KanjiText> = sqlx::query_as(
            "SELECT * FROM kanji_text WHERE seq = 1277450 AND text = '好き' ORDER BY id LIMIT 1",
        )
        .fetch_all(&ctx.pool)
        .await
        .expect("好き 1277450 row");
        let w = KaniWordDispatchEnum::Kanji(rows.into_iter().next().unwrap());
        let (score, info) = calc_score(&ctx, &w, false, None, None, &[]).await.unwrap();
        let info = info.unwrap();
        let got_kanji_text =
            kanji_break_penalty(&ctx, &[0], score, Some(&info), "好き", None, None)
                .await
                .unwrap();
        let got_kana_text = kanji_break_penalty(&ctx, &[0], score, Some(&info), "すき", None, None)
            .await
            .unwrap();
        // REPL pinned: both → score unchanged (seq-set short-circuits first).
        assert_eq!(got_kanji_text, score);
        assert_eq!(got_kana_text, score);
    }

    #[tokio::test]
    async fn classify_end_results() {
        // pinned via direct cond evaluation on .103: kanji-break list →
        // (cond ((cdr kb) :both) ((eql (car kb) 0) :beg) (t :end))
        assert_eq!(classify_end(&[]), KanjiBreakEnd::End);
        assert_eq!(classify_end(&[0]), KanjiBreakEnd::Beg);
        assert_eq!(classify_end(&[3]), KanjiBreakEnd::End);
        assert_eq!(classify_end(&[5]), KanjiBreakEnd::End);
        assert_eq!(classify_end(&[0, 2]), KanjiBreakEnd::Both);
        assert_eq!(classify_end(&[1, 4]), KanjiBreakEnd::Both);
        assert_eq!(classify_end(&[0, 1, 2]), KanjiBreakEnd::Both);
    }
}

#[cfg(test)]
mod test_gap_penalty {
    use super::*;

    // REPL-pinned (.103 SBCL 2.2.9, 2026-05-14):
    //   (ichiran/dict::gap-penalty 0 0)   => 0
    //   (ichiran/dict::gap-penalty 0 3)   => -1500
    //   (ichiran/dict::gap-penalty 7 9)   => -1000
    //   (ichiran/dict::gap-penalty 10 10) => 0
    //   (ichiran/dict::gap-penalty 5 2)   => 1500
    #[test]
    fn matches_repl() {
        assert_eq!(gap_penalty(0, 0), 0);
        assert_eq!(gap_penalty(0, 3), -1500);
        assert_eq!(gap_penalty(7, 9), -1000);
        assert_eq!(gap_penalty(10, 10), 0);
        assert_eq!(gap_penalty(5, 2), 1500);
    }
}

#[cfg(test)]
mod test_find_sticky_positions {
    use super::find_sticky_positions;

    #[test]
    fn empty_string() {
        assert_eq!(find_sticky_positions(""), Vec::<usize>::new());
    }

    #[test]
    fn no_stickies_kanji() {
        assert_eq!(find_sticky_positions("食べる"), Vec::<usize>::new());
        assert_eq!(find_sticky_positions("学校"), Vec::<usize>::new());
        assert_eq!(
            find_sticky_positions("私はその本を読みました"),
            Vec::<usize>::new()
        );
        assert_eq!(find_sticky_positions("東京特許許可局"), Vec::<usize>::new());
    }

    #[test]
    fn modifier_mid_word() {
        assert_eq!(find_sticky_positions("きゃく"), vec![1]);
        assert_eq!(find_sticky_positions("けーき"), vec![1]);
        assert_eq!(find_sticky_positions("あぁい"), vec![1]);
    }

    #[test]
    fn modifier_at_end_collected_when_no_long_vowel_match() {
        // +YA at end: long_vowel_modifier_p returns false (not in +A/+I/+U/+E/+O).
        assert_eq!(find_sticky_positions("きゃ"), vec![1]);
        // +A after KI: vowels don't agree (KI ends in I), so collected.
        assert_eq!(find_sticky_positions("きぁ"), vec![1]);
        // +O after NI: vowels don't agree, collected.
        assert_eq!(find_sticky_positions("にぉ"), vec![1]);
        // Modifier after non-kana char (prev has no KanaClass): collected.
        assert_eq!(find_sticky_positions("漢ぁ"), vec![1]);
        // +WA at end: long_vowel_modifier_p false for PlusWa, collected.
        assert_eq!(find_sticky_positions("かゎ"), vec![1]);
    }

    #[test]
    fn modifier_at_end_suppressed_when_long_vowel_matches() {
        // +A after KA: vowel agrees, suppressed.
        assert_eq!(find_sticky_positions("かぁ"), Vec::<usize>::new());
        // +I after NI: vowel agrees, suppressed.
        assert_eq!(find_sticky_positions("にぃ"), Vec::<usize>::new());
    }

    #[test]
    fn long_vowel_at_end_suppressed() {
        assert_eq!(find_sticky_positions("かー"), Vec::<usize>::new());
        assert_eq!(find_sticky_positions("あー"), Vec::<usize>::new());
    }

    #[test]
    fn long_vowel_at_start_collected() {
        assert_eq!(find_sticky_positions("ーあ"), vec![0]);
    }

    #[test]
    fn modifier_first_char_not_last_collected() {
        // Modifier at pos 0 with str_len > 1: not last, so lvmp branch irrelevant.
        assert_eq!(find_sticky_positions("ぁか"), vec![0]);
    }

    #[test]
    fn modifier_lone_char_collected() {
        // pos==0, last, but `(> pos 0)` is false, so lvmp branch short-circuits.
        assert_eq!(find_sticky_positions("ぁ"), vec![0]);
        // Same — PlusWa at lone position.
        assert_eq!(find_sticky_positions("ゎ"), vec![0]);
    }

    #[test]
    fn sokuon_mid_word_collects_pos_plus_one() {
        assert_eq!(find_sticky_positions("いっぱい"), vec![2]);
        assert_eq!(find_sticky_positions("ニッポン"), vec![2]);
        assert_eq!(find_sticky_positions("ニッキ"), vec![2]);
        assert_eq!(find_sticky_positions("っあっい"), vec![1, 3]);
    }

    #[test]
    fn sokuon_at_end_not_collected() {
        assert_eq!(find_sticky_positions("いっ"), Vec::<usize>::new());
        assert_eq!(find_sticky_positions("っ"), Vec::<usize>::new());
    }

    #[test]
    fn sokuon_followed_by_non_kana_not_collected() {
        assert_eq!(find_sticky_positions("っ漢"), Vec::<usize>::new());
        assert_eq!(find_sticky_positions("っX"), Vec::<usize>::new());
    }

    #[test]
    fn iteration_characters() {
        // Both iter marks: pos 0 not last (collect 0), pos 1 last & not long-vowel & lvmp false → collect 1.
        assert_eq!(find_sticky_positions("ゝゞ"), vec![0, 1]);
        // ゝ at end after い: lvmp false, long-vowel false → collected.
        assert_eq!(find_sticky_positions("いゝ"), vec![1]);
    }

    #[test]
    fn single_kana_char_no_sticky() {
        assert_eq!(find_sticky_positions("あ"), Vec::<usize>::new());
        assert_eq!(find_sticky_positions("いろは"), Vec::<usize>::new());
    }

    #[test]
    fn combined_sokuon_and_modifier() {
        assert_eq!(find_sticky_positions("きゃっき"), vec![1, 3]);
    }
}

#[cfg(test)]
mod test_make_slice {
    use super::*;

    /// REPL: `(length (make-slice))` → 0, `(string= (make-slice) "")` → T
    #[test]
    fn empty_seed() {
        let s = make_slice();
        assert_eq!(s.len(), 0);
        assert_eq!(s, "");
    }
}

#[cfg(test)]
mod test_subseq_slice {
    use super::*;

    /// REPL: `(subseq-slice nil "あいうえお" 1 3)` → `"いう"` (length 2).
    /// Pins character-offset semantics across multi-byte UTF-8.
    #[test]
    fn character_offsets_multi_byte() {
        let r = subseq_slice(None, "あいうえお", 1, Some(3));
        assert_eq!(r, "いう");
    }

    /// REPL: `(subseq-slice nil "abcde" 0 5)` → `"abcde"`.
    #[test]
    fn full_range_ascii() {
        let r = subseq_slice(None, "abcde", 0, Some(5));
        assert_eq!(r, "abcde");
    }

    /// REPL: `(subseq-slice nil "abcde" 0)` → `"abcde"` (default end).
    #[test]
    fn end_defaults_to_length() {
        let r = subseq_slice(None, "abcde", 0, None);
        assert_eq!(r, "abcde");
    }

    /// REPL: `(subseq-slice nil "abc" 1)` → `"bc"` (default end past start).
    #[test]
    fn end_default_with_offset_start() {
        let r = subseq_slice(None, "abc", 1, None);
        assert_eq!(r, "bc");
    }

    /// REPL: `(subseq-slice nil "hello" 2 2)` → `""` (start == end).
    #[test]
    fn empty_range_when_start_equals_end() {
        let r = subseq_slice(None, "hello", 2, Some(2));
        assert_eq!(r, "");
    }

    /// REPL: passing in an existing slice returns a view of `s` regardless.
    /// `(let ((s (make-slice))) (subseq-slice s "hello" 1 4))` → `"ell"`.
    #[test]
    fn slice_argument_is_ignored() {
        let seed = crate::dict::segment::make_slice();
        let r = subseq_slice(Some(seed), "hello", 1, Some(4));
        assert_eq!(r, "ell");
    }

    /// REPL: `(subseq-slice nil "hello" 4 2)` → assertion failure
    /// `(>= END START)` (END=2, START=4).
    #[test]
    #[should_panic(expected = "subseq-slice: end (2) < start (4)")]
    fn end_less_than_start_panics() {
        let _ = subseq_slice(None, "hello", 4, Some(2));
    }

    /// REPL: `(subseq-slice nil "hello" 0 10)` →
    /// `ERROR: The :DISPLACED-TO array is too small.`
    #[test]
    #[should_panic(expected = "subseq-slice: end (10) > (length s) (5)")]
    fn end_past_length_panics() {
        let _ = subseq_slice(None, "hello", 0, Some(10));
    }

    /// REPL: `(subseq-slice nil "hello" 2 7)` →
    /// `ERROR: The :DISPLACED-TO array is too small.` (start in range,
    /// end past length).
    #[test]
    #[should_panic(expected = "subseq-slice: end (7) > (length s) (5)")]
    fn start_in_range_end_past_length_panics() {
        let _ = subseq_slice(None, "hello", 2, Some(7));
    }

    /// REPL: `(subseq-slice nil "hello" 10 12)` →
    /// `ERROR: The :DISPLACED-TO array is too small.` (both out of
    /// range; rejected via the end-bound check).
    #[test]
    #[should_panic(expected = "subseq-slice: end (12) > (length s) (5)")]
    fn start_and_end_past_length_panics() {
        let _ = subseq_slice(None, "hello", 10, Some(12));
    }

    /// REPL: `(subseq-slice nil "hello" 5 5)` → `""` (start == end ==
    /// length is the upper-edge OK case, no error).
    #[test]
    fn start_equal_to_length_at_end_is_ok() {
        let r = subseq_slice(None, "hello", 5, Some(5));
        assert_eq!(r, "");
    }

    /// REPL: `(subseq-slice nil "hello" 0 5)` → `"hello"` (end ==
    /// length is the upper-edge OK case).
    #[test]
    fn end_equal_to_length_is_ok() {
        let r = subseq_slice(None, "hello", 0, Some(5));
        assert_eq!(r, "hello");
    }
}

#[cfg(test)]
mod test_compare_common {
    use super::*;
    use CompareCommonResult::*;

    // All assertions REPL-pinned against upstream ichiran. Each value
    // matches the exact Lisp return: branch 1 returns c1 itself, so
    // (compare-common 5 NIL) = 5 (C1(5)); branches 2/3 return T or NIL.
    #[test]
    fn nil_c1_always_nil() {
        // (compare-common NIL <anything>) = NIL.
        for c2 in [None, Some(0), Some(1), Some(2), Some(5), Some(10), Some(-3)] {
            assert_eq!(compare_common(None, c2), Nil);
        }
    }

    #[test]
    fn nil_c2_returns_c1_itself() {
        // (compare-common <integer> NIL) returns c1 (branch 1).
        assert_eq!(compare_common(Some(0), None), C1(0));
        assert_eq!(compare_common(Some(1), None), C1(1));
        assert_eq!(compare_common(Some(2), None), C1(2));
        assert_eq!(compare_common(Some(5), None), C1(5));
        assert_eq!(compare_common(Some(10), None), C1(10));
        assert_eq!(compare_common(Some(-3), None), C1(-3));
    }

    #[test]
    fn zero_c1_only_truthy_when_c2_nil() {
        // (compare-common 0 NIL) = 0 (C1(0), truthy); all others NIL.
        assert_eq!(compare_common(Some(0), None), C1(0));
        assert_eq!(compare_common(Some(0), Some(0)), Nil);
        assert_eq!(compare_common(Some(0), Some(1)), Nil);
        assert_eq!(compare_common(Some(0), Some(2)), Nil);
        assert_eq!(compare_common(Some(0), Some(5)), Nil);
        assert_eq!(compare_common(Some(0), Some(10)), Nil);
        assert_eq!(compare_common(Some(0), Some(-3)), Nil);
    }

    #[test]
    fn c2_zero_returns_true_when_c1_positive() {
        // (compare-common <pos> 0) = T (branch 2); otherwise NIL.
        assert_eq!(compare_common(Some(1), Some(0)), True);
        assert_eq!(compare_common(Some(2), Some(0)), True);
        assert_eq!(compare_common(Some(5), Some(0)), True);
        assert_eq!(compare_common(Some(10), Some(0)), True);
        assert_eq!(compare_common(Some(0), Some(0)), Nil);
        assert_eq!(compare_common(Some(-3), Some(0)), Nil);
    }

    #[test]
    fn positive_pair_lt_predicate() {
        // Branch 3: (compare-common 1 2) = T (since 1 < 2);
        // (compare-common 2 1) = NIL (since 2 not < 1).
        assert_eq!(compare_common(Some(1), Some(2)), True);
        assert_eq!(compare_common(Some(1), Some(5)), True);
        assert_eq!(compare_common(Some(1), Some(10)), True);
        assert_eq!(compare_common(Some(2), Some(5)), True);
        assert_eq!(compare_common(Some(2), Some(10)), True);
        assert_eq!(compare_common(Some(5), Some(10)), True);
        assert_eq!(compare_common(Some(1), Some(1)), Nil);
        assert_eq!(compare_common(Some(2), Some(1)), Nil);
        assert_eq!(compare_common(Some(2), Some(2)), Nil);
        assert_eq!(compare_common(Some(5), Some(1)), Nil);
        assert_eq!(compare_common(Some(5), Some(2)), Nil);
        assert_eq!(compare_common(Some(5), Some(5)), Nil);
        assert_eq!(compare_common(Some(10), Some(1)), Nil);
        assert_eq!(compare_common(Some(10), Some(5)), Nil);
        assert_eq!(compare_common(Some(10), Some(10)), Nil);
    }

    #[test]
    fn negative_c1_falls_off() {
        // (compare-common -3 1) = NIL — c1 not > 0, cond falls off.
        assert_eq!(compare_common(Some(-3), Some(1)), Nil);
        assert_eq!(compare_common(Some(-3), Some(2)), Nil);
        assert_eq!(compare_common(Some(-3), Some(5)), Nil);
        assert_eq!(compare_common(Some(-3), Some(10)), Nil);
        assert_eq!(compare_common(Some(-3), Some(-3)), Nil);
        // (compare-common <any> -3) when c2 != 0: third clause requires
        // c1 > 0, so c1<0 falls off; c1>0 returns (< c1 -3) = NIL for
        // any positive c1.
        assert_eq!(compare_common(Some(1), Some(-3)), Nil);
        assert_eq!(compare_common(Some(2), Some(-3)), Nil);
        assert_eq!(compare_common(Some(5), Some(-3)), Nil);
        assert_eq!(compare_common(Some(10), Some(-3)), Nil);
    }

    #[test]
    fn is_truthy_maps_nil_to_false() {
        assert!(!Nil.is_truthy());
        assert!(C1(0).is_truthy());
        assert!(C1(-3).is_truthy());
        assert!(C1(5).is_truthy());
        assert!(True.is_truthy());
    }
}

#[cfg(test)]
mod test_cull_segments {
    use super::*;
    use crate::dict::dao::KanaText;
    use crate::dict::kani::KaniWordDispatchEnum;
    use crate::dict::segment::{KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo};
    use crate::dict::text_classes::SimpleText;

    fn dummy_word(seq: i32) -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(KanaText {
            id: 0,
            seq,
            text: String::new(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        })
    }

    fn info_with_common(common: Option<i32>) -> KaniSegmentInfo {
        KaniSegmentInfo {
            posi: Vec::new(),
            seq_set: Vec::new(),
            conj: Vec::new(),
            common,
            score_info: KaniScoreInfo {
                prop_score: 0,
                kanji_break: Vec::new(),
                use_length_bonus: 0,
                split_info: KaniSplitInfo::None,
            },
            kpcl: (false, false, false, false),
        }
    }

    fn seg(seq: i32, score: i32, common: Option<Option<i32>>) -> Segment {
        Segment {
            start: 0,
            end: 1,
            word: dummy_word(seq),
            score: Some(score),
            info: common.map(info_with_common),
            top: None,
            text: None,
        }
    }

    fn scores(segs: &[Segment]) -> Vec<i32> {
        segs.iter().map(|s| s.score.unwrap()).collect()
    }

    fn seqs(segs: &[Segment]) -> Vec<i32> {
        segs.iter()
            .map(|s| match &s.word {
                KaniWordDispatchEnum::Kana(k) => k.seq,
                _ => unreachable!(),
            })
            .collect()
    }

    // REPL T1: (cull-segments nil) => NIL.
    #[test]
    fn empty_input_returns_empty() {
        let out = cull_segments(Vec::new());
        assert!(out.is_empty());
    }

    // REPL T2: single segment passes through.
    //   IN: [(score 10)] -> OUT: [(score 10)]
    #[test]
    fn single_segment_passes_through() {
        let out = cull_segments(vec![seg(1, 10, None)]);
        assert_eq!(scores(&out), vec![10]);
        assert_eq!(seqs(&out), vec![1]);
    }

    // REPL T3: descending scores with culling.
    //   IN scores [20, 15, 9, 8] -> max=20 cutoff=10 -> OUT [20, 15].
    #[test]
    fn descending_scores_cull_below_half() {
        let out = cull_segments(vec![
            seg(1, 20, None),
            seg(2, 15, None),
            seg(3, 9, None),
            seg(4, 8, None),
        ]);
        assert_eq!(scores(&out), vec![20, 15]);
        assert_eq!(seqs(&out), vec![1, 2]);
    }

    // REPL T4: identical scores — none culled, order preserved.
    //   IN scores [10, 10, 10] -> OUT [10, 10, 10].
    #[test]
    fn identical_scores_none_culled() {
        let out = cull_segments(vec![seg(1, 10, None), seg(2, 10, None), seg(3, 10, None)]);
        assert_eq!(scores(&out), vec![10, 10, 10]);
        assert_eq!(seqs(&out), vec![1, 2, 3]);
    }

    // REPL T5: unsorted input sorted by score desc.
    //   IN scores [5, 20, 15, 12] -> sorted [20, 15, 12, 5] -> max=20
    //   cutoff=10 -> OUT [20, 15, 12].
    #[test]
    fn unsorted_input_sorted_descending() {
        let out = cull_segments(vec![
            seg(1, 5, None),
            seg(2, 20, None),
            seg(3, 15, None),
            seg(4, 12, None),
        ]);
        assert_eq!(scores(&out), vec![20, 15, 12]);
        assert_eq!(seqs(&out), vec![2, 3, 4]);
    }

    // REPL T6: same score, varying :common — compare-common is the
    // primary sort key but score (all equal) is the secondary.
    // Input order [nil, 0, 10, 5] (commons), all score=10.
    //   Expected sorted by compare-common then stable score:
    //   [5, 10, 0, nil] per REPL.
    #[test]
    fn same_score_varying_commons() {
        let out = cull_segments(vec![
            seg(1, 10, Some(None)),
            seg(2, 10, Some(Some(0))),
            seg(3, 10, Some(Some(10))),
            seg(4, 10, Some(Some(5))),
        ]);
        assert_eq!(scores(&out), vec![10, 10, 10, 10]);
        // REPL output order: commons [5, 10, 0, nil] -> seqs [4, 3, 2, 1].
        assert_eq!(seqs(&out), vec![4, 3, 2, 1]);
    }

    // REPL T7: boundary — max=10 cutoff=5; score 5 stays (>= 5), 4
    // dropped.
    //   IN [10, 5, 4] -> OUT [10, 5].
    #[test]
    fn boundary_cutoff_equal_kept() {
        let out = cull_segments(vec![seg(1, 10, None), seg(2, 5, None), seg(3, 4, None)]);
        assert_eq!(scores(&out), vec![10, 5]);
    }

    // REPL T8: odd boundary — max=11 cutoff=11/2=5.5; 6 stays, 5
    // dropped.
    //   IN [11, 6, 5] -> OUT [11, 6].
    #[test]
    fn odd_boundary_cutoff_strict() {
        let out = cull_segments(vec![seg(1, 11, None), seg(2, 6, None), seg(3, 5, None)]);
        assert_eq!(scores(&out), vec![11, 6]);
    }

    // REPL T9: odd boundary with 5 below 5.5.
    //   IN [11, 5] -> OUT [11].
    #[test]
    fn odd_boundary_drops_below_half() {
        let out = cull_segments(vec![seg(1, 11, None), seg(2, 5, None)]);
        assert_eq!(scores(&out), vec![11]);
    }

    // REPL T10: max=10 cutoff=5; score 5 kept.
    //   IN [10, 5] -> OUT [10, 5].
    #[test]
    fn even_boundary_keeps_exactly_half() {
        let out = cull_segments(vec![seg(1, 10, None), seg(2, 5, None)]);
        assert_eq!(scores(&out), vec![10, 5]);
    }

    // REPL T11: zero scores — cutoff 0, all kept (0 >= 0).
    //   IN [0, 0] -> OUT [0, 0].
    #[test]
    fn zero_scores_all_kept() {
        let out = cull_segments(vec![seg(1, 0, None), seg(2, 0, None)]);
        assert_eq!(scores(&out), vec![0, 0]);
    }

    // REPL T12: negative scores — max=-5 cutoff=-2.5; -5 NOT >= -2.5
    // so loop terminates at first segment.
    //   IN [-10, -5] -> sorted [-5, -10] -> OUT [].
    #[test]
    fn negative_scores_all_culled() {
        let out = cull_segments(vec![seg(1, -10, None), seg(2, -5, None)]);
        assert!(out.is_empty());
    }

    // REPL T13: compare-common ordering on commons [nil, 5, 0, 3]
    // with all score=10. Result order (commons): [3, 5, 0, nil] per
    // REPL probe — exercises every compare-common branch:
    //   - 3 < 5 (third clause T)
    //   - 5 < 0 (second clause T)
    //   - 0 < nil (first clause returns 0, truthy)
    //   - nil never sorts before anything (first clause returns nil).
    #[test]
    fn compare_common_ordering_full() {
        let out = cull_segments(vec![
            seg(1, 10, Some(None)),
            seg(2, 10, Some(Some(5))),
            seg(3, 10, Some(Some(0))),
            seg(4, 10, Some(Some(3))),
        ]);
        // REPL order: commons [3, 5, 0, nil] -> seqs [4, 2, 3, 1].
        assert_eq!(seqs(&out), vec![4, 2, 3, 1]);
    }
}

#[cfg(test)]
mod test_get_seg_initial {
    use super::*;
    use crate::dict::conj_data::ConjData;
    use crate::dict::dao::KanaText;
    use crate::dict::kani::KaniWordDispatchEnum;
    use crate::dict::segment::{
        KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment, SegmentList,
    };
    use crate::dict::text_classes::SimpleText;

    fn dummy_word() -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(KanaText {
            id: 0,
            seq: 0,
            text: String::new(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        })
    }

    fn info_with_seq_set(seq_set: Vec<i32>) -> KaniSegmentInfo {
        KaniSegmentInfo {
            posi: vec![],
            seq_set,
            conj: vec![] as Vec<ConjData>,
            common: None,
            score_info: KaniScoreInfo {
                prop_score: 0,
                kanji_break: vec![],
                use_length_bonus: 0,
                split_info: KaniSplitInfo::None,
            },
            kpcl: (false, false, false, false),
        }
    }

    fn seg(start: usize, end: usize, seq_set: Vec<i32>) -> Segment {
        Segment {
            start,
            end,
            word: dummy_word(),
            score: None,
            info: Some(info_with_seq_set(seq_set)),
            top: None,
            text: Some(String::new()),
        }
    }

    fn lite_sl(
        start: usize,
        end: usize,
        matches: usize,
        segments: Vec<Segment>,
    ) -> Arc<KaniLiteSegmentList> {
        Arc::new(KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments,
            start,
            end,
            top: None,
            matches,
        }))
    }

    fn assert_seq_sets(actual: &KaniLiteSegmentList, expected: &[Vec<i32>]) {
        assert_eq!(actual.segments.len(), expected.len());
        for (i, exp) in expected.iter().enumerate() {
            assert_eq!(&actual.segments[i].seq_set, exp, "segments[{}]", i);
        }
    }

    #[test]
    fn a1_empty_segment_list_returns_passthrough() {
        let r = lite_sl(0, 0, 0, vec![]);
        let got = get_seg_initial(&r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].start, 0);
        assert_eq!(got[0].end, 0);
        assert!(got[0].segments.is_empty());
    }

    #[test]
    fn a2_seq_not_in_any_segfilter_returns_one_unchanged() {
        let r = lite_sl(0, 2, 0, vec![seg(0, 2, vec![999])]);
        let got = get_seg_initial(&r);
        assert_eq!(got.len(), 1);
        assert_seq_sets(&got[0], &[vec![999]]);
    }

    #[test]
    fn a3_aux_verb_only_seg_yields_zero_splits() {
        let r = lite_sl(0, 2, 0, vec![seg(0, 2, vec![1342560])]);
        let got = get_seg_initial(&r);
        assert!(got.is_empty());
    }

    #[test]
    fn a4_matches_field_carries_through_unchanged() {
        let r = lite_sl(0, 2, 7, vec![seg(0, 2, vec![999])]);
        let got = get_seg_initial(&r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].matches, 7);
        assert_seq_sets(&got[0], &[vec![999]]);
    }

    #[test]
    fn a5_mixed_aux_and_normal_yields_filtered_subset() {
        // dict-grammar.lisp:1047-1054 — seg-left=nil + non-empty
        // satisfies-right → clause-2 pushes (nil, mslf(r, contradicts-right)).
        let r = lite_sl(
            0,
            2,
            0,
            vec![seg(0, 2, vec![1342560]), seg(0, 2, vec![999])],
        );
        let got = get_seg_initial(&r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].segments.len(), 1);
        assert!(got[0].segments[0].seq_set.contains(&999));
        assert!(!got[0].segments[0].seq_set.contains(&1342560));
    }
}

#[cfg(test)]
mod test_get_seg_splits {
    use super::*;
    use crate::dict::conj_data::ConjData;
    use crate::dict::dao::{ConjProp, KanaText};
    use crate::dict::grammar::synergy::Synergy;
    use crate::dict::kani::KaniWordDispatchEnum;
    use crate::dict::segment::{
        KaniScoreInfo, KaniSegmentInfo, KaniSplitInfo, Segment, SegmentList,
    };
    use crate::dict::text_classes::SimpleText;

    fn dummy_word() -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(KanaText {
            id: 0,
            seq: 0,
            text: String::new(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        })
    }

    fn cdata(conj_type: i32) -> ConjData {
        ConjData {
            seq: None,
            from: None,
            via: None,
            prop: Some(ConjProp {
                id: 0,
                conj_id: 0,
                conj_type,
                pos: String::new(),
                neg: None,
                fml: None,
            }),
            src_map: vec![],
        }
    }

    fn info(
        seq_set: Vec<i32>,
        conj: Vec<ConjData>,
        posi: Vec<&str>,
        kpcl: (bool, bool, bool, bool),
    ) -> KaniSegmentInfo {
        KaniSegmentInfo {
            posi: posi.into_iter().map(String::from).collect(),
            seq_set,
            conj,
            common: None,
            score_info: KaniScoreInfo {
                prop_score: 0,
                kanji_break: vec![],
                use_length_bonus: 0,
                split_info: KaniSplitInfo::None,
            },
            kpcl,
        }
    }

    fn seg(start: usize, end: usize, info: KaniSegmentInfo, text: &str) -> Segment {
        Segment {
            start,
            end,
            word: dummy_word(),
            score: None,
            info: Some(info),
            top: None,
            text: Some(text.to_string()),
        }
    }

    fn lite_sl(start: usize, end: usize, segments: Vec<Segment>) -> Arc<KaniLiteSegmentList> {
        Arc::new(KaniLiteSegmentList::from_segment_list(&SegmentList {
            segments,
            start,
            end,
            top: None,
            matches: 0,
        }))
    }

    fn unwrap_sl(elem: &KaniLitePathElement) -> &Arc<KaniLiteSegmentList> {
        match elem {
            KaniLitePathElement::SegmentList(sl) => sl,
            other => panic!("expected SegmentList, got {:?}", other),
        }
    }

    fn unwrap_synergy(elem: &KaniLitePathElement) -> &Synergy {
        match elem {
            KaniLitePathElement::Synergy(s) => s,
            other => panic!("expected Synergy, got {:?}", other),
        }
    }

    // REPL probes (`/tmp/probe_gss_synth*.lisp` on .103, 2026-05-19).

    #[test]
    fn a_no_penalty_no_synergy_yields_one_fallback_outer() {
        let l = lite_sl(
            0,
            3,
            vec![seg(
                0,
                3,
                info(vec![9999], vec![], vec![], (true, false, false, false)),
                "abc",
            )],
        );
        let r = lite_sl(
            3,
            6,
            vec![seg(
                3,
                6,
                info(vec![8888], vec![], vec![], (true, false, false, false)),
                "def",
            )],
        );
        let got = get_seg_splits(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].len(), 2);
        assert_eq!(unwrap_sl(&got[0][0]).start, 3);
        assert_eq!(unwrap_sl(&got[0][0]).end, 6);
        assert_eq!(unwrap_sl(&got[0][1]).start, 0);
        assert_eq!(unwrap_sl(&got[0][1]).end, 3);
    }

    #[test]
    fn b_penalty_short_only_yields_one_penalty_outer() {
        let l = lite_sl(
            0,
            1,
            vec![seg(
                0,
                1,
                info(vec![9999], vec![], vec![], (false, false, false, false)),
                "あ",
            )],
        );
        let r = lite_sl(
            3,
            4,
            vec![seg(
                3,
                4,
                info(vec![8888], vec![], vec![], (false, false, false, false)),
                "い",
            )],
        );
        let got = get_seg_splits(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].len(), 3);
        assert_eq!(unwrap_sl(&got[0][0]).start, 3);
        assert_eq!(unwrap_sl(&got[0][0]).end, 4);
        let syn = unwrap_synergy(&got[0][1]);
        assert_eq!(syn.description.as_deref(), Some("short"));
        assert_eq!(syn.score, -9);
        assert_eq!(syn.start, 1);
        assert_eq!(syn.end, 3);
        assert_eq!(unwrap_sl(&got[0][2]).start, 0);
        assert_eq!(unwrap_sl(&got[0][2]).end, 1);
    }

    #[test]
    fn c_synergy_no_adjectives_only_yields_fallback_plus_synergy() {
        let l = lite_sl(
            0,
            1,
            vec![seg(
                0,
                1,
                info(vec![], vec![], vec!["adj-no"], (true, false, false, false)),
                "x",
            )],
        );
        let r = lite_sl(
            1,
            2,
            vec![seg(
                1,
                2,
                info(vec![1469800], vec![], vec![], (false, false, false, false)),
                "y",
            )],
        );
        let got = get_seg_splits(&l, &r);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].len(), 2);
        assert_eq!(unwrap_sl(&got[0][0]).start, 1);
        assert_eq!(unwrap_sl(&got[0][1]).start, 0);
        assert_eq!(got[1].len(), 3);
        let syn = unwrap_synergy(&got[1][1]);
        assert_eq!(syn.description.as_deref(), Some("no-adjective"));
        assert_eq!(syn.score, 15);
        assert_eq!(syn.start, 1);
        assert_eq!(syn.end, 1);
    }

    #[test]
    fn d_aux_verb_segfilter_split_yields_two_fallback_outers() {
        let l = lite_sl(
            0,
            2,
            vec![
                seg(
                    0,
                    2,
                    info(
                        vec![],
                        vec![cdata(13)],
                        vec![],
                        (false, false, false, false),
                    ),
                    "x1",
                ),
                seg(
                    0,
                    2,
                    info(vec![], vec![cdata(3)], vec![], (false, false, false, false)),
                    "x2",
                ),
            ],
        );
        let r = lite_sl(
            2,
            4,
            vec![
                seg(
                    2,
                    4,
                    info(vec![1342560], vec![], vec![], (false, false, false, false)),
                    "y1",
                ),
                seg(
                    2,
                    4,
                    info(vec![999], vec![], vec![], (false, false, false, false)),
                    "y2",
                ),
            ],
        );
        let got = get_seg_splits(&l, &r);
        assert_eq!(got.len(), 2);
        for outer in &got {
            assert_eq!(outer.len(), 2);
            assert_eq!(unwrap_sl(&outer[0]).start, 2);
            assert_eq!(unwrap_sl(&outer[1]).start, 0);
        }
    }

    #[test]
    fn e_non_adjacent_blocks_synergy_keeps_fallback() {
        let l = lite_sl(
            0,
            1,
            vec![seg(
                0,
                1,
                info(vec![], vec![], vec!["adj-no"], (true, false, false, false)),
                "x",
            )],
        );
        let r = lite_sl(
            3,
            4,
            vec![seg(
                3,
                4,
                info(vec![1469800], vec![], vec![], (false, false, false, false)),
                "y",
            )],
        );
        let got = get_seg_splits(&l, &r);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].len(), 2);
        assert_eq!(unwrap_sl(&got[0][0]).start, 3);
        assert_eq!(unwrap_sl(&got[0][1]).start, 0);
    }

    #[test]
    fn f_penalty_semi_final_plus_synergy_no_adjectives() {
        let l = lite_sl(
            0,
            1,
            vec![seg(
                0,
                1,
                info(
                    vec![2029110],
                    vec![],
                    vec!["adj-no"],
                    (true, false, false, false),
                ),
                "x",
            )],
        );
        let r = lite_sl(
            1,
            2,
            vec![seg(
                1,
                2,
                info(vec![1469800], vec![], vec![], (false, false, false, false)),
                "y",
            )],
        );
        let got = get_seg_splits(&l, &r);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].len(), 3);
        let syn0 = unwrap_synergy(&got[0][1]);
        assert_eq!(syn0.description.as_deref(), Some("semi-final not final"));
        assert_eq!(syn0.score, -15);
        assert_eq!(got[1].len(), 3);
        let syn1 = unwrap_synergy(&got[1][1]);
        assert_eq!(syn1.description.as_deref(), Some("no-adjective"));
        assert_eq!(syn1.score, 15);
    }
}

#[cfg(test)]
mod test_get_segment_score {
    //! All assertions back-checked via REPL on the .103 SBCL — see
    //! `/tmp/probe_gss.lisp` 2026-05-17 run.
    use super::*;
    use crate::dict::dao::KanaText;
    use crate::dict::kani::KaniWordDispatchEnum;
    use crate::dict::text_classes::SimpleText;

    fn dummy_word() -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(KanaText {
            id: 0,
            seq: 0,
            text: String::new(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: false,
            nokanji: false,
            best_kanji: None,
            state: SimpleText::default(),
        })
    }

    fn seg(score: Option<i32>) -> Segment {
        Segment {
            start: 0,
            end: 1,
            word: dummy_word(),
            score,
            info: None,
            top: None,
            text: None,
        }
    }

    #[test]
    fn synergy_returns_score() {
        // REPL: get-segment-score on synergy with score=7 -> 7
        let s = Synergy {
            description: Some("x".into()),
            connector: Some(String::new()),
            score: 7,
            start: 0,
            end: 1,
        };
        assert_eq!(
            get_segment_score(&KaniSegmentScoreArg::Synergy(&s)),
            Some(7)
        );
    }

    #[test]
    fn segment_returns_score_when_present() {
        // REPL: segment with score=13 -> 13
        let s = seg(Some(13));
        assert_eq!(
            get_segment_score(&KaniSegmentScoreArg::Segment(&s)),
            Some(13)
        );
    }

    #[test]
    fn segment_returns_none_when_score_unset() {
        // REPL: segment with no score -> NIL
        let s = seg(None);
        assert_eq!(get_segment_score(&KaniSegmentScoreArg::Segment(&s)), None);
    }

    #[test]
    fn empty_segment_list_returns_zero() {
        // REPL: segment-list with no segments -> 0
        let sl = SegmentList {
            segments: Vec::new(),
            start: 0,
            end: 1,
            top: None,
            matches: 0,
        };
        assert_eq!(
            get_segment_score(&KaniSegmentScoreArg::SegmentList(&sl)),
            Some(0)
        );
    }

    #[test]
    fn segment_list_returns_first_segment_score() {
        // REPL: segment-list with two segs (99, 50) -> 99
        let sl = SegmentList {
            segments: vec![seg(Some(99)), seg(Some(50))],
            start: 0,
            end: 1,
            top: None,
            matches: 0,
        };
        assert_eq!(
            get_segment_score(&KaniSegmentScoreArg::SegmentList(&sl)),
            Some(99)
        );
    }

    #[test]
    fn segment_list_returns_none_when_first_segment_score_unset() {
        // REPL: segment-list with one nil-score seg -> NIL
        let sl = SegmentList {
            segments: vec![seg(None)],
            start: 0,
            end: 1,
            top: None,
            matches: 0,
        };
        assert_eq!(
            get_segment_score(&KaniSegmentScoreArg::SegmentList(&sl)),
            None
        );
    }

    #[test]
    fn single_segment_list_returns_that_score() {
        // REPL: segment-list with one seg (42) -> 42
        let sl = SegmentList {
            segments: vec![seg(Some(42))],
            start: 0,
            end: 1,
            top: None,
            matches: 0,
        };
        assert_eq!(
            get_segment_score(&KaniSegmentScoreArg::SegmentList(&sl)),
            Some(42)
        );
    }
}

#[cfg(test)]
mod test_dict_segment {
    //! Unit tests against the real .103 PG via `KaniranContext::from_env()`.
    //! Expected paths / scores captured from `ichiran/dict:dict-segment` on
    //! the capture host. Coverage:
    //! - multi-path result (loop runs N times), scores descending
    //! - `:limit` caps the number of paths and is forwarded to find-best-path
    //! - default limit (None) resolves to 5
    //! - empty string yields one seed path with an empty word-info-list
    //! - all-gap input yields one path with the gap-penalty score
    use super::*;
    use crate::dict::word_info::WordInfoType;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    fn texts(word_info_list: &[WordInfo]) -> Vec<String> {
        word_info_list
            .iter()
            .map(|wi| {
                if wi.kind == WordInfoType::Gap {
                    ":GAP".to_string()
                } else {
                    wi.text.clone()
                }
            })
            .collect()
    }

    #[tokio::test]
    async fn multi_path_scores_descending() {
        let ctx = ctx_from_env().await;
        let result = dict_segment(&ctx, "しませんか", Some(3)).await.unwrap();
        let scores: Vec<i32> = result.iter().map(|(_, score)| *score).collect();
        assert_eq!(scores, vec![352, 52, 48]);
        assert_eq!(texts(&result[0].0), vec!["しません", "か"]);
        assert_eq!(texts(&result[1].0), vec!["しま", "せんか"]);
        assert_eq!(texts(&result[2].0), vec!["しま", "せん", "か"]);
    }

    #[tokio::test]
    async fn limit_one_returns_single_best_path() {
        let ctx = ctx_from_env().await;
        let result = dict_segment(&ctx, "しませんか", Some(1)).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, 352);
        assert_eq!(texts(&result[0].0), vec!["しません", "か"]);
    }

    #[tokio::test]
    async fn default_limit_is_five() {
        let ctx = ctx_from_env().await;
        let result = dict_segment(&ctx, "ご注文はうさぎですか", None)
            .await
            .unwrap();
        let scores: Vec<i32> = result.iter().map(|(_, score)| *score).collect();
        assert_eq!(scores, vec![518, 504, 485, 474, 465]);
        assert_eq!(
            texts(&result[0].0),
            vec!["ご注文", "は", "うさぎ", "です", "か"]
        );
    }

    #[tokio::test]
    async fn empty_string_seeds_one_empty_path() {
        let ctx = ctx_from_env().await;
        let result = dict_segment(&ctx, "", Some(5)).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, 0);
        assert!(result[0].0.is_empty());
    }

    #[tokio::test]
    async fn all_gap_input_one_path_with_gap_penalty() {
        let ctx = ctx_from_env().await;
        let result = dict_segment(&ctx, "abcde", Some(5)).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, -2500);
        assert_eq!(texts(&result[0].0), vec![":GAP"]);
    }
}

#[cfg(test)]
mod test_simple_segment {
    //! Mirror of the upstream `segmentation-test` (`tests.lisp:39`), the
    //! canonical unit test for `simple-segment`. Each case maps the
    //! returned word-infos to their text (or `GAP` for gap segments) and
    //! compares against the upstream-asserted segmentation. Runs against
    //! the real .103 PG via `KaniranContext::from_env()`. All 541 cases
    //! pass upstream on the host DB (verified via `run-parallel-tests`);
    //! the 4 commented-out upstream cases are omitted.
    use super::*;
    use crate::dict::word_info::WordInfoType;

    const GAP: &str = ":GAP";

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    // tests.lisp:33 (assert-segment) — :gap word-infos map to GAP, others
    // to their text.
    fn segmentation(word_info_list: &[WordInfo]) -> Vec<&str> {
        word_info_list
            .iter()
            .map(|wi| {
                if wi.kind == WordInfoType::Gap {
                    GAP
                } else {
                    wi.text.as_str()
                }
            })
            .collect()
    }

    #[tokio::test]
    async fn segmentation_test() {
        let ctx = ctx_from_env().await;
        // tests.lisp:39 (define-parallel-test segmentation-test)
        let cases: &[(&str, &[&str])] = &[
            (
                "ご注文はうさぎですか",
                &["ご注文", "は", "うさぎ", "です", "か"],
            ),
            ("しませんか", &["しません", "か"]),
            ("ドンマイ", &["ドンマイ"]),
            ("みんな土足でおいで", &["みんな", "土足で", "おいで"]),
            ("おもわぬオチ提供中", &["おもわぬ", "オチ", "提供", "中"]),
            ("わたし", &["わたし"]),
            (
                "お姉ちゃんにまかせて地球まるごと",
                &["お姉ちゃん", "に", "まかせて", "地球", "まるごと"],
            ),
            ("名人になってるはず", &["名人", "に", "なってる", "はず"]),
            ("いいとこ", &["いいとこ"]),
            ("そういうお隣どうし", &["そういう", "お", "隣どうし"]),
            ("はしゃいじゃう", &["はしゃいじゃう"]),
            ("分かっちゃうのよ", &["分かっちゃう", "の", "よ"]),
            (
                "懐かしく新しいまだそしてまた",
                &["懐かしく", "新しい", "まだ", "そして", "また"],
            ),
            (
                "あたりまえみたいに思い出いっぱい",
                &["あたりまえ", "みたい", "に", "思い出", "いっぱい"],
            ),
            (
                "何でもない日々とっておきのメモリアル",
                &["何でもない", "日々", "とっておき", "の", "メモリアル"],
            ),
            (
                "しつれいしなければならないんです",
                &["しつれいし", "なければならない", "ん", "です"],
            ),
            (
                "だけど気付けば馴染んじゃってる",
                &["だけど", "気付けば", "馴染んじゃってる"],
            ),
            ("飲んで笑っちゃえば", &["飲んで", "笑っちゃえば"]),
            ("なんで", &["なんで"]),
            ("遠慮しないでね", &["遠慮しないで", "ね"]),
            ("出かけるまえに", &["出かける", "まえ", "に"]),
            ("感じたいでしょ", &["感じたい", "でしょ"]),
            ("まじで", &["まじ", "で"]),
            (
                "その山を越えたとき",
                &["その", "山", "を", "越えた", "とき"],
            ),
            ("遊びたいのに", &["遊びたい", "のに"]),
            ("しながき", &["しながき"]),
            ("楽しさ求めて", &["楽しさ", "求めて"]),
            ("日常のなかにも", &["日常", "の", "なかにも"]),
            (
                "ほんとは好きなんだと",
                &["ほんと", "は", "好き", "な", "ん", "だ", "と"],
            ),
            ("内緒なの", &["内緒", "なの"]),
            ("魚が好きじゃない", &["魚", "が", "好き", "じゃない"]),
            ("物語になってく", &["物語", "に", "なってく"]),
            ("書いてきてくださった", &["書いてきて", "くださった"]),
            ("今日は何の日", &["今日", "は", "何の", "日"]),
            ("何から話そうか", &["何", "から", "話そう", "か"]),
            ("話したくなる", &["話したくなる"]),
            ("進化してく友情", &["進化してく", "友情"]),
            ("私に任せてくれ", &["私", "に", "任せてくれ"]),
            (
                "時までに帰ってくると約束してくれるのなら外出してよろしい",
                &[
                    "時",
                    "まで",
                    "に",
                    "帰ってくる",
                    "と",
                    "約束してくれる",
                    "の",
                    "なら",
                    "外出して",
                    "よろしい",
                ],
            ),
            (
                "雪が降りそうな気がします",
                &["雪", "が", "降りそう", "な", "気がします"],
            ),
            ("新しそうだ", &["新しそう", "だ"]),
            (
                "本を読んだりテレビを見たりします",
                &["本", "を", "読んだり", "テレビ", "を", "見たり", "します"],
            ),
            (
                "今日母はたぶんうちにいるでしょう",
                &[
                    "今日",
                    "母",
                    "は",
                    "たぶん",
                    "うち",
                    "に",
                    "いる",
                    "でしょう",
                ],
            ),
            ("赤かったろうです", &["赤かったろう", "です"]),
            ("そう呼んでくれていい", &["そう", "呼んでくれていい"]),
            ("払わなくてもいい", &["払わなくてもいい"]),
            (
                "体に悪いと知りながらタバコをやめることはできない",
                &[
                    "体に悪い",
                    "と",
                    "知り",
                    "ながら",
                    "タバコをやめる",
                    "こと",
                    "は",
                    "できない",
                ],
            ),
            ("微笑みはまぶしすぎる", &["微笑み", "は", "まぶしすぎる"]),
            ("なにをしていますか", &["なに", "を", "しています", "か"]),
            (
                "優しすぎそのうえカッコいいの",
                &["優しすぎ", "そのうえ", "カッコいい", "の"],
            ),
            (
                "この本は複雑すぎるから",
                &["この", "本", "は", "複雑", "すぎる", "から"],
            ),
            ("かわいいです", &["かわいいです"]),
            ("学生なんだ", &["学生", "な", "ん", "だ"]),
            ("なんだから", &["な", "ん", "だから"]),
            ("名付けたい", &["名付けたい"]),
            ("切なくなってしまう", &["切なくなってしまう"]),
            ("らいかな", &["らい", "かな"]),
            ("誰かいなくなった", &["誰か", "いなくなった"]),
            ("思い出すな", &["思い出す", "な"]),
            ("かなって思ったら", &["かなって", "思ったら"]),
            (
                "法律にかなっているさま",
                &["法律", "に", "かなっている", "さま"],
            ),
            ("ことすら難しい", &["こと", "すら", "難しい"]),
            ("投下しました", &["投下しました"]),
            ("車止める", &["車", "止める"]),
            ("円盤はただの", &["円盤", "は", "ただ", "の"]),
            (
                "ズボンからすねをむき出しにする",
                &["ズボン", "から", "すね", "を", "むき", "出しにする"],
            ),
            (
                "駅の前で会いましょう",
                &["駅", "の", "前", "で", "会いましょう"],
            ),
            (
                "あなたの質問は答えにくい",
                &["あなた", "の", "質問", "は", "答えにくい"],
            ),
            ("とかそういう", &["とか", "そういう"]),
            ("好評のうちに", &["好評", "の", "うち", "に"]),
            (
                "映像もすごくよかったです",
                &["映像", "も", "すごく", "よかったです"],
            ),
            ("情けねえ", &["情けねえ"]),
            ("春ですねえ", &["春", "です", "ねえ"]),
            ("春ですねぇ", &["春", "です", "ねぇ"]),
            ("きつねじゃなかった", &["きつね", "じゃなかった"]),
            (
                "ワシじゃなくて和紙じゃよ",
                &["ワシ", "じゃなくて", "和紙", "じゃ", "よ"],
            ),
            ("ほうがいいよ", &["ほうがいい", "よ"]),
            (
                "痛さはどれくらいですか",
                &["痛さ", "は", "どれくらい", "です", "か"],
            ),
            ("見てくれたかな", &["見てくれた", "かな"]),
            ("とても良かった", &["とても", "良かった"]),
            (
                "戻りたいかと言われる",
                &["戻りたい", "か", "と", "言われる"],
            ),
            (
                "こういうのでいいんだよ",
                &["こういう", "の", "でいい", "ん", "だ", "よ"],
            ),
            (
                "そんなのでいいと思ってるの",
                &["そんな", "の", "でいい", "と", "思ってる", "の"],
            ),
            ("だけが墓参りしてた", &["だけ", "が", "墓参りしてた"]),
            ("はいいんだけどな", &["は", "いい", "ん", "だけど", "な"]),
            ("なりつつあるんだが", &["なりつつある", "ん", "だが"]),
            ("反論は認めません", &["反論", "は", "認めません"]),
            ("見たような気がする", &["見た", "ような気がする"]),
            (
                "幽霊を見たような顔つきをしていた",
                &["幽霊", "を", "見た", "ような", "顔つき", "を", "していた"],
            ),
            ("元気になる", &["元気", "に", "なる"]),
            ("半端なかった", &["半端なかった"]),
            ("一人ですね", &["一人", "です", "ね"]),
            ("行事がある", &["行事", "が", "ある"]),
            ("当てられたものになる", &["当てられた", "ものになる"]),
            ("獲得しうる", &["獲得しうる"]),
            ("ことができず", &["ことができず"]),
            (
                "一生一度だけの忘られぬ約束",
                &["一生一度", "だけ", "の", "忘られぬ", "約束"],
            ),
            (
                "やらずにこの路線でよかったのに",
                &["やらず", "に", "この", "路線", "で", "よかった", "のに"],
            ),
            ("歌ってしまいそう", &["歌ってしまいそう"]),
            ("しまいそう", &["しまいそう"]),
            ("まいそう祭り", &["まいそう", "祭り"]),
            ("何ですか", &["何", "です", "か"]),
            ("浮かれたいから", &["浮かれたい", "から"]),
            ("なくなっちゃう", &["なくなっちゃう"]),
            ("になりそうだけど", &["に", "なりそう", "だけど"]),
            (
                "これは辛い選択になりそうだな",
                &["これ", "は", "辛い", "選択", "に", "なりそう", "だ", "な"],
            ),
            ("はっきりしそうだな", &["はっきりしそう", "だ", "な"]),
            ("泣きそうなんだけど", &["泣きそう", "な", "ん", "だけど"]),
            ("これですね", &["これ", "です", "ね"]),
            ("はいなくなります", &["は", "いなくなります"]),
            ("忘れなく", &["忘れなく"]),
            ("じゃないですか", &["じゃないです", "か"]),
            ("純粋さ健気さ", &["純粋さ", "健気さ"]),
            ("着てたからね", &["着てた", "から", "ね"]),
            (
                "仕出かすからだと思います",
                &["仕出かす", "から", "だ", "と", "思います"],
            ),
            ("みんながした", &["みんな", "が", "した"]),
            ("ほうが速いと", &["ほう", "が", "速い", "と"]),
            ("注意してください", &["注意してください"]),
            (
                "昨日といいどうしてこう",
                &["昨日", "と", "いい", "どうして", "こう"],
            ),
            ("いっぱいきそう", &["いっぱい", "きそう"]),
            ("仲良しになったら", &["仲良し", "に", "なったら"]),
            ("全くといっていい", &["全く", "と", "いって", "いい"]),
            ("発狂しそうなんだ", &["発狂しそう", "な", "ん", "だ"]),
            ("していたんだ", &["していた", "ん", "だ"]),
            ("引き上げられた", &["引き上げられた"]),
            ("をつかむため", &["を", "つかむ", "ため"]),
            ("ときが自分", &["とき", "が", "自分"]),
            ("もうこころ", &["もう", "こころ"]),
            ("届けしたら", &["届け", "したら"]),
            (
                "おまえら低いんだよ",
                &["おまえら", "低い", "ん", "だ", "よ"],
            ),
            (
                "すべてがかかっていると思いながら",
                &["すべて", "が", "かかっている", "と", "思い", "ながら"],
            ),
            ("エロいと思っちゃう", &["エロい", "と", "思っちゃう"]),
            ("変わり映えしない", &["変わり映えしない"]),
            (
                "あなたがいなきゃこんな計画思いつかなかった",
                &[
                    "あなた",
                    "が",
                    "いなきゃ",
                    "こんな",
                    "計画",
                    "思いつかなかった",
                ],
            ),
            ("見たかったです", &["見たかったです"]),
            ("出来て楽しかったな", &["出来て", "楽しかった", "な"]),
            ("つかってください", &["つかってください"]),
            ("誰もが思ってた", &["誰も", "が", "思ってた"]),
            ("参考にしたらしい", &["参考にしたらしい"]),
            ("狙いやすそうで", &["狙い", "やすそう", "で"]),
            (
                "予定はございませんので",
                &["予定", "は", "ございません", "ので"],
            ),
            (
                "犬はトラックにはねられた",
                &["犬", "は", "トラック", "に", "はねられた"],
            ),
            ("仕事してください", &["仕事してください"]),
            ("おいかけっこしましょ", &["おい", "かけっこしましょ"]),
            (
                "イラストカードが付きます",
                &["イラスト", "カード", "が", "付きます"],
            ),
            ("じゃないかしら", &["じゃない", "かしら"]),
            ("いつか本当に", &["いつか", "本当に"]),
            ("言い方もします", &["言い方", "も", "します"]),
            ("何でこれ", &["何で", "これ"]),
            (
                "こういう物語ができるんだ",
                &["こういう", "物語", "が", "できる", "ん", "だ"],
            ),
            (
                "といったところでしょうか",
                &["といった", "ところ", "でしょうか"],
            ),
            ("広めたいと思っている", &["広めたい", "と", "思っている"]),
            ("のせいかな", &["の", "せい", "かな"]),
            ("その場合", &["その", "場合"]),
            ("教えてくれてありがとう", &["教えてくれて", "ありがとう"]),
            (
                "彼が来るかどうか疑問だ",
                &["彼", "が", "来る", "かどうか", "疑問", "だ"],
            ),
            (
                "泳ぎに行ってはどうかな",
                &["泳ぎ", "に", "行って", "は", "どうかな"],
            ),
            (
                "どうか僕を許して下さい",
                &["どうか", "僕", "を", "許して", "下さい"],
            ),
            ("鏡はいらないですよ", &["鏡", "は", "いらないです", "よ"]),
            (
                "ベッドで跳ねちゃいけません",
                &["ベッド", "で", "跳ねちゃ", "いけません"],
            ),
            (
                "お酒を飲んじゃだめです",
                &["お酒", "を", "飲んじゃ", "だめ", "です"],
            ),
            ("これ洗濯しといて", &["これ", "洗濯しといて"]),
            (
                "来週までに読んどいて",
                &["来週", "まで", "に", "読んどいて"],
            ),
            (
                "奴がまともに見られない",
                &["奴", "が", "まともに", "見られない"],
            ),
            ("間違いなし", &["間違いなし"]),
            ("見ませんでしょうか", &["見ません", "でしょうか"]),
            (
                "書いていただけませんでしょうか",
                &["書いていただけません", "でしょうか"],
            ),
            ("友達できる", &["友達", "できる"]),
            ("実はそうなんだ", &["実は", "そう", "なんだ"]),
            ("やらしいです", &["やらしいです"]),
            ("荒いとこもある", &["荒い", "とこ", "も", "ある"]),
            ("あったかいとこ行こう", &["あったかい", "とこ", "行こう"]),
            ("ぶっちゃけ話", &["ぶっちゃけ", "話"]),
            ("いけないわー", &["いけない", "わ", GAP]),
            (
                "社長としてやっていけないわ",
                &["社長", "として", "やっていけない", "わ"],
            ),
            ("よくわかんないけど", &["よく", "わかんない", "けど"]),
            (
                "ほうがいいんじゃないの",
                &["ほうがいい", "ん", "じゃない", "の"],
            ),
            ("こんなんじゃ", &["こんなん", "じゃ"]),
            ("増やしたほうがいいな", &["増やした", "ほうがいい", "な"]),
            ("屈しやすいものだ", &["屈し", "やすい", "もの", "だ"]),
            ("目をもっている", &["目", "を", "もっている"]),
            (
                "これが君のなすべきものだ",
                &["これ", "が", "君", "の", "なすべき", "もの", "だ"],
            ),
            ("泥棒をつかまえた", &["泥棒", "を", "つかまえた"]),
            (
                "金もないし友達もいません",
                &["金", "も", "ない", "し", "友達", "も", "いません"],
            ),
            (
                "出来たからほら見てよ",
                &["出来た", "から", "ほら", "見て", "よ"],
            ),
            (
                "眠いからもう寝るね",
                &["眠い", "から", "もう", "寝る", "ね"],
            ),
            ("浮気してやがった", &["浮気してやがった"]),
            ("見本通りに", &["見本", "通り", "に"]),
            ("不適応", &["不", "適応"]),
            ("良いそうです", &["良い", "そう", "です"]),
            ("むらむらとわいた", &["むらむら", "と", "わいた"]),
            ("否定しちゃいけない", &["否定しちゃ", "いけない"]),
            ("観たいです", &["観たいです"]),
            ("あんたはわからん", &["あんた", "は", "わからん"]),
            ("見られたくないとこ", &["見られたくない", "とこ"]),
            ("多分家で", &["多分", "家", "で"]),
            ("三十八", &["三十八"]),
            (
                "エロそうだヤバそうだ",
                &["エロそう", "だ", "ヤバそう", "だ"],
            ),
            ("私にとっても", &["私", "にとって", "も"]),
            (
                "睡眠を十分にとってください",
                &["睡眠", "を", "十分", "に", "とってください"],
            ),
            ("そうなんだけど", &["そう", "な", "ん", "だけど"]),
            ("進んでない", &["進んでない"]),
            (
                "一回だけであとは言わない",
                &["一回", "だけ", "で", "あと", "は", "言わない"],
            ),
            (
                "ご親切に恐縮しております",
                &["ご親切に", "恐縮しております"],
            ),
            (
                "官吏となっておる者がある",
                &["官吏", "と", "なっておる", "者", "が", "ある"],
            ),
            (
                "間違えておられたようですね",
                &["間違えておられた", "ようです", "ね"],
            ),
            ("人気のせいな", &["人気", "の", "せい", "な"]),
            ("コレはアレ", &["コレ", "は", "アレ"]),
            ("アレハレ", &["アレ", GAP]),
            (
                "上に文字があったり",
                &["上", "に", "文字", "が", "あったり"],
            ),
            ("言っただろ", &["言った", "だろ"]),
            (
                "嵐が起ころうとしている",
                &["嵐", "が", "起ころうとしている"],
            ),
            ("知らないでしょう", &["知らないでしょう"]),
            ("読まないでしょう", &["読まないでしょう"]),
            ("来ないでしょう", &["来ないでしょう"]),
            ("何もかもがめんどい", &["何もかも", "が", "めんどい"]),
            ("なにもかもがめんどい", &["なにもかも", "が", "めんどい"]),
            (
                "あいつ規制されりゃいいのに",
                &["あいつ", "規制されりゃ", "いい", "のに"],
            ),
            (
                "塗ってみようと思って",
                &["塗って", "みよう", "と", "思って"],
            ),
            ("肩を並べられなかった", &["肩を並べられなかった"]),
            ("じゃなくて良かった", &["じゃなくて", "良かった"]),
            ("申し訳なさそう", &["申し訳なさそう"]),
            ("決まってたし", &["決まってた", "し"]),
            ("決まっている", &["決まっている"]),
            ("恐れ入りました", &["恐れ入りました"]),
            ("はうまい", &["は", "うまい"]),
            ("弾け飛びました", &["弾け飛びました"]),
            ("ぶっこんでいるようで", &["ぶっこんでいる", "よう", "で"]),
            ("じゃないけど下手に", &["じゃない", "けど", "下手", "に"]),
            ("的にそうではない", &["的", "に", "そう", "ではない"]),
            ("入り込めなかった", &["入り込めなかった"]),
            ("がいまいちなんだよ", &["が", "いまいち", "なんだ", "よ"]),
            ("脱がしにかかってる", &["脱がし", "に", "かかってる"]),
            ("必死になってる", &["必死", "に", "なってる"]),
            ("安心させた", &["安心させた"]),
            ("人が好きそうだ", &["人", "が", "好き", "そう", "だ"]),
            ("もっていこうとする", &["もっていこうとする"]),
            ("増やして", &["増やして"]),
            ("ぜいたくで", &["ぜいたく", "で"]),
            ("したくらいで", &["したくらい", "で"]),
            ("でもうまく人", &["でも", "うまく", "人"]),
            (
                "好き嫌いもしないように",
                &["好き嫌い", "も", "しない", "ように"],
            ),
            ("のどこが思える", &["の", "どこ", "が", "思える"]),
            ("出会えて良かった", &["出会えて", "良かった"]),
            ("無理しなくていいから", &["無理しなくていい", "から"]),
            ("調子にのらないほうが", &["調子にのらない", "ほう", "が"]),
            ("こなさそう", &["こなさそう"]),
            ("伸びてこなさそう", &["伸びてこなさそう"]),
            ("手にとって", &["手にとって"]),
            ("平和である", &["平和", "で", "ある"]),
            (
                "私にとっては少しおかしいです",
                &["私", "にとって", "は", "少し", "おかしいです"],
            ),
            ("パーティーは", &["パーティー", "は"]),
            (
                "彼以上のばかはいない",
                &["彼", "以上", "の", "ばか", "は", "いない"],
            ),
            (
                "君がいないと淋しい",
                &["君", "が", "いない", "と", "淋しい"],
            ),
            ("思いきって", &["思いきって"]),
            ("思いきっている", &["思いきっている"]),
            ("大事になります", &["大事", "に", "なります"]),
            ("元気にします", &["元気", "に", "します"]),
            (
                "ご迷惑おかけしてすみません",
                &["ご迷惑", "おかけして", "すみません"],
            ),
            (
                "不便をおかけすることを謝ります",
                &["不便", "を", "おかけする", "こと", "を", "謝ります"],
            ),
            (
                "お手数おかけし申し訳ないが",
                &["お手数", "おかけし", "申し訳ない", "が"],
            ),
            (
                "私はあなたにお手数をおかけました",
                &[
                    "私",
                    "は",
                    "あなた",
                    "に",
                    "お手数",
                    "を",
                    "お",
                    "かけました",
                ],
            ),
            ("ここにおかけなさい", &["ここ", "に", "お", "かけなさい"]),
            ("弾き出されてる", &["弾き出されてる"]),
            ("あかんわ", &["あかん", "わ"]),
            ("ぶっちゃけ", &["ぶっちゃけ"]),
            ("賢人たち", &["賢人", "たち"]),
            ("差ついた", &["差", "ついた"]),
            ("ですら", &["ですら"]),
            ("でさえ", &["でさえ"]),
            ("みごとにやってのける", &["みごと", "に", "やってのける"]),
            ("いる", &["いる"]),
            ("はいずれ", &["は", "いずれ"]),
            ("お下がり", &["お下がり"]),
            (
                "でも1000台とか1桁はあんまりだよな",
                &[
                    "でも",
                    "1000台",
                    "とか",
                    "1桁",
                    "は",
                    "あんまり",
                    "だ",
                    "よな",
                ],
            ),
            (
                "みんなにうらやましがられている",
                &["みんな", "に", "うらやましがられている"],
            ),
            ("悪がられて", &["悪がられて"]),
            (
                "期待されがちなので男女",
                &["期待されがち", "なので", "男女"],
            ),
            ("とぎれがちに話す", &["とぎれがち", "に", "話す"]),
            (
                "手にとっていただきやすくなる",
                &["手にとって", "いただき", "やすくなる"],
            ),
            ("さほど", &["さほど"]),
            ("大きさほどもある", &["大きさ", "ほど", "も", "ある"]),
            ("しかいない", &["しか", "いない"]),
            ("掴めていない", &["掴めていない"]),
            ("振り回されたいな", &["振り回されたい", "な"]),
            ("さぼっている", &["さぼっている"]),
            ("のままで来る", &["の", "まま", "で", "来る"]),
            ("5人中4人", &["5人中", "4人"]),
            (
                "彼はどなりすぎて声をからした",
                &["彼", "は", "どなり", "すぎて", "声", "を", "からした"],
            ),
            (
                "そうしたいからしただけだ",
                &["そう", "したい", "から", "した", "だけ", "だ"],
            ),
            ("推し続けている", &["推し", "続けている"]),
            ("少し直せたら", &["少し", "直せたら"]),
            ("良いほう", &["良い", "ほう"]),
            ("いいえ", &["いいえ"]),
            ("割り当てられた", &["割り当てられた"]),
            (
                "綺麗だけど近よりがたいよね",
                &["綺麗", "だけど", "近よりがたい", "よね"],
            ),
            ("そうなんじゃない", &["そう", "な", "ん", "じゃない"]),
            ("なんというかすみません", &["なんというか", "すみません"]),
            ("めんどくそがる", &["めんどくそがる"]),
            ("がなんで終わった", &["が", "なんで", "終わった"]),
            (
                "てか最近ファン層は円盤すら買わないからそいつらから金とるってのは無謀",
                &[
                    "てか",
                    "最近",
                    "ファン層",
                    "は",
                    "円盤",
                    "すら",
                    "買わない",
                    "から",
                    "そいつら",
                    "から",
                    "金",
                    "とる",
                    "ってのは",
                    "無謀",
                ],
            ),
            ("とろいな", &["とろい", "な"]),
            ("なんでもかんでも", &["なんでもかんでも"]),
            ("しないかい", &["しない", "かい"]),
            (
                "参拝しちゃいかんという人がいます",
                &["参拝しちゃ", "いかん", "という", "人", "が", "います"],
            ),
            (
                "人をひやかしちゃいやよ",
                &["人", "を", "ひやかしちゃ", "いや", "よ"],
            ),
            ("しちゃいたい", &["しちゃいたい"]),
            (
                "けがなどをしないように",
                &["けが", "など", "を", "しない", "ように"],
            ),
            ("買い支えたいと思う", &["買い", "支えたい", "と", "思う"]),
            ("おじゃましています", &["おじゃましています"]),
            ("とかいらんから", &["とか", "いらん", "から"]),
            (
                "ということだろうけど",
                &["という", "こと", "だろう", "けど"],
            ),
            (
                "のはわからなくもない",
                &["の", "は", "わからなく", "も", "ない"],
            ),
            ("変わっていくだろう", &["変わっていく", "だろう"]),
            ("待ってねぇ", &["待って", "ねぇ"]),
            (
                "おかしいと思わんですか",
                &["おかしい", "と", "思わん", "です", "か"],
            ),
            ("ズレてる", &["ズレてる"]),
            ("紅茶飲みたい", &["紅茶", "飲みたい"]),
            ("電気がついた", &["電気", "が", "ついた"]),
            ("脚本会議", &["脚本", "会議"]),
            (
                "見せなきゃいけなくなって",
                &["見せなきゃ", "いけなくなって"],
            ),
            (
                "私じゃなくなるような瞬間があって",
                &["私", "じゃなくなる", "ような", "瞬間", "が", "あって"],
            ),
            ("効いててかなりぬくい", &["効いてて", "かなり", "ぬくい"]),
            ("撮影してていつもは", &["撮影してて", "いつも", "は"]),
            (
                "むしろいないほうが珍しい",
                &["むしろ", "いない", "ほう", "が", "珍しい"],
            ),
            ("旅行にいきたい", &["旅行", "に", "いきたい"]),
            (
                "見ててこんな話あったっけ",
                &["見てて", "こんな", "話", "あった", "っけ"],
            ),
            ("いじめとかある", &["いじめ", "とか", "ある"]),
            ("となったらしい", &["となったらしい"]),
            ("基地外が必死過ぎ", &["基地外", "が", "必死", "過ぎ"]),
            ("調整のせいとか", &["調整", "の", "せい", "とか"]),
            ("はっしていない", &["はっしていない"]),
            ("無理さえしなければ", &["無理", "さえ", "しなければ"]),
            ("ところで", &["ところで"]),
            ("外に出て", &["外", "に", "出て"]),
            ("大人しそうな顔", &["大人しそう", "な", "顔"]),
            (
                "おとなしそうなようすにだまされた",
                &["おとなしそう", "な", "ようす", "に", "だまされた"],
            ),
            ("勝手に入る", &["勝手に", "入る"]),
            ("後継ぎする", &["後継ぎ", "する"]),
            ("なすまん", &["な", "すまん"]),
            ("強いんだね", &["強い", "ん", "だ", "ね"]),
            ("おんなじなんだろ", &["おんなじ", "な", "ん", "だろ"]),
            ("女神様", &["女神", "様"]),
            ("邪推した事柄", &["邪推した", "事柄"]),
            ("邪推してしまう", &["邪推してしまう"]),
            ("良さげかも", &["良さげ", "かも"]),
            ("事故ってます", &["事故ってます"]),
            ("卒倒している", &["卒倒している"]),
            ("卒倒させる", &["卒倒させる"]),
            ("出したいときは", &["出したい", "とき", "は"]),
            ("柔らかさ", &["柔らかさ"]),
            ("次がある", &["次", "が", "ある"]),
            ("のせいですね", &["の", "せい", "です", "ね"]),
            (
                "それただの怪しい人ですし",
                &["それ", "ただ", "の", "怪しい", "人", "です", "し"],
            ),
            ("ごときが知る", &["ごとき", "が", "知る"]),
            ("山にはさまれて", &["山", "に", "はさまれて"]),
            (
                "物がぼんやりとかすんで見える",
                &["物", "が", "ぼんやり", "と", "かすんで", "見える"],
            ),
            (
                "どなた様でございましょうか",
                &["どなた", "様", "でございましょう", "か"],
            ),
            (
                "読んでくださりありがとうございました",
                &["読んで", "くださり", "ありがとうございました"],
            ),
            ("ふざけんな", &["ふざけんな"]),
            ("観終わってた", &["観", "終わってた"]),
            ("意味深終わり", &["意味深", "終わり"]),
            ("今日とて居残りです", &["今日", "とて", "居残り", "です"]),
            ("堪能させていただきます", &["堪能させていただきます"]),
            (
                "わからんからそう思った",
                &["わからん", "から", "そう", "思った"],
            ),
            (
                "うちからそうなっても",
                &["うち", "から", "そう", "なっても"],
            ),
            ("上映会やな", &["上映", "会", "や", "な"]),
            ("以上書いてください", &["以上", "書いてください"]),
            (
                "してしまったのがいまだに忘れられないし",
                &["してしまった", "の", "が", "いまだに", "忘れられない", "し"],
            ),
            ("彼ははんぱじゃなく", &["彼", "は", "はんぱじゃなく"]),
            ("許さないじゃなくてさ", &["許さない", "じゃなくて", "さ"]),
            ("じゃなかったです", &["じゃなかったです"]),
            (
                "彼女は苦しげにうめいて横たわった",
                &["彼女", "は", "苦しげ", "に", "うめいて", "横たわった"],
            ),
            (
                "わたしにはちょっとわかりかねますので",
                &["わたし", "には", "ちょっと", "わかりかねます", "ので"],
            ),
            ("要素はないかと", &["要素", "は", "ない", "か", "と"]),
            ("すごいじゃん", &["すごい", "じゃん"]),
            ("腕をつかまれて路地", &["腕", "を", "つかまれて", "路地"]),
            (
                "別にマイナスにならん",
                &["別に", "マイナス", "に", "ならん"],
            ),
            (
                "遊びばかりはだめだよ",
                &["遊び", "ばかり", "は", "だめ", "だ", "よ"],
            ),
            ("最中でも", &["最中", "でも"]),
            ("小動物好き物好き", &["小動物", "好き", "物好き"]),
            ("知れないですか", &["知れないです", "か"]),
            ("かも知れないですね", &["かも知れない", "です", "ね"]),
            ("匙ですくう", &["匙", "で", "すくう"]),
            ("デカかったクドくない", &["デカかった", "クドくない"]),
            (
                "決めたらしい教われたらしい",
                &["決めたらしい", "教われたらしい"],
            ),
            (
                "臆病なくせにとてもよい仲間だった",
                &["臆病", "な", "くせに", "とても", "よい", "仲間", "だった"],
            ),
            ("あのねあのさ", &["あのね", "あのさ"]),
            (
                "これまでになかったような名優",
                &["これまで", "に", "なかった", "ような", "名優"],
            ),
            ("確かめてちゃんと", &["確かめて", "ちゃんと"]),
            (
                "ことにしましょうってなった",
                &["ことにしましょう", "って", "なった"],
            ),
            ("見てござる", &["見て", "ござる"]),
            (
                "彼がいうことはわけがわからない",
                &["彼", "が", "いう", "こと", "は", "わけがわからない"],
            ),
            (
                "わけのわからないことをくどくど言う",
                &["わけのわからない", "こと", "を", "くどくど", "言う"],
            ),
            ("ごくまれに", &["ごくまれ", "に"]),
            (
                "天をうらんでみたところで始まらない",
                &["天", "を", "うらんで", "みた", "ところで", "始まらない"],
            ),
            ("癒やされたかった", &["癒やされたかった"]),
            ("7時には帰ってきなさい", &["7時", "には", "帰ってきなさい"]),
            ("人はいますか", &["人", "は", "います", "か"]),
            ("トマトづくし", &["トマト", "づくし"]),
            ("見えざる関係性", &["見えざる", "関係性"]),
            ("だめだったら", &["だめ", "だったら"]),
            (
                "万事不都合の無いようにはからってくれ",
                &["万事", "不都合", "の", "無い", "ように", "はからってくれ"],
            ),
            ("ではみなさん", &["では", "みなさん"]),
            ("鉄とはがね", &["鉄", "と", "はがね"]),
            ("抹茶とは", &["抹茶", "とは"]),
            ("工夫がされる", &["工夫", "が", "される"]),
            ("うまいことしたね", &["うまいこと", "した", "ね"]),
            (
                "ことしは新成人１４人のうち８人が避難先などから村の村民会館に集まりました",
                &[
                    "ことし",
                    "は",
                    "新成人",
                    "１４人",
                    "の",
                    "うち",
                    "８人",
                    "が",
                    "避難先",
                    "など",
                    "から",
                    "村",
                    "の",
                    "村民",
                    "会館",
                    "に",
                    "集まりました",
                ],
            ),
            ("鬱が悪化する", &["鬱", "が", "悪化する"]),
            (
                "一部が手に入ればことし１年の願いがかなうとされています",
                &[
                    "一部",
                    "が",
                    "手に入れば",
                    "ことし",
                    "１年",
                    "の",
                    "願い",
                    "が",
                    "かなう",
                    "とされています",
                ],
            ),
            ("汗を流しました", &["汗を流しました"]),
            ("気がついてる", &["気がついてる"]),
            ("ガスがついている", &["ガス", "が", "ついている"]),
            ("再開通", &["再", "開通"]),
            ("謝罪はあったにせよ", &["謝罪", "は", "あった", "にせよ"]),
            ("うそではないにしろ", &["うそ", "ではない", "にしろ"]),
            ("普段着てる服", &["普段", "着てる", "服"]),
            ("エレガントなお洋服", &["エレガント", "な", "お", "洋服"]),
            (
                "老いてなお元気なこと",
                &["老いて", "なお", "元気", "な", "こと"],
            ),
            ("何も口にせぬ", &["何も", "口", "に", "せぬ"]),
            ("切ねぇ", &["切ねぇ"]),
            ("何故人気がある", &["何故", "人気がある"]),
            ("バラしちゃってる", &["バラしちゃってる"]),
            ("気を使わせている", &["気を使わせている"]),
            ("一段上がる", &["一段", "上がる"]),
            ("一段落ちる", &["一段", "落ちる"]),
            ("恐怖ですくむ", &["恐怖", "で", "すくむ"]),
            (
                "全員がたちすくみました",
                &["全員", "が", "たちすくみました"],
            ),
            ("雪がないため", &["雪", "が", "ない", "ため"]),
            ("雪がなく", &["雪", "が", "なく"]),
            ("零れ落ちてる", &["零れ落ちてる"]),
            ("使い物にならんだろ", &["使い物", "に", "ならん", "だろ"]),
            ("私とならんで走った", &["私", "と", "ならんで", "走った"]),
            ("のうえに", &["の", "うえ", "に"]),
            ("皇位についたが", &["皇位", "に", "ついた", "が"]),
            ("疱瘡がついたか", &["疱瘡", "が", "ついた", "か"]),
            ("折りたたみ式ついたて", &["折りたたみ式", "ついたて"]),
            (
                "いろいろな部分をもんだりこすったりすること",
                &[
                    "いろいろ",
                    "な",
                    "部分",
                    "を",
                    "もんだり",
                    "こすったり",
                    "する",
                    "こと",
                ],
            ),
            (
                "たまにはいいもんだよ",
                &["たまに", "は", "いい", "もんだ", "よ"],
            ),
            (
                "歩みをはやめるのだった",
                &["歩み", "を", "はやめる", "の", "だった"],
            ),
            (
                "たばこはやめると誓います",
                &["たばこ", "は", "やめる", "と", "誓います"],
            ),
            (
                "私個人の生活についてとやかくうるさくいうのはやめてください",
                &[
                    "私",
                    "個人",
                    "の",
                    "生活",
                    "について",
                    "とやかく",
                    "うるさく",
                    "いう",
                    "の",
                    "は",
                    "やめてください",
                ],
            ),
            ("こもりがちな人", &["こもりがち", "な", "人"]),
            ("がちなやつ", &["がち", "な", "やつ"]),
            (
                "長くはかからないでしょう",
                &["長く", "は", "かからないでしょう"],
            ),
            (
                "人はいないでしょうね",
                &["人", "は", "いないでしょう", "ね"],
            ),
            ("人はいないですね", &["人", "は", "いないです", "ね"]),
            ("猛者どもの集い", &["猛者", "ども", "の", "集い"]),
            ("うまいかまずいか", &["うまい", "か", "まずい", "か"]),
            ("守衛にとがめられた", &["守衛", "に", "とがめられた"]),
            ("問い合わせがたくさん", &["問い合わせ", "が", "たくさん"]),
            ("楽しみがたくさん", &["楽しみ", "が", "たくさん"]),
            ("ふくろうは", &["ふくろう", "は"]),
            ("語れるもんだな", &["語れる", "もんだ", "な"]),
            ("筋をもんでくれ", &["筋", "を", "もんでくれ"]),
            (
                "いわきからさいたままで",
                &["いわき", "から", "さいたま", "まで"],
            ),
            ("新型コロナウイルス", &["新型コロナウイルス"]),
            ("新型コロナウィルス", &["新型コロナウィルス"]),
            (
                "映画を見るとか食事をするとか",
                &["映画", "を", "見る", "とか", "食事", "を", "する", "とか"],
            ),
            (
                "さもうれしそうに笑う",
                &["さも", "うれしそう", "に", "笑う"],
            ),
            ("出しなに客が来る", &["出しな", "に", "客", "が", "来る"]),
            ("出しながら飛んで", &["出し", "ながら", "飛んで"]),
            ("正直言いたい", &["正直", "言いたい"]),
            (
                "おとめにふさわしい振る舞い",
                &["おとめ", "に", "ふさわしい", "振る舞い"],
            ),
            ("気がないのよ", &["気がない", "の", "よ"]),
            (
                "口論のあげくに殴り合いになった",
                &["口論", "の", "あげく", "に", "殴り合い", "に", "なった"],
            ),
            ("お手数おかけします", &["お手数", "おかけします"]),
            (
                "30分後におかけ直しください",
                &["30分", "後", "に", "お", "かけ直し", "ください"],
            ),
            ("わかりきった", &["わかりきった"]),
            (
                "最良の方法は何だと思いますか",
                &[
                    "最良",
                    "の",
                    "方法",
                    "は",
                    "何",
                    "だ",
                    "と",
                    "思います",
                    "か",
                ],
            ),
            (
                "どうせいやがらせでする",
                &["どうせ", "いやがらせ", "で", "する"],
            ),
            (
                "芝居もどきのせりふを言う",
                &["芝居", "もどき", "の", "せりふ", "を", "言う"],
            ),
            ("がんもどきという食品", &["がんもどき", "という", "食品"]),
            ("落ちこぼれている", &["落ちこぼれている"]),
            ("1話しか見てない", &["1話", "しか", "見てない"]),
            (
                "忙しくてろくに更新もできず",
                &["忙しくて", "ろくに", "更新", "も", "できず"],
            ),
            ("だまってろって", &["だまってろ", "って"]),
            ("しっぽく蕎麦", &["しっぽく", "蕎麦"]),
            (
                "猫はしっぽをぴんとはね上がって歩いた",
                &[
                    "猫",
                    "は",
                    "しっぽ",
                    "を",
                    "ぴんと",
                    "はね上がって",
                    "歩いた",
                ],
            ),
            (
                "物がぴんとはね上がるさま",
                &["物", "が", "ぴんと", "はね上がる", "さま"],
            ),
            ("やる気はない", &["やる気", "は", "ない"]),
            (
                "あけましておめでとうございます",
                &["あけましておめでとうございます"],
            ),
            (
                "おれたちは行くのにおまえたちは行かぬ",
                &[
                    "おれたち",
                    "は",
                    "行く",
                    "のに",
                    "おまえたち",
                    "は",
                    "行かぬ",
                ],
            ),
            ("よろしくおねがいします", &["よろしくおねがいします"]),
            (
                "気を遣ってくれてるのかと思ってました",
                &["気を遣ってくれてる", "のか", "と", "思ってました"],
            ),
            (
                "太陽をかたどったしるし",
                &["太陽", "を", "かたどった", "しるし"],
            ),
            (
                "間違えていらっしゃるのかしら",
                &["間違えて", "いらっしゃる", "の", "かしら"],
            ),
            (
                "ヤツはいそうにないな",
                &["ヤツ", "は", "いそうにない", "な"],
            ),
            ("確認をとっています", &["確認", "を", "とっています"]),
            (
                "人口10万人以上の都市の中で唯一旅客を扱う鉄道駅が存在せず",
                &[
                    "人口",
                    "10万人",
                    "以上",
                    "の",
                    "都市",
                    "の",
                    "中",
                    "で",
                    "唯一",
                    "旅客",
                    "を",
                    "扱う",
                    "鉄道駅",
                    "が",
                    "存在",
                    "せず",
                ],
            ),
            ("だし", &["だ", "し"]),
            ("だしはおいしい", &["だし", "は", "おいしい"]),
            ("だして", &["だして"]),
            ("だしといて", &["だしといて"]),
            ("割り切れたら", &["割り切れたら"]),
            ("あり得なかったり", &["あり得なかったり"]),
            ("代わり映え", &["代わり映え"]),
            (
                "器用なのですぐ上達しますよ",
                &["器用", "なので", "すぐ", "上達します", "よ"],
            ),
            ("おにいちゃん", &["おにいちゃん"]),
            ("動画につまってる", &["動画", "に", "つまってる"]),
            ("出来そう", &["出来そう"]),
            (
                "その上着貸してください",
                &["その", "上着", "貸してください"],
            ),
            ("幸多き", &["幸", "多き"]),
            (
                "きっと気に入っていつかまた来てくれるよ",
                &["きっと", "気に入って", "いつか", "また", "来てくれる", "よ"],
            ),
            (
                "私がいそうな場所知ってたんだから",
                &[
                    "私",
                    "が",
                    "いそう",
                    "な",
                    "場所",
                    "知ってた",
                    "ん",
                    "だから",
                ],
            ),
            ("うまくハメられた", &["うまく", "ハメられた"]),
            ("してるとこだから", &["してる", "とこ", "だから"]),
            ("下記のとおりです", &["下記", "の", "とおり", "です"]),
            ("123ヶ年", &["123ヶ年"]),
            ("そうはいかん", &["そう", "は", "いかん"]),
            (
                "いつなりともお使いなさい",
                &["いつなりと", "も", "お", "使いなさい"],
            ),
            ("よそで待ってて", &["よそ", "で", "待ってて"]),
            ("3つおきの席", &["3つ", "おき", "の", "席"]),
            ("1年おきに", &["1年", "おきに"]),
            ("練習したかいがあって", &["練習した", "かいがあって"]),
            (
                "高いお金を払ったかいがあったと思う",
                &["高い", "お金", "を", "払った", "かいがあった", "と", "思う"],
            ),
            ("養生したかいもなく", &["養生した", "かいもなく"]),
            ("読みがいがある", &["読みがい", "が", "ある"]),
            ("狩りがいのある", &["狩りがい", "の", "ある"]),
            ("懐いている", &["懐いている"]),
            ("カッコよさ", &["カッコよさ"]),
            (
                "上手く案内出来てたらいいんですけど",
                &["上手く", "案内", "出来てたら", "いい", "ん", "です", "けど"],
            ),
            (
                "仲間になりたそうに見ている",
                &["仲間", "に", "なりたそう", "に", "見ている"],
            ),
            (
                "何か問いたそうな口調",
                &["何か", "問いたそう", "な", "口調"],
            ),
            (
                "どんなものにも潮時がある",
                &["どんな", "もの", "にも", "潮時", "が", "ある"],
            ),
            (
                "特化してるというからね",
                &["特化してる", "という", "から", "ね"],
            ),
            ("歩いたぁ", &["歩いた", GAP]),
            ("りばてぃ", &[GAP]),
            ("サウンドトラック", &["サウンドトラック"]),
            ("写真を撮りました", &["写真を撮りました"]),
            ("取り留めの無い", &["取り留めの無い"]),
            ("取り留めも無い", &["取り留めも無い"]),
            ("これへんだ", &["これ", "へん", "だ"]),
            ("おそれたか", &["おそれた", "か"]),
            ("不確かなものに", &["不確か", "な", "もの", "に"]),
            ("まとめていかねばな", &["まとめていかねば", "な"]),
            ("来るからすき", &["来る", "から", "すき"]),
            ("けんかを引分ける", &["けんか", "を", "引分ける"]),
            ("取り計らいましょう", &["取り計らいましょう"]),
            ("一日置いただけで", &["一日", "置いた", "だけ", "で"]),
        ];
        let mut failures: Vec<String> = Vec::new();
        for (input, expected) in cases {
            let result = simple_segment(&ctx, input, None).await.unwrap();
            let actual = segmentation(&result);
            if actual != *expected {
                failures.push(format!(
                    "{:?}: rust={:?} expected={:?}",
                    input, actual, expected
                ));
            }
        }
        assert!(
            failures.is_empty(),
            "{} of {} segmentation cases diverged:\n{}",
            failures.len(),
            cases.len(),
            failures.join("\n")
        );
    }
}
