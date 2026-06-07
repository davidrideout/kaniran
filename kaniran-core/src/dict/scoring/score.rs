use crate::characters::char_class::get_char_class;
use crate::characters::constants::{ITERATION_CHARACTERS, KANA_CHARACTERS, MODIFIER_CHARACTERS};
use crate::characters::kana::{long_vowel_modifier_p, mora_length};
use crate::characters::kani_kana_class::KanaClass;
use crate::conn::kani_context::KaniranContext;
use crate::dict::conj::ConjData;
use crate::dict::errata::NO_KANJI_BREAK_PENALTY;
use crate::dict::grammar::suffix::resolve::get_suffixes;
use crate::dict::kani_word::KaniWordDispatchEnum;
use crate::dict::path::TopArray;
use crate::dict::scoring::calc_score::calc_score;
use crate::dict::text_classes::ScoreMod;
use sqlx::PgPool;
use std::collections::HashSet;

/// Port of `ichiran/dict:*is-arch-cache*` (`dict.lisp:745`).
///
/// Set of seqs whose every sense is tagged `arch`/`obsc`/`rare`, plus
/// every conjugation root whose `from` column points at such a seq.
pub fn is_arch_cache(ctx: &KaniranContext) -> &HashSet<i32> {
    &ctx.is_arch
}

pub async fn build_is_arch(pool: &PgPool) -> Result<HashSet<i32>, sqlx::Error> {
    let a1: Vec<i32> = sqlx::query_scalar(
        "SELECT sense.seq FROM sense \
         LEFT JOIN sense_prop sp \
                ON sp.sense_id = sense.id \
               AND sp.tag = 'misc' \
               AND sp.text IN ('arch', 'obsc', 'rare') \
         GROUP BY sense.seq \
         HAVING bool_and(sp.id IS NOT NULL)",
    )
    .fetch_all(pool)
    .await?;
    let a2: Vec<i32> =
        sqlx::query_scalar("SELECT DISTINCT seq FROM conjugation WHERE \"from\" = ANY($1)")
            .bind(&a1)
            .fetch_all(pool)
            .await?;
    let mut set: HashSet<i32> = a1.into_iter().collect();
    set.extend(a2);
    Ok(set)
}

/// Port of `ichiran/dict:segment` (`dict.lisp:674`).
///
/// In-memory record for one candidate word match at a fixed
/// `(start, end)` slice, decorated with score and info plist before
/// the find-best-path scoring loop runs.
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
    /// `text(segment.word)` (via [`crate::dict::counters::methods::text`]),
    /// stores it, and returns a borrow. Mirrors upstream's `setf`
    /// — repeated calls are O(1) after the first.
    pub fn get_text(&mut self) -> &str {
        if self.text.is_none() {
            let t = crate::dict::counters::methods::text(&self.word).into_owned();
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

/// Port of `ichiran/dict:length-multiplier` (`dict.lisp:681`).
///
/// Returns `length^power` while `length <= len-lim`, otherwise goes
/// linear with `length * len-lim^(power-1)`.
pub fn length_multiplier(length: i64, power: i64, len_lim: i64) -> i64 {
    if length <= len_lim {
        length.pow(power as u32)
    } else {
        length * len_lim.pow((power - 1) as u32)
    }
}

/// Port of `ichiran/dict:*length-coeff-sequences*` (`dict.lisp:686`).
///
/// Per-class coefficient sequences (`:strong`/`:weak`/`:tail`/`:ltail`)
/// that `length-multiplier-coeff` looks up to score a segment by length.
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

/// Port of `ichiran/dict:length-multiplier-coeff` (`dict.lisp:694`).
///
/// Lookup helper for the `calc-score` length-bonus formula
/// (`dict.lisp:928, 933`). The first argument is a mora count and the
/// second selects one of four pre-tabulated coefficient sequences.
/// Inside the tabled range it returns the coefficient at that index;
/// outside the range it linearly extrapolates from the last tabled
/// value.
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

/// Port of `ichiran/dict:kanji-break-penalty` (`dict.lisp:702`).
///
/// Adjusts a candidate's `score` when the segmenter's hard kanji-break
/// marker falls on the candidate's boundary. The `kanji-break` argument
/// lists the character positions of the break(s) within the matched
/// slice; the function decides whether the break sits at the beginning
/// (`:beg`), end (`:end`), or both (`:both`) of the slice, applies a
/// small per-`posi` bonus when an n-suf / pref overlap exists, then
/// halves the score (`ceiling score 2`) unless the candidate is
/// exempted (`*no-kanji-break-penalty*`, `すー`-starting beg, or the
/// `vs-s` / `v5s` suru-suffix path).
///
/// Mutually recursive with [`crate::dict::scoring::calc_score::calc_score`] via the
/// suru-suffix branch.
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

/// Port of `ichiran/dict:is-arch` (`dict.lisp:759`).
///
/// True when `seq` is recorded as archaic. Lisp's
/// `(nth-value 1 (gethash seq cache))` reads only key presence.
pub fn is_arch(ctx: &KaniranContext, seq: i32) -> bool {
    ctx.is_arch.contains(&seq)
}

/// Port of `ichiran/dict:get-non-arch-posi` (`dict.lisp:762`).
///
/// Returns the distinct list of `pos`-tagged property values for senses
/// inside `seq_set` whose containing sense does NOT carry an `arch` /
/// `obsc` / `rare` misc tag (an anti-join via `sp2.id IS NULL`).
pub async fn get_non_arch_posi(
    ctx: &KaniranContext,
    seq_set: &[i32],
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT DISTINCT sp1.text \
         FROM sense_prop sp1 \
         LEFT JOIN sense_prop sp2 \
                ON sp1.sense_id = sp2.sense_id \
               AND sp2.tag = 'misc' \
               AND sp2.text IN ('arch', 'obsc', 'rare') \
         WHERE sp1.seq = ANY($1) \
           AND sp1.tag = 'pos' \
           AND sp2.id IS NULL",
    )
    .bind(seq_set)
    .fetch_all(&ctx.pool)
    .await
}

/// Port of `ichiran/dict:gen-score` (`dict.lisp:985`).
///
/// Mutates `segment.score` and `segment.info` in place with the
/// `(score, info)` pair returned by [`calc_score`], then returns the
/// same segment so call sites can chain.
pub async fn gen_score<'a>(
    ctx: &KaniranContext,
    segment: &'a mut Segment,
    final_: bool,
    kanji_break: &[usize],
) -> Result<&'a mut Segment, sqlx::Error> {
    // dict.lisp:986-987 — (setf (values (segment-score segment) (segment-info segment))
    //                       (calc-score (segment-word segment) :final final :kanji-break kanji-break))
    let (score, info) = calc_score(
        ctx,
        &segment.word,
        final_,
        /* use_length */ None,
        /* score_mod */ None,
        kanji_break,
    )
    .await?;
    segment.score = Some(score);
    segment.info = info;
    // dict.lisp:988 — segment (the function returns the same segment).
    Ok(segment)
}

/// Port of `ichiran/dict:find-sticky-positions` (`dict.lisp:990`).
///
/// Positions where a word can neither start nor end: after a sokuon
/// when the following character is a kana mora, and at any modifier
/// or iteration character unless it sits at the end and would extend
/// the preceding mora's vowel (long vowel mark, or `+a/+i/+u/+e/+o`
/// agreeing with the prior kana's vowel).
///
/// Returned offsets are **character** positions per CONVENTIONS §4.5.
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

/// Port of `ichiran/dict:make-slice` (`dict.lisp:1009`).
///
/// Returns the empty seed string-view that callers thread through
/// `subseq_slice` (upstream a zero-length displaced character vector).
pub fn make_slice() -> &'static str {
    ""
}

/// Port of `ichiran/dict:subseq-slice` (`dict.lisp:1013`).
///
/// Returns the substring `s[start..end]` using *character* offsets;
/// `end = None` slices to the end of `s`.
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

/// Port of `ichiran/dict:*identical-word-score-cutoff*` (`dict.lisp:1020`).
///
/// Cutoff ratio `1/2` that `cull-segments` multiplies against the
/// max score to drop low-scoring identical-word segments.
pub const IDENTICAL_WORD_SCORE_CUTOFF: (i64, i64) = (1, 2);

/// Port of `ichiran/dict:compare-common` (`dict.lisp:1022`).
///
/// Ranking predicate over two JMdict `common` values that orders
/// readings by commonness (lower rank = more common). Inputs: `None`
/// mirrors Lisp `nil` (no rank), `Some(0)` is the "common but
/// unranked" marker, positive values are rank tiers, and negative or
/// zero c1 values fall off the `cond` ladder and return `Nil`.
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

/// Port of `ichiran/dict:cull-segments` (`dict.lisp:1027`).
///
/// Sorts segments by [`compare_common`] over each segment's
/// `info.common` key, then by descending [`Segment::score`], then keeps
/// the leading run whose score is at least `max-score * 1/2`. Empty
/// input returns empty.
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

/// Port of `ichiran/dict:*score-cutoff*` (`dict.lisp:1069`).
///
/// Minimum segment score (5) used to filter out bad kana spellings
/// without dropping any kanji spellings.
pub const SCORE_CUTOFF: i32 = 5;

/// Port of `ichiran/dict:*segment-score-cutoff*` (`dict.lisp:1351`).
///
/// Threshold ratio `2/3` that `word-info-from-segment-list` multiplies
/// against the max score to drop low-scoring segments.
pub const SEGMENT_SCORE_CUTOFF: (i64, i64) = (2, 3);

#[cfg(test)]
mod tests;
