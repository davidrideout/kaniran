//! Port of `ichiran/dict:join-substring-words*` (`dict.lisp:1071`).
//!
//! ```lisp
//! (defun join-substring-words* (str)
//!   (loop with sticky = (find-sticky-positions str)
//!         with substring-hash = (find-substring-words str :sticky sticky)
//!         with katakana-groups = (consecutive-char-groups :katakana str)
//!         with number-groups = (consecutive-char-groups :number str)
//!         and kanji-break and ends
//!        with suffix-map = (get-suffix-map str)
//!        for start from 0 below (length str)
//!        for katakana-group-end = (cdr (assoc start katakana-groups))
//!        for number-group-end = (cdr (assoc start number-groups))
//!        unless (member start sticky)
//!        nconcing
//!        (loop for end from (1+ start) upto (min (length str) (+ start *max-word-length*))
//!             unless (member end sticky)
//!             nconcing
//!             (let* ((part (subseq str start end))
//!                    (segments (mapcar (lambda (word) (make-segment :start start :end end :word word))
//!                               (let ((*suffix-map-temp* suffix-map) (*suffix-next-end* end)
//!                                     (*substring-hash* substring-hash))
//!                                 (find-word-full part :as-hiragana ... :counter ...)))))
//!               (when segments
//!                 (when (or (= start 0) (find start ends))
//!                   (setf kanji-break (nconc (cond ...) kanji-break)))
//!                 (pushnew end ends)
//!                 (list (list start end segments)))))
//!        into result
//!      finally (return (values result (remove-duplicates kanji-break)))))
//! ```
//!
//! Enumerates every length-bounded `(start, end)` substring that is not
//! blocked by a sticky position, collects the segments
//! [`find_word_full`] yields for each, and accumulates the kanji-break
//! positions reachable from prior segment ends.
//!
//! Divergences: the CL `(values result kanji-break)` becomes a tuple;
//! the `*suffix-map-temp*` / `*suffix-next-end*` / `*substring-hash*`
//! rebinds become sibling-ctx construction. Offsets are character
//! positions.

use std::sync::Arc;

use crate::characters::char_classes::CharClass;
use crate::characters::text_utils::consecutive_char_groups;
use crate::characters::kanji::sequential_kanji_positions;
use crate::conn::kani_context::KaniranContext;
use crate::dict::_star_force_kanji_break_star_::FORCE_KANJI_BREAK;
use crate::dict::_star_max_word_length_star_::MAX_WORD_LENGTH;
use crate::dict::_star_no_kanji_break_star_::NO_KANJI_BREAK;
use crate::dict::_star_suffix_map_temp_star_::SuffixMapTemp;
use crate::dict::find_sticky_positions::find_sticky_positions;
use crate::dict::find_substring_words::find_substring_words;
use crate::dict::find_word_full::{find_word_full, CounterArg};
use crate::dict::get_suffix_map::get_suffix_map;
use crate::dict::segment_struct::Segment;

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

#[cfg(test)]
mod tests {
    //! Every assertion is REPL-verified against the .103 SBCL via
    //! `(ichiran/dict::join-substring-words* …)` (2026-05-23 probe runs).
    //! Run with `cargo test ... -- --test-threads=1` per the DB-test
    //! convention.
    use super::*;
    use crate::dict::kani_word::KaniWordDispatchEnum;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
    }

    /// `(start, end, segment-count)` shape of the result — the
    /// loop-bound / sticky / find-word behavior that the function owns.
    fn shape(result: &[(usize, usize, Vec<Segment>)]) -> Vec<(usize, usize, usize)> {
        result.iter().map(|(s, e, segs)| (*s, *e, segs.len())).collect()
    }

    /// REPL `(join-substring-words* "日本語")`:
    /// `[0 1] n=4 [0 2] n=1 [0 3] n=1 [1 2] n=2 [2 3] n=2`,
    /// kanji-break `(2 1)` — sequential-kanji-positions accumulated
    /// across reachable starts, deduped keep-last.
    #[tokio::test]
    async fn nihongo_kanji_run() {
        let ctx = ctx().await;
        let (result, kanji_break) = join_substring_words_star_(&ctx, "日本語").await.unwrap();
        assert_eq!(
            shape(&result),
            vec![(0, 1, 4), (0, 2, 1), (0, 3, 1), (1, 2, 2), (2, 3, 2)]
        );
        assert_eq!(kanji_break, vec![2, 1]);
    }

    /// REPL `(join-substring-words* "特大")`:
    /// `[0 2] n=1 [1 2] n=5`, kanji-break `(1)`. start=1 is not in
    /// `ends` so its segment does not add to kanji-break.
    #[tokio::test]
    async fn tokudai_start_not_reachable() {
        let ctx = ctx().await;
        let (result, kanji_break) = join_substring_words_star_(&ctx, "特大").await.unwrap();
        assert_eq!(shape(&result), vec![(0, 2, 1), (1, 2, 5)]);
        assert_eq!(kanji_break, vec![1]);
    }

    /// REPL `(join-substring-words* "私は学生です")`:
    /// kanji-break `(5 3)`. The `[4 6]` slice "です" is in
    /// *force-kanji-break* → iota over its interior (position 5); the
    /// `[2 4]` slice "学生" contributes the sequential position 3.
    #[tokio::test]
    async fn watashi_force_kanji_break_desu() {
        let ctx = ctx().await;
        let (result, kanji_break) =
            join_substring_words_star_(&ctx, "私は学生です").await.unwrap();
        assert_eq!(
            shape(&result),
            vec![
                (0, 1, 14),
                (1, 2, 11),
                (2, 3, 1),
                (2, 4, 2),
                (3, 4, 7),
                (4, 5, 4),
                (4, 6, 2),
                (5, 6, 10),
            ]
        );
        assert_eq!(kanji_break, vec![5, 3]);
    }

    /// REPL `(join-substring-words* "一日置く")`: the `[1 3]` slice
    /// "日置" is in *no-kanji-break*, so the sequential position 2 it
    /// would otherwise contribute is suppressed — kanji-break is `(1)`,
    /// not `(2 1)`.
    #[tokio::test]
    async fn ichinichi_no_kanji_break() {
        let ctx = ctx().await;
        let (result, kanji_break) = join_substring_words_star_(&ctx, "一日置く").await.unwrap();
        assert_eq!(
            shape(&result),
            vec![(0, 1, 6), (0, 2, 5), (1, 2, 4), (1, 3, 1), (2, 4, 1), (3, 4, 8)]
        );
        assert_eq!(kanji_break, vec![1]);
        // The [1 3] "日置" slice is present but suppresses its break.
        assert!(result.iter().any(|(s, e, _)| *s == 1 && *e == 3));
    }

    /// REPL `(join-substring-words* "コーヒー")` (sticky=(1)): the
    /// katakana group spans 0..4, so the `[0 4]` slice runs
    /// find-word-full with as-hiragana=T and yields the kana row.
    /// start=1 and end=1 are sticky → absent.
    #[tokio::test]
    async fn coffee_as_hiragana_and_sticky() {
        let ctx = ctx().await;
        let (result, kanji_break) = join_substring_words_star_(&ctx, "コーヒー").await.unwrap();
        assert_eq!(shape(&result), vec![(0, 4, 1), (3, 4, 1)]);
        assert!(kanji_break.is_empty());
        // No slice starts or ends at the sticky position 1.
        assert!(!result.iter().any(|(s, e, _)| *s == 1 || *e == 1));
        // [0 4] is the existing コーヒー kana row (as-hiragana path).
        let (_, _, segs) = result.iter().find(|(s, e, _)| *s == 0 && *e == 4).unwrap();
        assert!(matches!(segs[0].word, KaniWordDispatchEnum::Kana(_)));
    }

    /// REPL `(join-substring-words* "5本")`: the number group at 0..1
    /// drives the :counter argument. `[0 1]` "5" yields a NUMBER-TEXT;
    /// `[0 2]` "5本" yields COUNTER-TEXT + COUNTER-HIFUMI; `[1 2]` "本"
    /// is two plain KANJI-TEXT.
    #[tokio::test]
    async fn counter_number_group() {
        let ctx = ctx().await;
        let (result, kanji_break) = join_substring_words_star_(&ctx, "5本").await.unwrap();
        assert_eq!(shape(&result), vec![(0, 1, 1), (0, 2, 2), (1, 2, 2)]);
        assert!(kanji_break.is_empty());
        let (_, _, num) = result.iter().find(|(s, e, _)| *s == 0 && *e == 1).unwrap();
        assert!(matches!(num[0].word, KaniWordDispatchEnum::Counter(_)));
        let (_, _, cnt) = result.iter().find(|(s, e, _)| *s == 0 && *e == 2).unwrap();
        assert!(cnt.iter().all(|seg| matches!(seg.word, KaniWordDispatchEnum::Counter(_))));
        let (_, _, hon) = result.iter().find(|(s, e, _)| *s == 1 && *e == 2).unwrap();
        assert!(hon.iter().all(|seg| matches!(seg.word, KaniWordDispatchEnum::Kanji(_))));
    }

    /// REPL `(join-substring-words* "やっぱり")` (sticky=(2)): the
    /// sokuon makes position 2 sticky, so no slice starts or ends
    /// there. kanji-break empty (all-kana input).
    #[tokio::test]
    async fn yappari_sokuon_sticky() {
        let ctx = ctx().await;
        let (result, kanji_break) = join_substring_words_star_(&ctx, "やっぱり").await.unwrap();
        assert_eq!(
            shape(&result),
            vec![(0, 1, 9), (0, 3, 1), (0, 4, 1), (1, 3, 1), (3, 4, 8)]
        );
        assert!(kanji_break.is_empty());
        assert!(!result.iter().any(|(s, e, _)| *s == 2 || *e == 2));
    }

    /// REPL `(join-substring-words* "")`: empty input → empty result,
    /// empty kanji-break (the outer loop range is empty).
    #[tokio::test]
    async fn empty_string() {
        let ctx = ctx().await;
        let (result, kanji_break) = join_substring_words_star_(&ctx, "").await.unwrap();
        assert!(result.is_empty());
        assert!(kanji_break.is_empty());
    }

    /// `remove-duplicates` keep-last semantics, pinned directly:
    /// REPL `(remove-duplicates '(1 2 1))` → `(2 1)`.
    #[test]
    fn remove_duplicates_keeps_last() {
        assert_eq!(remove_duplicates(&[1, 2, 1]), vec![2, 1]);
        assert_eq!(remove_duplicates(&[5, 3]), vec![5, 3]);
        assert_eq!(remove_duplicates(&[]), Vec::<usize>::new());
        assert_eq!(remove_duplicates(&[2, 2, 2]), vec![2]);
    }
}
