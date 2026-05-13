//! Port of `ichiran/dict:fill-segment-path` (`dict.lisp:1390`).
//!
//! Walks a `find-best-path` result (heterogeneous list of
//! [`PathElement`]s — each is a [`SegmentList`] or a
//! [`super::synergy_struct::Synergy`]) and builds the flat
//! [`WordInfo`] sequence the JSON / display pipeline consumes:
//! gap-typed word-infos fill the runs between segment-list slices,
//! and each segment-list lifts via [`word_info_from_segment_list`].
//! Synergy elements are filtered out (upstream's
//! `(typep _ 'segment-list)` guard). The flat output runs through
//! [`super::process_word_info::process_word_info`] (the 何-reading
//! fixup) before returning.
//!
//! Diverges from the upstream lambda list `(str path)` by taking
//! `&KaniranContext` for the database handle per
//! [`crate::conn::kani_context`]. `path` is `&mut [PathElement]` so
//! `word_info_from_segment_list` can run the per-segment
//! `(get-text segment)` memoization in place. Character offsets follow
//! CONVENTIONS §4.5: `subseq str start end` is char-indexed in SBCL,
//! and `make_substr_gap` slices via `chars().skip().take()`.

use crate::conn::kani_context::KaniranContext;
use crate::dict::process_word_info::process_word_info;
use crate::dict::top_array_item_struct::PathElement;
use crate::dict::word_info_class::{WordInfo, WordInfoKana, WordInfoType};
use crate::dict::word_info_from_segment_list::word_info_from_segment_list;

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

#[cfg(test)]
mod tests {
    //! Unit tests against the real .103 PG via `KaniranContext::from_env()`.
    //! Coverage:
    //! - leading / internal / trailing gap insertion
    //! - empty path with non-empty string emits one full-string gap
    //! - empty path + empty string emits nothing
    //! - synergy elements are filtered out
    //! - char-indexed slicing (multibyte chars don't shift offsets)
    use super::*;
    use crate::dict::find_word::{find_word, FindWordRows};
    use crate::dict::kani_word::KaniWordDispatchEnum;
    use crate::dict::segment_list_struct::SegmentList;
    use crate::dict::segment_struct::Segment;
    use crate::dict::synergy_struct::Synergy;
    use crate::dict::word_info_class::WordInfoSeq;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    async fn first_reading(ctx: &KaniranContext, word: &str) -> KaniWordDispatchEnum {
        let rows = find_word(ctx, word, false, None).await.unwrap();
        match rows {
            FindWordRows::Kanji(v) => v
                .into_iter()
                .next()
                .map(KaniWordDispatchEnum::Kanji)
                .expect("no kanji rows"),
            FindWordRows::Kana(v) => v
                .into_iter()
                .next()
                .map(KaniWordDispatchEnum::Kana)
                .expect("no kana rows"),
        }
    }

    async fn one_seg_list(
        ctx: &KaniranContext,
        word: &str,
        score: i32,
        start: usize,
        end: usize,
    ) -> SegmentList {
        let reading = first_reading(ctx, word).await;
        SegmentList {
            segments: vec![Segment {
                start,
                end,
                word: reading,
                score: Some(score),
                info: None,
                top: None,
                text: None,
            }],
            start,
            end,
            top: None,
            matches: 1,
        }
    }

    #[tokio::test]
    async fn fills_internal_gap_between_two_segment_lists() {
        let ctx = ctx_from_env().await;
        let sl_neko = one_seg_list(&ctx, "ねこ", 16, 0, 2).await;
        let sl_inu = one_seg_list(&ctx, "いぬ", 16, 4, 6).await;
        let mut path = vec![
            PathElement::SegmentList(sl_neko),
            PathElement::SegmentList(sl_inu),
        ];
        let result = fill_segment_path(&ctx, "ねこと いぬ", &mut path)
            .await
            .unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].text, "ねこ");
        assert_eq!(result[0].seq, Some(WordInfoSeq::Single(1467640)));
        assert_eq!(result[1].kind, WordInfoType::Gap);
        assert_eq!(result[1].text, "と ");
        assert_eq!(
            result[1].kana,
            Some(WordInfoKana::Single("と ".to_string()))
        );
        assert_eq!(result[1].start, Some(2));
        assert_eq!(result[1].end, Some(4));
        assert!(result[1].seq.is_none());
        assert_eq!(result[2].text, "いぬ");
        assert_eq!(result[2].seq, Some(WordInfoSeq::Single(1258330)));
    }

    #[tokio::test]
    async fn fills_leading_and_trailing_gap() {
        let ctx = ctx_from_env().await;
        let sl = one_seg_list(&ctx, "ねこ", 16, 2, 4).await;
        let mut path = vec![PathElement::SegmentList(sl)];
        let result = fill_segment_path(&ctx, "あいねこ犬", &mut path)
            .await
            .unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].kind, WordInfoType::Gap);
        assert_eq!(result[0].text, "あい");
        assert_eq!(result[0].start, Some(0));
        assert_eq!(result[0].end, Some(2));
        assert_eq!(result[1].text, "ねこ");
        assert_eq!(result[2].kind, WordInfoType::Gap);
        assert_eq!(result[2].text, "犬");
        assert_eq!(result[2].start, Some(4));
        assert_eq!(result[2].end, Some(5));
    }

    #[tokio::test]
    async fn empty_path_with_text_emits_single_gap() {
        let ctx = ctx_from_env().await;
        let result = fill_segment_path(&ctx, "abcde", &mut []).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].kind, WordInfoType::Gap);
        assert_eq!(result[0].text, "abcde");
        assert_eq!(
            result[0].kana,
            Some(WordInfoKana::Single("abcde".to_string()))
        );
        assert_eq!(result[0].start, Some(0));
        assert_eq!(result[0].end, Some(5));
    }

    #[tokio::test]
    async fn empty_path_empty_string_emits_nothing() {
        let ctx = ctx_from_env().await;
        let result = fill_segment_path(&ctx, "", &mut []).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn segment_list_covers_entire_string_no_gap() {
        let ctx = ctx_from_env().await;
        let sl = one_seg_list(&ctx, "ねこ", 16, 0, 2).await;
        let mut path = vec![PathElement::SegmentList(sl)];
        let result = fill_segment_path(&ctx, "ねこ", &mut path).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].text, "ねこ");
    }

    #[tokio::test]
    async fn synergy_elements_are_skipped() {
        let ctx = ctx_from_env().await;
        let sl_neko = one_seg_list(&ctx, "ねこ", 16, 0, 2).await;
        let sl_inu = one_seg_list(&ctx, "いぬ", 16, 4, 6).await;
        let mut path = vec![
            PathElement::SegmentList(sl_neko),
            PathElement::Synergy(Synergy {
                description: Some("stub".into()),
                connector: Some(" + ".into()),
                score: 5,
                start: 2,
                end: 4,
            }),
            PathElement::SegmentList(sl_inu),
        ];
        let result = fill_segment_path(&ctx, "ねこと いぬ", &mut path)
            .await
            .unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].text, "ねこ");
        assert_eq!(result[1].kind, WordInfoType::Gap);
        assert_eq!(result[2].text, "いぬ");
    }
}
