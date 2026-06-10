use crate::characters::char_class::{consecutive_char_groups, CharClass};
use crate::characters::kanji::sequential_kanji_positions;
use crate::conn::kani_context::KaniranContext;
use crate::dict::accessors::{get_segment_score, KaniSegmentScoreArg};
use crate::dict::counters::dispatchers::find_counter;
use crate::dict::errata::{FORCE_KANJI_BREAK, NO_KANJI_BREAK};
use crate::dict::grammar::penalty::get_synergies;
use crate::dict::grammar::segfilter::{apply_segfilters, get_penalties};
use crate::dict::grammar::suffix::resolve::{find_word_suffix, get_suffix_map};
use crate::dict::grammar::synergy::Synergy;
use crate::dict::kani_lite_segment_list::KaniLiteSegmentList;
use crate::dict::kani_lite_top_array::{
    kani_lite_get_array, kani_lite_register_item, KaniLiteTopArray,
};
use crate::dict::kani_lite_top_array_item::{
    KaniLitePath, KaniLitePathElement, KaniLiteTopArrayItem,
};
use crate::dict::kani_word::KaniWordDispatchEnum;
use crate::dict::readings::{find_substring_words, find_word, FindWordRows};
use crate::dict::scoring::score::{
    cull_segments, find_sticky_positions, gen_score, subseq_slice, Segment, SCORE_CUTOFF,
};
use crate::dict::split::segsplit::get_segsplit;
use crate::dict::text_classes::find_word_as_hiragana;
use crate::dict::word_info::SuffixMapTemp;
use std::collections::HashMap;
use std::sync::Arc;

/// Port of `ichiran/dict:*max-word-length*` (`dict.lisp:486`).
///
/// Upper bound (50 chars) on the length of a word looked up in the
/// JMdict text tables; anything longer is treated as nonexistent
/// without hitting the database.
pub const MAX_WORD_LENGTH: usize = 50;

/// Port of `ichiran/dict:*substring-hash*` (`dict.lisp:487`).
///
/// Per-call-tree cache of pre-fetched `kana_text` / `kanji_text` rows
/// keyed by their `text` column, letting nested `find-word` calls skip
/// the database round-trip for keys already present.
/// Map from a substring of an input string to the `kana_text` /
/// `kanji_text` rows pre-fetched for it by `find-substring-words`.
/// Per-key uniformity (all rows from one table) is enforced by the
/// populator's kana-vs-kanji key split (`dict.lisp:511`).
pub type SubstringHash = HashMap<String, FindWordRows>;

/// Port of `ichiran/dict:segment-list` (`dict.lisp:1038`).
///
/// Group of [`Segment`]s sharing the same `(start, end)` slice — one
/// per substring that produced at least one above-cutoff segment.
#[derive(Debug, Clone)]
pub struct SegmentList {
    /// Divergence from Lisp: `Arc`-shared. Lisp passes segment objects
    /// by pointer, so the same segment is shared between the input
    /// segment-lists and every surviving `find-best-path` payload;
    /// `Arc` restores that sharing (a deep `Vec<Segment>` clone per
    /// surviving path was 27% of allocation traffic). Segments are
    /// fully populated by `gen-score` before they enter a list; no
    /// consumer mutates them afterwards.
    pub segments: Vec<Arc<Segment>>,
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

/// Port of `ichiran/dict:find-word-full` (`dict.lisp:1052`).
///
/// Composes [`find_word`], [`find_word_suffix`],
/// [`find_word_as_hiragana`], and [`find_counter`] into a single
/// ordered result list (simple words, suffix expansions, hiragana
/// proxies, counter candidates). `CounterArg::At(n)` treats `n` as a
/// character position splitting `word` into number and counter unit.
/// `CounterArg::Auto` finds the first run of `:number` characters
/// via [`consecutive_char_groups`] and uses the first group's
/// `(start, end)` to slice the number; the counter is whatever
/// follows up to the end of `word`.
///
/// [`find_word`]: crate::dict::readings::find_word
/// [`find_word_suffix`]: crate::dict::grammar::suffix::resolve::find_word_suffix
/// [`find_word_as_hiragana`]: crate::dict::text_classes::find_word_as_hiragana
/// [`find_counter`]: crate::dict::counters::dispatchers::find_counter
/// [`consecutive_char_groups`]: crate::characters::char_class::consecutive_char_groups
/// Closed shape of the upstream `:counter` keyword. Per CONVENTIONS
/// §4.3: the Lisp value is `nil` (absent), the keyword `:auto`, or a
/// character-index integer. `Option<CounterArg>` carries the
/// nil-vs-present distinction; the enum carries the auto-vs-integer
/// distinction.
#[derive(Debug, Clone, Copy)]
pub enum CounterArg {
    Auto,
    At(usize),
}

pub async fn find_word_full(
    ctx: &KaniranContext,
    word: &str,
    as_hiragana: bool,
    counter: Option<CounterArg>,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    // dict.lisp:1053 (find-word word)
    let simple_words_rows = find_word(ctx, word, false).await?;

    // Pre-collect simple words as KaniWordDispatchEnum values for the
    // suffix / hiragana branches that need `:matches` / `:exclude`
    // references against them.
    let simple_words: Vec<KaniWordDispatchEnum> = match &simple_words_rows {
        FindWordRows::Kana(rows) => rows
            .iter()
            .cloned()
            .map(KaniWordDispatchEnum::Kana)
            .collect(),
        FindWordRows::Kanji(rows) => rows
            .iter()
            .cloned()
            .map(KaniWordDispatchEnum::Kanji)
            .collect(),
    };

    let mut out: Vec<KaniWordDispatchEnum> = simple_words.clone();

    // dict.lisp:1055 (find-word-suffix word :matches simple-words)
    // find_word_suffix returns Vec<KaniWordDispatchEnum> directly —
    // it carries both Compound (def-simple-suffix output) and Proxy
    // (def-abbr-suffix output) variants per the etypecase at
    // dict-grammar.lisp:565-577.
    let suffix_words = find_word_suffix(ctx, word, &simple_words).await?;
    out.extend(suffix_words);

    // dict.lisp:1056-1057 (when as-hiragana (find-word-as-hiragana …))
    if as_hiragana {
        // (mapcar 'seq simple-words) — simple-words are kanji-text /
        // kana-text rows; (seq r) is the i32 slot. Mirror with a
        // direct field read keyed by variant.
        let exclude: Vec<i32> = simple_words
            .iter()
            .filter_map(|w| match w {
                KaniWordDispatchEnum::Kanji(k) => Some(k.seq),
                KaniWordDispatchEnum::Kana(k) => Some(k.seq),
                _ => None,
            })
            .collect();
        let proxies = find_word_as_hiragana(ctx, word, &exclude, None).await?;
        out.extend(proxies.into_iter().map(KaniWordDispatchEnum::Proxy));
    }

    // dict.lisp:1058-1067 (when counter …)
    if let Some(counter_arg) = counter {
        match counter_arg {
            CounterArg::Auto => {
                // dict.lisp:1060-1064 (:auto branch)
                let word_len = word.chars().count();
                let groups = consecutive_char_groups(CharClass::Number, word, 0, word_len);
                if let Some(&(g_start, g_end)) = groups.first() {
                    // (subseq word (caar groups) (cdar groups))
                    let number = subseq_slice(None, word, g_start, Some(g_end));
                    // (subseq word (cdar groups) (length word))
                    let counter_text = subseq_slice(None, word, g_end, Some(word_len));
                    let counters = find_counter(ctx, number, counter_text, None);
                    out.extend(counters.into_iter().map(KaniWordDispatchEnum::Counter));
                }
            }
            CounterArg::At(idx) => {
                // dict.lisp:1065-1067 (t branch)
                let word_len = word.chars().count();
                let number = subseq_slice(None, word, 0, Some(idx));
                let counter_text = subseq_slice(None, word, idx, Some(word_len));
                // dict.lisp:1067 (:unique (not simple-words))
                let unique = simple_words.is_empty();
                let counters = find_counter(ctx, number, counter_text, Some(unique));
                out.extend(counters.into_iter().map(KaniWordDispatchEnum::Counter));
            }
        }
    }

    Ok(out)
}

/// Port of `ichiran/dict:join-substring-words*` (`dict.lisp:1071`).
///
/// Enumerates every length-bounded `(start, end)` substring that is not
/// blocked by a sticky position, collects the segments
/// [`find_word_full`] yields for each, and accumulates the kanji-break
/// positions reachable from prior segment ends. Offsets are character
/// positions.
pub async fn join_substring_words_star_(
    ctx: &KaniranContext,
    str: &str,
) -> Result<(Vec<(usize, usize, Vec<Segment>)>, Vec<usize>), sqlx::Error> {
    let chars: Vec<char> = str.chars().collect();
    let length = chars.len();

    let sticky = find_sticky_positions(str);
    let substring_hash = Arc::new(find_substring_words(ctx, str, &sticky).await?);
    let katakana_groups = consecutive_char_groups(CharClass::Katakana, str, 0, length);
    let number_groups = consecutive_char_groups(CharClass::Number, str, 0, length);
    // (get-suffix-map str) returns triples borrowing str / ctx.suffix_cache;
    // *suffix-map-temp* owns its data, so materialize owned triples once.
    let suffix_map: Arc<SuffixMapTemp> = Arc::new(
        get_suffix_map(ctx, str)
            .into_iter()
            .map(|(end, items)| {
                let owned: Vec<(String, String, Option<_>)> = items
                    .into_iter()
                    .map(|(substr, key, kf)| (substr.to_string(), key.to_string(), kf.cloned()))
                    .collect();
                (end, owned)
            })
            .collect(),
    );

    let mut kanji_break: Vec<usize> = Vec::new();
    let mut ends: Vec<usize> = Vec::new();
    let mut result: Vec<(usize, usize, Vec<Segment>)> = Vec::new();

    for start in 0..length {
        // (cdr (assoc start katakana-groups)) / (cdr (assoc start number-groups))
        let katakana_group_end = katakana_groups
            .iter()
            .find(|(group_start, _)| *group_start == start)
            .map(|(_, group_end)| *group_end);
        let number_group_end = number_groups
            .iter()
            .find(|(group_start, _)| *group_start == start)
            .map(|(_, group_end)| *group_end);
        // unless (member start sticky)
        if sticky.contains(&start) {
            continue;
        }
        // for end from (1+ start) upto (min (length str) (+ start *max-word-length*))
        let end_max = length.min(start + MAX_WORD_LENGTH);
        for end in (start + 1)..=end_max {
            // unless (member end sticky)
            if sticky.contains(&end) {
                continue;
            }
            // (subseq str start end)
            let part: String = chars[start..end].iter().collect();
            // :as-hiragana (and katakana-group-end (= end katakana-group-end))
            let as_hiragana = katakana_group_end == Some(end);
            // :counter (and number-group-end (<= number-group-end end)
            //               (let ((d (- number-group-end start))) (and (<= d 20) d)))
            let counter = match number_group_end {
                Some(number_group_end) if number_group_end <= end => {
                    let d = number_group_end - start;
                    if d <= 20 {
                        Some(CounterArg::At(d))
                    } else {
                        None
                    }
                }
                _ => None,
            };
            // dict.lisp:1090-1092 — (let ((*suffix-map-temp* suffix-map)
            //   (*suffix-next-end* end) (*substring-hash* substring-hash)) (find-word-full ...))
            let ctx2 = ctx
                .with_suffix_map_temp(Some(Arc::clone(&suffix_map)))
                .with_suffix_next_end(Some(end as i32))
                .with_substring_hash(Arc::clone(&substring_hash));
            let words = find_word_full(&ctx2, &part, as_hiragana, counter).await?;
            // (mapcar (lambda (word) (make-segment :start start :end end :word word)) ...)
            let segments: Vec<Segment> = words
                .into_iter()
                .map(|word| Segment {
                    start,
                    end,
                    word,
                    score: None,
                    info: None,
                    top: None,
                    text: None,
                })
                .collect();
            // (when segments ...)
            if !segments.is_empty() {
                // (when (or (= start 0) (find start ends)) (setf kanji-break (nconc (cond ...) kanji-break)))
                if start == 0 || ends.contains(&start) {
                    let new_positions: Vec<usize> = if FORCE_KANJI_BREAK.contains(&part.as_str()) {
                        // (alexandria:iota (1- (length part)) :start (1+ start))
                        ((start + 1)..end).collect()
                    } else if NO_KANJI_BREAK.contains(&part.as_str()) {
                        Vec::new()
                    } else {
                        sequential_kanji_positions(&part, start)
                    };
                    // (nconc new-positions kanji-break)
                    let mut combined = new_positions;
                    combined.append(&mut kanji_break);
                    kanji_break = combined;
                }
                // (pushnew end ends)
                if !ends.contains(&end) {
                    ends.insert(0, end);
                }
                // (list (list start end segments))
                result.push((start, end, segments));
            }
        }
    }

    // (values result (remove-duplicates kanji-break))
    Ok((result, remove_duplicates(&kanji_break)))
}

/// `(remove-duplicates seq)` with the default `:from-end nil`: an
/// element recurring later in the list is dropped at its earlier
/// position, so the last occurrence survives; the surviving relative
/// order is preserved.
fn remove_duplicates(items: &[usize]) -> Vec<usize> {
    let mut out: Vec<usize> = Vec::new();
    for (index, position) in items.iter().enumerate() {
        if !items[index + 1..].contains(position) {
            out.push(*position);
        }
    }
    out
}

/// Port of `ichiran/dict:join-substring-words` (`dict.lisp:1113`).
///
/// Scores every segment [`join_substring_words_star_`] produced for each
/// slice, drops those below [`SCORE_CUTOFF`], and wraps each surviving,
/// [`cull_segments`]-filtered group in a [`SegmentList`] tagged with its
/// pre-filter `matches` count.
pub async fn join_substring_words(
    ctx: &KaniranContext,
    str: &str,
) -> Result<Vec<SegmentList>, sqlx::Error> {
    // (multiple-value-bind (result kanji-break) (join-substring-words* str) ...)
    let (result, kanji_break) = join_substring_words_star_(ctx, str).await?;
    let length = str.chars().count();
    // dict.lisp:1116 — (alexandria:ends-with #\ー str)
    let ends_with_lw = str.chars().last() == Some('ー');

    let mut sls: Vec<SegmentList> = Vec::new();
    // for (start end segments) in result
    for (start, end, segments) in result {
        // dict.lisp:1118 — (mapcar (lambda (n) (- n start))
        //   (intersection (list start end) kanji-break))
        // SBCL conses each match onto the front, so the intersection of
        // (start end) is (end start) when both are present.
        let mut intersection: Vec<usize> = Vec::new();
        if kanji_break.contains(&start) {
            intersection.push(start);
        }
        if kanji_break.contains(&end) {
            intersection.push(end);
        }
        intersection.reverse();
        let kb: Vec<usize> = intersection.iter().map(|n| n - start).collect();

        // :matches (length segments) — the pre-filter segment count.
        let matches = segments.len();
        // for sl = (loop for segment in segments do (gen-score ...)
        //              if (>= (segment-score segment) *score-cutoff*) collect segment)
        let mut sl: Vec<Segment> = Vec::new();
        for mut segment in segments {
            // dict.lisp:1121-1123 — :final (or (= (segment-end segment) (length str))
            //   (and ends-with-lw (= (segment-end segment) (1- (length str)))))
            let final_ = segment.end == length || (ends_with_lw && segment.end == length - 1);
            gen_score(ctx, &mut segment, final_, &kb).await?;
            if segment.score.expect("gen-score populates segment.score") >= SCORE_CUTOFF {
                sl.push(segment);
            }
        }
        // when sl collect (make-segment-list :segments (cull-segments sl) ...)
        if !sl.is_empty() {
            sls.push(SegmentList {
                segments: cull_segments(sl).into_iter().map(Arc::new).collect(),
                start,
                end,
                top: None,
                matches,
            });
        }
    }
    Ok(sls)
}

/// Port of `ichiran/dict:substring-index` (`dict.lisp:1131`).
///
/// Re-indexes `join-substring-words`' output by each segment-list's
/// `(start, end)` slice.
pub async fn substring_index(
    ctx: &KaniranContext,
    str: &str,
) -> Result<HashMap<(usize, usize), SegmentList>, sqlx::Error> {
    let sls = join_substring_words(ctx, str).await?;
    let mut index: HashMap<(usize, usize), SegmentList> = HashMap::new();
    for sl in sls {
        index.insert((sl.start, sl.end), sl);
    }
    Ok(index)
}

/// Port of `ichiran/dict:top-array-item` (`dict.lisp:1138`).
///
/// One entry in a [`crate::dict::path::TopArray`] — a
/// `(score, payload)` pair representing one candidate path through the
/// segment graph, where the payload mixes [`SegmentList`] and
/// [`Synergy`] elements.
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

/// Port of `ichiran/dict:top-array` (`dict.lisp:1140`).
///
/// Bounded-size top-K accumulator used by `find-best-path`: a
/// fixed-length backing array of [`TopArrayItem`]s sorted highest
/// score first, plus a monotone insert counter. `count` may exceed the
/// array length while later inserts keep pushing out lowest scores.
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

/// Port of `ichiran/dict:*gap-penalty*` (`dict.lisp:1165`).
///
/// Per-character score penalty applied to gaps when scoring a path.
pub const GAP_PENALTY: i64 = -500;

/// Port of `ichiran/dict:gap-penalty` (`dict.lisp:1168`).
///
/// Score contribution for the unconsumed-character gap between two
/// segments: `(end - start) * *gap-penalty*`. Negative is a gap cost;
/// positive (`end < start`) scores overlapping endpoints.
pub fn gap_penalty(start: usize, end: usize) -> i64 {
    (end as i64 - start as i64) * GAP_PENALTY
}

/// Port of `ichiran/dict:get-seg-initial` (`dict.lisp:1171-1173`).
///
/// Runs `seg` through [`apply_segfilters`] (with no left segment) and
/// collects the right segment of each resulting split.
pub fn get_seg_initial(seg: &Arc<KaniLiteSegmentList>) -> Vec<Arc<KaniLiteSegmentList>> {
    apply_segfilters(None, seg)
        .into_iter()
        .map(|(_left, right)| right)
        .collect()
}

/// Port of `ichiran/dict:get-seg-splits` (`dict.lisp:1175`).
///
/// Runs the `(seg_left, seg_right)` pair through [`apply_segfilters`]
/// and, for each resulting split, concatenates [`get_penalties`] with
/// [`get_synergies`].
///
/// [`get_penalties`]: crate::dict::grammar::segfilter::get_penalties
/// [`get_synergies`]: crate::dict::grammar::penalty::get_synergies
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

/// Port of `ichiran/dict:expand-segment-list` (`dict.lisp:1180`).
///
/// In-place expansion of a `cull-segments`-shaped [`SegmentList`]:
/// walks `segments` once, asking `get-segsplit` for a compound-text
/// decomposition per row; each non-nil result is appended next to the
/// original and counted against `matches`. The whole list is then
/// stable-sorted high-to-low by `segment-score`.
pub async fn expand_segment_list(
    ctx: &KaniranContext,
    segment_list: &mut SegmentList,
) -> Result<(), sqlx::Error> {
    // dict.lisp:1183-1187 — `(loop for segment in segments for segsplit = (get-segsplit segment) collect segment when segsplit collect segsplit and do (incf matches))`.
    // Move the existing segments out so we can hand each one to
    // get_segsplit by reference, then push owned values into the new
    // working list.
    let pre_segments = std::mem::take(&mut segment_list.segments);
    let mut working: Vec<Arc<Segment>> = Vec::with_capacity(pre_segments.len() * 2);
    for segment in pre_segments {
        let segsplit = get_segsplit(ctx, &segment).await?;
        working.push(segment);
        if let Some(segsplit) = segsplit {
            working.push(Arc::new(segsplit));
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

/// Port of `ichiran/dict:find-best-path` (`dict.lisp:1190`).
///
/// Builds a top-`limit` array of path scorings over the input
/// segment-lists. Each result is a `(path, score)` pair where `path` is
/// a heterogeneous list of [`SegmentList`]s (left-to-right picks) and
/// [`crate::dict::grammar::synergy::Synergy`]s (inter-slice bonuses).
const DEFAULT_LIMIT: usize = 5;

pub async fn find_best_path(
    ctx: &KaniranContext,
    segment_lists: &mut [SegmentList],
    str_length: usize,
    limit: Option<usize>,
) -> Result<Vec<(Vec<PathElement>, i32)>, sqlx::Error> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT);

    // dict.lisp:1195-1196 — expand-segment-list mutates each input
    // SegmentList. Do this on the FULL list before lite conversion so
    // the slot mutation is preserved upstream.
    for sl in segment_lists.iter_mut() {
        expand_segment_list(ctx, sl).await?;
    }

    // Build lite sidecars; the per-list top-arrays live in a parallel
    // Vec to keep mutation simple in the inner loop.
    let lite_lists: Vec<Arc<KaniLiteSegmentList>> = segment_lists
        .iter()
        .map(|sl| Arc::new(KaniLiteSegmentList::from_segment_list(sl)))
        .collect();
    let mut per_list_tops: Vec<KaniLiteTopArray> = (0..lite_lists.len())
        .map(|_| KaniLiteTopArray::new(limit))
        .collect();

    // dict.lisp:1192 (let ((top (make-instance 'top-array :limit limit))))
    let mut top = KaniLiteTopArray::new(limit);

    // dict.lisp:1193 (register-item top (gap-penalty 0 str-length) nil)
    kani_lite_register_item(&mut top, gap_penalty(0, str_length) as i32, KaniLitePath::nil());

    let n = lite_lists.len();
    // dict.lisp:1200 (loop for (seg1 . rest) on segment-lists ...)
    for i in 0..n {
        let seg1 = Arc::clone(&lite_lists[i]);
        let seg1_start = seg1.start;
        let seg1_end = seg1.end;

        // dict.lisp:1202-1203
        let gap_left_outer = gap_penalty(0, seg1_start);
        let gap_right_outer = gap_penalty(seg1_end, str_length);

        // dict.lisp:1204 (let ((initial-segs (get-seg-initial seg1))))
        let initial_segs = get_seg_initial(&seg1);

        // dict.lisp:1205-1209 (loop for seg in initial-segs ...)
        for seg in initial_segs {
            // dict.lisp:1206 (for score1 = (get-segment-score seg))
            let score1 = get_segment_score(&KaniSegmentScoreArg::KaniLiteSegmentList(&seg))
                .expect("get-seg-initial output carries a scored first segment");
            let payload = KaniLitePath::cons(
                KaniLitePathElement::SegmentList(Arc::clone(&seg)),
                &KaniLitePath::nil(),
            );
            // dict.lisp:1208 (register-item (segment-list-top seg1) (+ gap-left score1) (list seg))
            kani_lite_register_item(
                &mut per_list_tops[i],
                (gap_left_outer + score1 as i64) as i32,
                payload.clone(),
            );
            // dict.lisp:1209 (register-item top (+ gap-left score1 gap-right) (list seg))
            kani_lite_register_item(
                &mut top,
                (gap_left_outer + score1 as i64 + gap_right_outer) as i32,
                payload,
            );
        }

        // dict.lisp:1210-1227 (loop for seg2 in rest ...)
        for j in (i + 1)..n {
            let seg2 = Arc::clone(&lite_lists[j]);
            let seg2_start = seg2.start;
            let seg2_end = seg2.end;

            if seg2_start < seg1_end {
                continue;
            }

            let score2 = get_segment_score(&KaniSegmentScoreArg::KaniLiteSegmentList(&seg2))
                .expect("post-expand segment-list carries a scored first segment");

            let gap_left = gap_penalty(seg1_end, seg2_start);
            let gap_right = gap_penalty(seg2_end, str_length);

            // dict.lisp:1215 — snapshot seg1.top entries before
            // mutating seg2.top in the inner loop.
            let tais: Vec<KaniLiteTopArrayItem> = kani_lite_get_array(&per_list_tops[i])
                .iter()
                .filter_map(|slot| slot.clone())
                .collect();

            for tai in tais {
                // dict.lisp:1216 (for (seg-left . tail) = (tai-payload tai))
                let head = tai.payload.head().unwrap_or_else(|| {
                    panic!(
                        "tai-payload must be non-empty (per-list top entries via dict.lisp:1208 / :1226)"
                    )
                });
                let seg_left_sl = match head {
                    KaniLitePathElement::SegmentList(sl) => Arc::clone(sl),
                    KaniLitePathElement::Synergy(_) => {
                        panic!("tai-payload head is always a SegmentList")
                    }
                };
                let tail: KaniLitePath = tai.payload.tail();

                let score3 =
                    get_segment_score(&KaniSegmentScoreArg::KaniLiteSegmentList(&seg_left_sl))
                        .expect("payload-head segment-list is scored");
                let score_tail = tai.score - score3;

                let splits = get_seg_splits(&seg_left_sl, &seg2);
                for split in splits {
                    let split_sum: i32 = split
                        .iter()
                        .map(|elem| {
                            let arg = match elem {
                                KaniLitePathElement::SegmentList(sl) => {
                                    KaniSegmentScoreArg::KaniLiteSegmentList(sl)
                                }
                                KaniLitePathElement::Synergy(s) => KaniSegmentScoreArg::Synergy(s),
                            };
                            get_segment_score(&arg)
                                .expect("split element is scored (get-seg-splits output)")
                        })
                        .sum();
                    let max_score = split_sum.max(score3 + 1).max(score2 + 1);
                    let accum_i64 = gap_left + max_score as i64 + score_tail as i64;
                    let accum = accum_i64 as i32;

                    // dict.lisp:1225 (for path = (nconc split tail)) — the
                    // path reads split[0], split[1], …, tail front-to-back,
                    // so the split folds onto the shared tail in reverse.
                    let mut path = tail.clone();
                    for elem in split.into_iter().rev() {
                        path = KaniLitePath::cons(elem, &path);
                    }

                    // dict.lisp:1226 (register-item (segment-list-top seg2) accum path)
                    kani_lite_register_item(&mut per_list_tops[j], accum, path.clone());
                    // dict.lisp:1227 (register-item top (+ accum gap-right) path)
                    kani_lite_register_item(&mut top, (accum_i64 + gap_right) as i32, path);
                }
            }
        }
    }

    // dict.lisp:1232-1233 — collect surviving top-K paths and
    // reconstruct full PathElements, Arc-sharing each
    // KaniLiteSegment.source.
    let mut result = Vec::new();
    for slot in kani_lite_get_array(&top) {
        let tai = slot
            .as_ref()
            .expect("get-array prefix slots are always Some (register-item invariant)");
        let mut full_path: Vec<PathElement> = tai
            .payload
            .iter()
            .map(|elem| match elem {
                KaniLitePathElement::SegmentList(lite) => {
                    PathElement::SegmentList(lite.to_segment_list())
                }
                KaniLitePathElement::Synergy(s) => PathElement::Synergy(s.clone()),
            })
            .collect();
        full_path.reverse();
        result.push((full_path, tai.score));
    }
    Ok(result)
}

#[cfg(test)]
mod tests;
