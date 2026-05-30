//! Port of the dict.lisp best-path / segmentation-runtime layer —
//! find-best-path + adjoin-word + join-substring-words[-star] +
//! substring-index + fill-segment-path + word-info-rec-find +
//! process-word-info plus the segmenter scoring cutoffs.

pub use find_best_path_inner::*;
pub use adjoin_word_inner::*;
pub use join_substring_words_inner::*;
pub use join_substring_words_star__inner::*;
pub use substring_index_inner::*;
pub use fill_segment_path_inner::*;
pub use word_info_rec_find_inner::*;
pub use process_word_info_inner::*;
pub use _star_score_cutoff_star__inner::*;
pub use _star_identical_word_score_cutoff_star__inner::*;
pub use _star_segment_score_cutoff_star__inner::*;
pub use _star_suffix_map_temp_star__inner::*;
pub use _star_suffix_next_end_star__inner::*;

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod find_best_path_inner {
use std::sync::Arc;

use crate::conn::kani_context::KaniranContext;
use crate::dict::segment::expand_segment_list;
use crate::dict::segment::gap_penalty;
use crate::dict::segment::get_seg_initial;
use crate::dict::segment::get_seg_splits;
use crate::dict::segment::{get_segment_score, KaniSegmentScoreArg};
use crate::dict::kani::KaniLiteSegmentList;
use crate::dict::kani::{
    kani_lite_get_array, kani_lite_register_item, KaniLiteTopArray,
};
use crate::dict::kani::{KaniLitePathElement, KaniLiteTopArrayItem};
use crate::dict::segment::SegmentList;
use crate::dict::segment::PathElement;

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
    let mut per_list_tops: Vec<KaniLiteTopArray> =
        (0..lite_lists.len()).map(|_| KaniLiteTopArray::new(limit)).collect();

    // dict.lisp:1192 (let ((top (make-instance 'top-array :limit limit))))
    let mut top = KaniLiteTopArray::new(limit);

    // dict.lisp:1193 (register-item top (gap-penalty 0 str-length) nil)
    kani_lite_register_item(
        &mut top,
        gap_penalty(0, str_length) as i32,
        Arc::<[KaniLitePathElement]>::from(Vec::new()),
    );

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
            let score1 =
                get_segment_score(&KaniSegmentScoreArg::KaniLiteSegmentList(&seg))
                    .expect("get-seg-initial output carries a scored first segment");
            let payload: Arc<[KaniLitePathElement]> = Arc::from(vec![
                KaniLitePathElement::SegmentList(Arc::clone(&seg)),
            ]);
            // dict.lisp:1208 (register-item (segment-list-top seg1) (+ gap-left score1) (list seg))
            kani_lite_register_item(
                &mut per_list_tops[i],
                (gap_left_outer + score1 as i64) as i32,
                Arc::clone(&payload),
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

            let score2 =
                get_segment_score(&KaniSegmentScoreArg::KaniLiteSegmentList(&seg2))
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
                let payload_slice: &[KaniLitePathElement] = &tai.payload;
                if payload_slice.is_empty() {
                    panic!(
                        "tai-payload must be non-empty (per-list top entries via dict.lisp:1208 / :1226)"
                    );
                }
                let seg_left_sl = match &payload_slice[0] {
                    KaniLitePathElement::SegmentList(sl) => Arc::clone(sl),
                    KaniLitePathElement::Synergy(_) => {
                        panic!("tai-payload head is always a SegmentList")
                    }
                };
                let tail: &[KaniLitePathElement] = &payload_slice[1..];

                let score3 = get_segment_score(&KaniSegmentScoreArg::KaniLiteSegmentList(
                    &seg_left_sl,
                ))
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
                                KaniLitePathElement::Synergy(s) => {
                                    KaniSegmentScoreArg::Synergy(s)
                                }
                            };
                            get_segment_score(&arg)
                                .expect("split element is scored (get-seg-splits output)")
                        })
                        .sum();
                    let max_score = split_sum.max(score3 + 1).max(score2 + 1);
                    let accum_i64 = gap_left + max_score as i64 + score_tail as i64;
                    let accum = accum_i64 as i32;

                    // dict.lisp:1225 (for path = (nconc split tail))
                    let mut path_vec: Vec<KaniLitePathElement> = split;
                    path_vec.extend_from_slice(tail);
                    let path: Arc<[KaniLitePathElement]> = Arc::from(path_vec);

                    // dict.lisp:1226 (register-item (segment-list-top seg2) accum path)
                    kani_lite_register_item(&mut per_list_tops[j], accum, Arc::clone(&path));
                    // dict.lisp:1227 (register-item top (+ accum gap-right) path)
                    kani_lite_register_item(&mut top, (accum_i64 + gap_right) as i32, path);
                }
            }
        }
    }

    // dict.lisp:1232-1233 — collect surviving top-K paths and
    // reconstruct full PathElements via deep-clone of each
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
mod tests {
    //! Empty-input unit tests pinned against `.103` REPL probes
    //! (SBCL 2.2.9, 2026-05-19). The DB-dependent non-empty paths
    //! (outer loop, get-seg-initial, get-seg-splits accumulation) are
    //! covered by the 522K-row audit binary at
    //! `audit/dict/find_best_path_test.rs`.

    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    // REPL: (ichiran/dict::find-best-path nil 5) => ((NIL . -2500))
    #[tokio::test]
    async fn empty_input_length_5_default_limit() {
        let ctx = ctx_from_env().await;
        let result = find_best_path(&ctx, &mut [], 5, None).await.unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_empty(), "initial gap-seed has empty payload");
        assert_eq!(result[0].1, -2500);
    }

    // REPL: (ichiran/dict::find-best-path nil 0) => ((NIL . 0))
    #[tokio::test]
    async fn empty_input_length_0() {
        let ctx = ctx_from_env().await;
        let result = find_best_path(&ctx, &mut [], 0, None).await.unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_empty());
        assert_eq!(result[0].1, 0);
    }

    // REPL: (ichiran/dict::find-best-path nil 1 :limit 3) => ((NIL . -500))
    #[tokio::test]
    async fn empty_input_length_1_limit_3() {
        let ctx = ctx_from_env().await;
        let result = find_best_path(&ctx, &mut [], 1, Some(3)).await.unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_empty());
        assert_eq!(result[0].1, -500);
    }

    // REPL: (ichiran/dict::find-best-path nil 1 :limit 1) => ((NIL . -500))
    #[tokio::test]
    async fn empty_input_length_1_limit_1() {
        let ctx = ctx_from_env().await;
        let result = find_best_path(&ctx, &mut [], 1, Some(1)).await.unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].0.is_empty());
        assert_eq!(result[0].1, -500);
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod adjoin_word_inner {
use crate::conn::kani_context::KaniranContext;
use crate::dict::text_classes::{CompoundText, ScoreMod};
use crate::dict::best_text::get_kana;
use crate::dict::best_text::get_text;
use crate::dict::kani::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};

pub async fn adjoin_word(
    ctx: &KaniranContext,
    word1: KaniWordDispatchEnum,
    word2: KaniSimpleTextDispatchEnum,
    text: Option<String>,
    kana: Option<String>,
    score_mod: Option<ScoreMod>,
    score_base: Option<KaniWordDispatchEnum>,
) -> Result<CompoundText, sqlx::Error> {
    // dict.lisp:635-640 (defmethod adjoin-word :around (t t))
    let resolved_text = match text {
        Some(t) => t,
        None => {
            let word2_as_word = word2.to_word();
            let t1 = get_text(&word1);
            let t2 = get_text(&word2_as_word);
            format!("{}{}", t1, t2)
        }
    };
    let resolved_kana = match kana {
        Some(k) => k,
        None => {
            let word2_as_word = word2.to_word();
            // dict.lisp:638 — (concatenate 'string (get-kana word1) (get-kana word2)).
            // Upstream `(concatenate 'string nil ...)` accepts nil as
            // the empty sequence; the Rust `Option<String>` from
            // `get_kana` mirrors that with `.unwrap_or_default()`.
            let k1 = get_kana(ctx, &word1).await?.unwrap_or_default();
            let k2 = get_kana(ctx, &word2_as_word).await?.unwrap_or_default();
            format!("{}{}", k1, k2)
        }
    };
    // dict.lisp:639 — (or score-mod 0).
    // `(or score-mod 0)` evaluates to 0 only when score-mod is nil;
    // any truthy value (integer literal or `(constantly N)` closure)
    // passes through unchanged. The Rust port collapses `None` and
    // the integer-literal default to `ScoreMod::Single(0)`.
    let resolved_score_mod = score_mod.unwrap_or(ScoreMod::Single(0));

    match word1 {
        // dict.lisp:642-645 (defmethod adjoin-word ((word1 simple-text) (word2 simple-text)))
        KaniWordDispatchEnum::Kanji(_)
        | KaniWordDispatchEnum::Kana(_)
        | KaniWordDispatchEnum::Proxy(_) => {
            let word2_as_word = word2.to_word();
            // dict.lisp:644 — `:primary word1 :words (list word1 word2)`.
            // Lisp aliases the same word1 cell into both slots; the
            // Rust port clones into `primary` and moves the original
            // into `words`.
            let primary = Box::new(word1.clone());
            Ok(CompoundText {
                text: resolved_text,
                kana: resolved_kana,
                primary,
                words: vec![word1, word2_as_word],
                score_base: score_base.map(Box::new),
                score_mod: resolved_score_mod,
            })
        }
        // dict.lisp:647-652 (defmethod adjoin-word ((word1 compound-text) (word2 simple-text)))
        KaniWordDispatchEnum::Compound(mut compound) => {
            // dict.lisp:649 — setf text/kana on word1.
            compound.text = resolved_text;
            compound.kana = resolved_kana;
            // dict.lisp:650 — (append s-words (list word2)).
            compound.words.push(word2.to_word());
            // dict.lisp:651 — (funcall (if (listp s-score-mod) 'cons 'list)
            //                          score-mod s-score-mod).
            // `cons` prepends onto an existing list; `list` wraps two
            // non-list values into a fresh 2-list. Both branches end
            // with new value at index 0 and old at index 1+.
            compound.score_mod = match compound.score_mod {
                ScoreMod::Stack(stack) => {
                    let mut new_stack = Vec::with_capacity(stack.len() + 1);
                    new_stack.push(resolved_score_mod);
                    new_stack.extend(stack);
                    ScoreMod::Stack(new_stack)
                }
                other @ (ScoreMod::Single(_) | ScoreMod::Constant(_)) => {
                    ScoreMod::Stack(vec![resolved_score_mod, other])
                }
            };
            // dict.lisp:647 — `&allow-other-keys` drops :score-base;
            // word1's score-base slot is unchanged. The `score_base`
            // parameter is discarded in this branch.
            let _ = score_base;
            // dict.lisp:652 — `word1` is returned.
            Ok(compound)
        }
        // dict.lisp:632 — no method specialized on counter-text.
        // Upstream would signal `no-applicable-method`.
        KaniWordDispatchEnum::Counter(_) => {
            unreachable!(
                "adjoin-word has no method specialized on counter-text \
                 (upstream signals no-applicable-method)"
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::dao::KanaText;
    use crate::dict::dao::KanjiText;
    use crate::dict::text_classes::SimpleText;

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

    // The `:around` text/kana defaults reach `get-kana` which hits
    // the database for kanji-text inputs; the unit tests below stay
    // synchronous by passing explicit `:text` / `:kana` keywords so
    // the `:around` defaulting paths fall through without touching
    // the DB. The text/kana defaulting paths are exercised in the
    // REPL probe `tests/repl_adjoin_concat_defaults` (see the
    // `repl_*` tests below) and pinned against the REPL transcript.

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    // ----- (simple-text, simple-text) primary method -----

    #[tokio::test]
    async fn simple_simple_explicit_text_and_kana() {
        // T2 from REPL probe of upstream ichiran:
        //   (adjoin-word w1 w2 :text "abc" :kana "xyz" :score-mod 7)
        //   => COMPOUND-TEXT text="abc" kana="xyz" score-mod=7
        //                    score-base=NIL words=("食べ" "たい")
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let result = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("abc".into()),
            Some("xyz".into()),
            Some(ScoreMod::Single(7)),
            None,
        )
        .await
        .unwrap();
        assert_eq!(result.text, "abc");
        assert_eq!(result.kana, "xyz");
        assert!(matches!(result.score_mod, ScoreMod::Single(7)));
        assert!(result.score_base.is_none());
        assert_eq!(result.words.len(), 2);
    }

    #[tokio::test]
    async fn simple_simple_primary_is_word1() {
        // T1 from REPL probe of upstream ichiran:
        //   words=("食べ" "たい"); primary is word1.
        // Pinned: result.primary derefs to the word1 input.
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let result = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("食べたい".into()),
            Some("たべたい".into()),
            None,
            None,
        )
        .await
        .unwrap();
        match &*result.primary {
            KaniWordDispatchEnum::Kanji(k) => assert_eq!(k.seq, 10092273),
            _ => panic!("primary must be the input word1 (kanji-text)"),
        }
        // Words are [word1, word2].
        assert_eq!(result.words.len(), 2);
    }

    #[tokio::test]
    async fn simple_simple_score_mod_none_defaults_to_zero() {
        // T1 from REPL probe of upstream ichiran:
        //   (adjoin-word w1 w2)  => score-mod=0
        // T8: explicit :score-mod nil => score-mod=0
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let result = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("食べたい".into()),
            Some("たべたい".into()),
            None,
            None,
        )
        .await
        .unwrap();
        assert!(matches!(result.score_mod, ScoreMod::Single(0)));
    }

    #[tokio::test]
    async fn simple_simple_score_base_passthrough() {
        // T5 from REPL probe of upstream ichiran:
        //   (adjoin-word w1 w2 :score-base w1)
        //   => score-base text="食べ"
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let sb = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let result = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("食べたい".into()),
            Some("たべたい".into()),
            Some(ScoreMod::Single(0)),
            Some(sb),
        )
        .await
        .unwrap();
        match result.score_base.as_deref() {
            Some(KaniWordDispatchEnum::Kanji(k)) => assert_eq!(k.text, "食べ"),
            _ => panic!("score-base must carry the kanji-text we passed in"),
        }
    }

    // ----- (compound-text, simple-text) primary method -----

    #[tokio::test]
    async fn compound_simple_appends_words_and_updates_text_kana() {
        // T3 from REPL probe of upstream ichiran:
        //   c3 = (adjoin-word w1 w2 :score-mod 3)  ; "食べたい" / "たべたい"
        //   c3b = (adjoin-word c3 w3 :score-mod 4) ; w3 = "ない"
        //   => c3b text="食べたいない" kana="たべたいない"
        //      words-text=("食べ" "たい" "ない")
        //      score-mod=(4 3)
        //      (eq c3 c3b)=T
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let c3 = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("食べたい".into()),
            Some("たべたい".into()),
            Some(ScoreMod::Single(3)),
            None,
        )
        .await
        .unwrap();
        let w3 = KaniSimpleTextDispatchEnum::Kana(kana(2257550, "ない"));
        let c3b = adjoin_word(
            &ctx,
            KaniWordDispatchEnum::Compound(c3),
            w3,
            Some("食べたいない".into()),
            Some("たべたいない".into()),
            Some(ScoreMod::Single(4)),
            None,
        )
        .await
        .unwrap();
        assert_eq!(c3b.text, "食べたいない");
        assert_eq!(c3b.kana, "たべたいない");
        assert_eq!(c3b.words.len(), 3);
        // dict.lisp:651 — first compound,simple adjoin: (list new old)
        match &c3b.score_mod {
            ScoreMod::Stack(v) => {
                assert_eq!(v.len(), 2);
                assert!(matches!(v[0], ScoreMod::Single(4)));
                assert!(matches!(v[1], ScoreMod::Single(3)));
            }
            _ => panic!("score-mod must be Stack([4, 3]) after first compound adjoin"),
        }
    }

    #[tokio::test]
    async fn compound_simple_third_adjoin_grows_stack() {
        // T4 from REPL probe of upstream ichiran:
        //   c4  = (adjoin-word w1 w2 :score-mod 1)   ; single 1
        //   c4b = (adjoin-word c4 w3 :score-mod 2)   ; (2 1)
        //   c4c = (adjoin-word c4b w4 :score-mod 5)  ; (5 2 1)
        //   words-text=("食べ" "たい" "ない" "だ")
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let c4 = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("食べたい".into()),
            Some("たべたい".into()),
            Some(ScoreMod::Single(1)),
            None,
        )
        .await
        .unwrap();
        let w3 = KaniSimpleTextDispatchEnum::Kana(kana(2257550, "ない"));
        let c4b = adjoin_word(
            &ctx,
            KaniWordDispatchEnum::Compound(c4),
            w3,
            Some("食べたいない".into()),
            Some("たべたいない".into()),
            Some(ScoreMod::Single(2)),
            None,
        )
        .await
        .unwrap();
        let w4 = KaniSimpleTextDispatchEnum::Kana(kana(2089020, "だ"));
        let c4c = adjoin_word(
            &ctx,
            KaniWordDispatchEnum::Compound(c4b),
            w4,
            Some("食べたいないだ".into()),
            Some("たべたいないだ".into()),
            Some(ScoreMod::Single(5)),
            None,
        )
        .await
        .unwrap();
        assert_eq!(c4c.words.len(), 4);
        match &c4c.score_mod {
            ScoreMod::Stack(v) => {
                assert_eq!(v.len(), 3);
                assert!(matches!(v[0], ScoreMod::Single(5)));
                assert!(matches!(v[1], ScoreMod::Single(2)));
                assert!(matches!(v[2], ScoreMod::Single(1)));
            }
            _ => panic!("score-mod must be Stack([5, 2, 1]) after third adjoin"),
        }
    }

    #[tokio::test]
    async fn compound_simple_ignores_score_base() {
        // T6 from REPL probe of upstream ichiran:
        //   (adjoin-word w1 w2 :score-mod 1 :score-base w1)  ; sets sb=w1
        //   (adjoin-word c6 w3 :score-mod 2 :score-base w2)  ; sb stays w1
        //   => after 2nd adjoin: score-base text="食べ"
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let sb_w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let c6 = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("食べたい".into()),
            Some("たべたい".into()),
            Some(ScoreMod::Single(1)),
            Some(sb_w1),
        )
        .await
        .unwrap();
        let w3 = KaniSimpleTextDispatchEnum::Kana(kana(2257550, "ない"));
        // Try to overwrite with :score-base w2 — should be ignored.
        let sb_w2 = KaniWordDispatchEnum::Kana(kana(1406940, "たい"));
        let c6b = adjoin_word(
            &ctx,
            KaniWordDispatchEnum::Compound(c6),
            w3,
            Some("食べたいない".into()),
            Some("たべたいない".into()),
            Some(ScoreMod::Single(2)),
            Some(sb_w2),
        )
        .await
        .unwrap();
        match c6b.score_base.as_deref() {
            Some(KaniWordDispatchEnum::Kanji(k)) => assert_eq!(k.text, "食べ"),
            _ => panic!("score-base must remain the originally-set w1 (Kanji '食べ')"),
        }
    }

    #[tokio::test]
    async fn compound_simple_primary_unchanged() {
        // T9 from REPL probe of upstream ichiran:
        //   After (adjoin-word c9 w3 ...) the primary slot's text is
        //   still "食べ" (the original word1 from the first adjoin),
        //   not w3's "ない".
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let c9 = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("食べたい".into()),
            Some("たべたい".into()),
            Some(ScoreMod::Single(1)),
            None,
        )
        .await
        .unwrap();
        let w3 = KaniSimpleTextDispatchEnum::Kana(kana(2257550, "ない"));
        let c9b = adjoin_word(
            &ctx,
            KaniWordDispatchEnum::Compound(c9),
            w3,
            Some("食べたいない".into()),
            Some("たべたいない".into()),
            Some(ScoreMod::Single(2)),
            None,
        )
        .await
        .unwrap();
        match &*c9b.primary {
            KaniWordDispatchEnum::Kanji(k) => {
                assert_eq!(k.text, "食べ");
                assert_eq!(k.seq, 10092273);
            }
            _ => panic!("primary must remain the original word1 kanji-text"),
        }
    }

    // ----- :around default computation (hits DB via get-kana) -----

    #[tokio::test]
    async fn around_defaults_text_and_kana_to_concat() {
        // T1 from REPL probe of upstream ichiran:
        //   w1 = 食べ kanji-text seq 10092273 (get-kana "たべ" via best-kana-conj)
        //   w2 = たい kana-text  seq 1406940
        //   (adjoin-word w1 w2)
        //   => text="食べたい" kana="たべたい" score-mod=0
        //
        // Exercises the `:around` defaulting path: text concatenates
        // get-text outputs, kana concatenates get-kana outputs
        // (kanji-text's best-kana-conj hits the conjugation tables).
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let result = adjoin_word(&ctx, w1, w2, None, None, None, None).await.unwrap();
        assert_eq!(result.text, "食べたい");
        assert_eq!(result.kana, "たべたい");
        assert!(matches!(result.score_mod, ScoreMod::Single(0)));
    }

    // ----- ScoreMod::Constant — (constantly N) callsites -----

    #[tokio::test]
    async fn simple_simple_constant_score_mod() {
        // REPL probe of upstream ichiran:
        //   (adjoin-word w1 w2 :text "ab" :kana "ab"
        //                      :score-mod (constantly 360))
        //   => slot = #<FUNCTION>  (functionp ✓, funcall returns 360
        //      regardless of argument).
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let result = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("食べたい".into()),
            Some("たべたい".into()),
            Some(ScoreMod::Constant(360)),
            None,
        )
        .await
        .unwrap();
        assert!(matches!(result.score_mod, ScoreMod::Constant(360)));
    }

    #[tokio::test]
    async fn compound_simple_grows_constant_into_list() {
        // REPL probe of upstream ichiran:
        //   c1 = (adjoin-word w1 w2 :score-mod (constantly 360))
        //   c2 = (adjoin-word c1 w3 :score-mod 5)
        //   => c2 score-mod = (5 #<FUNCTION>) — Single(5) prepended
        //      onto the Constant(360).
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let c1 = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("食べたい".into()),
            Some("たべたい".into()),
            Some(ScoreMod::Constant(360)),
            None,
        )
        .await
        .unwrap();
        let w3 = KaniSimpleTextDispatchEnum::Kana(kana(2257550, "ない"));
        let c2 = adjoin_word(
            &ctx,
            KaniWordDispatchEnum::Compound(c1),
            w3,
            Some("食べたいない".into()),
            Some("たべたいない".into()),
            Some(ScoreMod::Single(5)),
            None,
        )
        .await
        .unwrap();
        match &c2.score_mod {
            ScoreMod::Stack(v) => {
                assert_eq!(v.len(), 2);
                assert!(matches!(v[0], ScoreMod::Single(5)));
                assert!(matches!(v[1], ScoreMod::Constant(360)));
            }
            _ => panic!("score-mod must be Stack([Single(5), Constant(360)])"),
        }
    }

    #[tokio::test]
    async fn compound_simple_constant_onto_constant() {
        // REPL probe of upstream ichiran:
        //   c1 = (adjoin-word w1 w2 :score-mod (constantly 200))
        //   c2 = (adjoin-word c1 w3 :score-mod (constantly 300))
        //   => c2 score-mod = (#<300> #<200>) — Constant(300) prepended
        //      onto Constant(200).
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let c1 = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("食べたい".into()),
            Some("たべたい".into()),
            Some(ScoreMod::Constant(200)),
            None,
        )
        .await
        .unwrap();
        let w3 = KaniSimpleTextDispatchEnum::Kana(kana(2257550, "ない"));
        let c2 = adjoin_word(
            &ctx,
            KaniWordDispatchEnum::Compound(c1),
            w3,
            Some("食べたいない".into()),
            Some("たべたいない".into()),
            Some(ScoreMod::Constant(300)),
            None,
        )
        .await
        .unwrap();
        match &c2.score_mod {
            ScoreMod::Stack(v) => {
                assert_eq!(v.len(), 2);
                assert!(matches!(v[0], ScoreMod::Constant(300)));
                assert!(matches!(v[1], ScoreMod::Constant(200)));
            }
            _ => panic!("score-mod must be Stack([Constant(300), Constant(200)])"),
        }
    }

    #[tokio::test]
    async fn compound_simple_third_adjoin_mixes_constants_and_integers() {
        // REPL probe of upstream ichiran:
        //   c1 = (adjoin-word w1 w2 :score-mod (constantly 360))
        //   c2 = (adjoin-word c1 w3 :score-mod 5)
        //   c3 = (adjoin-word c2 w4 :score-mod (constantly 200))
        //   => c3 score-mod = (#<200> 5 #<360>)
        let ctx = ctx_from_env().await;
        let w1 = KaniWordDispatchEnum::Kanji(kanji(10092273, "食べ"));
        let w2 = KaniSimpleTextDispatchEnum::Kana(kana(1406940, "たい"));
        let c1 = adjoin_word(
            &ctx,
            w1,
            w2,
            Some("食べたい".into()),
            Some("たべたい".into()),
            Some(ScoreMod::Constant(360)),
            None,
        )
        .await
        .unwrap();
        let w3 = KaniSimpleTextDispatchEnum::Kana(kana(2257550, "ない"));
        let c2 = adjoin_word(
            &ctx,
            KaniWordDispatchEnum::Compound(c1),
            w3,
            Some("食べたいない".into()),
            Some("たべたいない".into()),
            Some(ScoreMod::Single(5)),
            None,
        )
        .await
        .unwrap();
        let w4 = KaniSimpleTextDispatchEnum::Kana(kana(2089020, "だ"));
        let c3 = adjoin_word(
            &ctx,
            KaniWordDispatchEnum::Compound(c2),
            w4,
            Some("食べたいないだ".into()),
            Some("たべたいないだ".into()),
            Some(ScoreMod::Constant(200)),
            None,
        )
        .await
        .unwrap();
        match &c3.score_mod {
            ScoreMod::Stack(v) => {
                assert_eq!(v.len(), 3);
                assert!(matches!(v[0], ScoreMod::Constant(200)));
                assert!(matches!(v[1], ScoreMod::Single(5)));
                assert!(matches!(v[2], ScoreMod::Constant(360)));
            }
            _ => panic!("score-mod must be Stack([Constant(200), Single(5), Constant(360)])"),
        }
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod join_substring_words_inner {
use crate::conn::kani_context::KaniranContext;
use crate::dict::best_path::SCORE_CUTOFF;
use crate::dict::segment::cull_segments;
use crate::dict::calc_score::gen_score;
use crate::dict::best_path::join_substring_words_star_;
use crate::dict::segment::SegmentList;
use crate::dict::segment::Segment;

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
            let final_ = segment.end == length
                || (ends_with_lw && segment.end == length - 1);
            gen_score(ctx, &mut segment, final_, &kb).await?;
            if segment.score.expect("gen-score populates segment.score") >= SCORE_CUTOFF {
                sl.push(segment);
            }
        }
        // when sl collect (make-segment-list :segments (cull-segments sl) ...)
        if !sl.is_empty() {
            sls.push(SegmentList {
                segments: cull_segments(sl),
                start,
                end,
                top: None,
                matches,
            });
        }
    }
    Ok(sls)
}

#[cfg(test)]
mod tests {
    //! Every assertion is REPL-verified against the .103 SBCL via
    //! `(ichiran/dict::join-substring-words …)` (2026-05-23 probe runs).
    //! Run with `cargo test ... -- --test-threads=1` per the DB-test
    //! convention.
    use super::*;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
    }

    /// Per segment-list: `(start, end, matches, [scores high-to-low])`.
    /// Score values are deterministic (calc-score); order among equal
    /// scores can rotate with `find-word`'s unordered SQL, so scores are
    /// compared as a sorted-descending list.
    fn summarize(sls: &[SegmentList]) -> Vec<(usize, usize, usize, Vec<i32>)> {
        sls.iter()
            .map(|sl| {
                let mut scores: Vec<i32> =
                    sl.segments.iter().map(|seg| seg.score.unwrap()).collect();
                scores.sort_unstable_by(|a, b| b.cmp(a));
                (sl.start, sl.end, sl.matches, scores)
            })
            .collect()
    }

    /// REPL `(join-substring-words "日本語")`: 5 segment-lists.
    /// kanji-break is `(2 1)`, so `[1 2]`'s kb is the two-element
    /// reverse-order case `(1 0)`. `[0 1]` keeps 2 of its 4 matches.
    #[tokio::test]
    async fn nihongo() {
        let ctx = ctx().await;
        let sls = join_substring_words(&ctx, "日本語").await.unwrap();
        assert_eq!(
            summarize(&sls),
            vec![
                (0, 1, 4, vec![12, 8]),
                (0, 2, 1, vec![104]),
                (0, 3, 1, vec![1054]),
                (1, 2, 2, vec![8, 6]),
                (2, 3, 2, vec![18]),
            ]
        );
    }

    /// REPL `(join-substring-words "特大")`: 2 segment-lists. `[1 2]`
    /// has matches=5 but a single surviving segment after cutoff/cull.
    #[tokio::test]
    async fn tokudai() {
        let ctx = ctx().await;
        let sls = join_substring_words(&ctx, "特大").await.unwrap();
        assert_eq!(
            summarize(&sls),
            vec![(0, 2, 1, vec![208]), (1, 2, 5, vec![18])]
        );
    }

    /// REPL `(join-substring-words "私は学生です")`: 7 segment-lists.
    /// です is in *force-kanji-break*, 学生 contributes a sequential
    /// break; `[0 1]` keeps 3 of 14 私 readings, `[3 4]` keeps 3 of 7.
    #[tokio::test]
    async fn watashi_wa_gakusei_desu() {
        let ctx = ctx().await;
        let sls = join_substring_words(&ctx, "私は学生です").await.unwrap();
        assert_eq!(
            summarize(&sls),
            vec![
                (0, 1, 14, vec![25, 16, 16]),
                (1, 2, 11, vec![11]),
                (2, 3, 1, vec![8]),
                (2, 4, 2, vec![325]),
                (3, 4, 7, vec![13, 13, 8]),
                (4, 5, 4, vec![11]),
                (4, 6, 2, vec![64]),
            ]
        );
    }

    /// REPL `(join-substring-words "5本")`: the number group drives the
    /// counter path. `[0 1]` "5" is a NUMBER-TEXT scoring exactly at the
    /// cutoff (5); `[0 2]` "5本" yields COUNTER-TEXT + COUNTER-HIFUMI.
    #[tokio::test]
    async fn counter_5hon() {
        let ctx = ctx().await;
        let sls = join_substring_words(&ctx, "5本").await.unwrap();
        assert_eq!(
            summarize(&sls),
            vec![
                (0, 1, 1, vec![5]),
                (0, 2, 2, vec![128, 88]),
                (1, 2, 2, vec![16, 11]),
            ]
        );
    }

    /// REPL `(join-substring-words "ねこー")`: ends-with-lw is T. The
    /// `[0 2]` "ねこ" slice ends at length-1 (=2), so its `:final` is the
    /// `(and ends-with-lw (= end (1- length)))` branch.
    #[tokio::test]
    async fn neko_lw_final_branch() {
        let ctx = ctx().await;
        let sls = join_substring_words(&ctx, "ねこー").await.unwrap();
        assert_eq!(
            summarize(&sls),
            vec![(0, 1, 8, vec![6]), (0, 2, 1, vec![16])]
        );
    }

    /// REPL `(join-substring-words "サッカー")`: ends-with-lw is T, sole
    /// slice `[0 4]` ends at length so `:final` is T via the first
    /// disjunct. matches=3 collapses to a single kana row.
    #[tokio::test]
    async fn sakka_lw() {
        let ctx = ctx().await;
        let sls = join_substring_words(&ctx, "サッカー").await.unwrap();
        assert_eq!(summarize(&sls), vec![(0, 4, 3, vec![80])]);
    }

    /// REPL `(join-substring-words "")`: empty input → no segment-lists.
    #[tokio::test]
    async fn empty() {
        let ctx = ctx().await;
        let sls = join_substring_words(&ctx, "").await.unwrap();
        assert!(sls.is_empty());
    }

    /// Word identity at the slice level: `[0 3]` of 日本語 is the single
    /// 日本語 entry (seq 1464530), `[2 4]` of 私は学生です is 学生
    /// (seq 1270790-class kanji row). Checks the matched word text, not
    /// just the score shape.
    #[tokio::test]
    async fn slice_word_text() {
        let ctx = ctx().await;
        let mut sls = join_substring_words(&ctx, "日本語").await.unwrap();
        let whole = sls.iter_mut().find(|sl| sl.start == 0 && sl.end == 3).unwrap();
        assert_eq!(whole.segments.len(), 1);
        assert_eq!(whole.segments[0].get_text(), "日本語");
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod join_substring_words_star__inner {
use std::sync::Arc;

use crate::characters::char_classes::CharClass;
use crate::characters::text_utils::consecutive_char_groups;
use crate::characters::kanji::sequential_kanji_positions;
use crate::conn::kani_context::KaniranContext;
use crate::dict::errata::FORCE_KANJI_BREAK;
use crate::dict::find_word::MAX_WORD_LENGTH;
use crate::dict::errata::NO_KANJI_BREAK;
use crate::dict::best_path::SuffixMapTemp;
use crate::dict::segment::find_sticky_positions;
use crate::dict::find_word::find_substring_words;
use crate::dict::find_word::{find_word_full, CounterArg};
use crate::dict::grammar::suffix_lookup::get_suffix_map;
use crate::dict::segment::Segment;

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
    use crate::dict::kani::KaniWordDispatchEnum;

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
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod substring_index_inner {
use std::collections::HashMap;

use crate::conn::kani_context::KaniranContext;
use crate::dict::best_path::join_substring_words;
use crate::dict::segment::SegmentList;

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

#[cfg(test)]
mod tests {
    //! Every assertion is REPL-verified against the .103 SBCL via
    //! `(ichiran/dict::substring-index …)` (2026-05-25 probe).
    //! Run with `-- --test-threads=1` per the DB-test convention.
    use super::*;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
    }

    /// Per index entry: `(key, sl.start, sl.end, n_segments)`, sorted by
    /// key so the unordered hash compares deterministically.
    fn summarize(
        index: &HashMap<(usize, usize), SegmentList>,
    ) -> Vec<((usize, usize), usize, usize, usize)> {
        let mut rows: Vec<((usize, usize), usize, usize, usize)> = index
            .iter()
            .map(|(key, sl)| (*key, sl.start, sl.end, sl.segments.len()))
            .collect();
        rows.sort_unstable();
        rows
    }

    /// REPL `(substring-index "日本語")`: 5 entries; each value's
    /// start/end equals its key, segment counts match join-substring-words.
    #[tokio::test]
    async fn nihongo() {
        let ctx = ctx().await;
        let index = substring_index(&ctx, "日本語").await.unwrap();
        assert_eq!(
            summarize(&index),
            vec![
                ((0, 1), 0, 1, 2),
                ((0, 2), 0, 2, 1),
                ((0, 3), 0, 3, 1),
                ((1, 2), 1, 2, 2),
                ((2, 3), 2, 3, 1),
            ]
        );
    }

    /// REPL `(substring-index "特大")`: 2 entries.
    #[tokio::test]
    async fn tokudai() {
        let ctx = ctx().await;
        let index = substring_index(&ctx, "特大").await.unwrap();
        assert_eq!(
            summarize(&index),
            vec![((0, 2), 0, 2, 1), ((1, 2), 1, 2, 1)]
        );
    }

    /// REPL `(substring-index "5本")`: 3 entries; the counter slice
    /// `(0 2)` keeps 2 segments.
    #[tokio::test]
    async fn counter_5hon() {
        let ctx = ctx().await;
        let index = substring_index(&ctx, "5本").await.unwrap();
        assert_eq!(
            summarize(&index),
            vec![((0, 1), 0, 1, 1), ((0, 2), 0, 2, 2), ((1, 2), 1, 2, 2)]
        );
    }

    /// REPL `(substring-index "")`: empty input → empty index.
    #[tokio::test]
    async fn empty() {
        let ctx = ctx().await;
        let index = substring_index(&ctx, "").await.unwrap();
        assert!(index.is_empty());
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod fill_segment_path_inner {
use crate::conn::kani_context::KaniranContext;
use crate::dict::best_path::process_word_info;
use crate::dict::segment::PathElement;
use crate::dict::word_info::{WordInfo, WordInfoKana, WordInfoType};
use crate::dict::word_info::word_info_from_segment_list;

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
    use crate::dict::kani::KaniWordDispatchEnum;
    use crate::dict::segment::SegmentList;
    use crate::dict::segment::Segment;
    use crate::dict::grammar::synergy::Synergy;
    use crate::dict::word_info::WordInfoSeq;

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
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod word_info_rec_find_inner {
use crate::dict::word_info::WordInfo;

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

#[cfg(test)]
mod tests {
    //! REPL fixtures (.103, ichiran/dict:word-info-rec-find), 2026-05-25.
    //! Each case runs `word-info-rec-find` over a synthetic word-info
    //! tree (parent `P` with components `こ` / `ねこ`, sibling `S`) and
    //! a `test-fn` matching by text — the same construction probed in
    //! the REPL. Pairs are compared by `(text, next-text)`.
    use super::*;
    use crate::dict::word_info::{WordInfo, WordInfoType};

    fn wi(text: &str, components: Vec<WordInfo>) -> WordInfo {
        WordInfo {
            kind: WordInfoType::Kana,
            text: text.to_string(),
            components,
            ..Default::default()
        }
    }

    fn tree() -> Vec<WordInfo> {
        let c1 = wi("こ", Vec::new());
        let c2 = wi("ねこ", Vec::new());
        let parent = wi("P", vec![c1, c2]);
        let sibling = wi("S", Vec::new());
        vec![parent, sibling]
    }

    fn pairs<'a>(
        result: &[(&'a WordInfo, Option<&'a WordInfo>)],
    ) -> Vec<(String, Option<String>)> {
        result
            .iter()
            .map(|(car, cdr)| (car.text.clone(), cdr.map(|wi| wi.text.clone())))
            .collect()
    }

    #[test]
    fn rec_find_paths() {
        let tree = tree();
        let matches = |texts: &'static [&'static str]| {
            move |wi: &WordInfo| texts.contains(&wi.text.as_str())
        };

        // all-match: ((P . S) (こ . ねこ) (ねこ . S)) — parent emits before
        // its components; the last component's nil cdr falls back to wi-next S.
        assert_eq!(
            pairs(&word_info_rec_find(&tree, &matches(&["こ", "ねこ", "P"]))),
            vec![
                ("P".into(), Some("S".into())),
                ("こ".into(), Some("ねこ".into())),
                ("ねこ".into(), Some("S".into())),
            ]
        );

        // comp-only: parent fails the test; only the components match.
        assert_eq!(
            pairs(&word_info_rec_find(&tree, &matches(&["こ", "ねこ"]))),
            vec![
                ("こ".into(), Some("ねこ".into())),
                ("ねこ".into(), Some("S".into())),
            ]
        );

        // last-only: S is the final element → cdr is nil.
        assert_eq!(
            pairs(&word_info_rec_find(&tree, &matches(&["S"]))),
            vec![("S".into(), None)]
        );

        // parent-only: just the top-level match.
        assert_eq!(
            pairs(&word_info_rec_find(&tree, &matches(&["P"]))),
            vec![("P".into(), Some("S".into()))]
        );

        // no-match / empty list both yield nothing.
        assert!(word_info_rec_find(&tree, &|_: &WordInfo| false).is_empty());
        assert!(word_info_rec_find(&[], &|_: &WordInfo| true).is_empty());
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod process_word_info_inner {
use crate::dict::word_info::{WordInfo, WordInfoKana};
use crate::characters::kana_class::get_char_class;
use crate::characters::kana_class::KanaClass;

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
        Ba | Bi | Bu | Be | Bo
            | Pa | Pi | Pu | Pe | Po
            | Da | Dji | Dzu | De | Do
            | Za | Ji | Zu | Ze | Zo
            | Ta | Chi | Tsu | Te | To
            | Na | Nu | Ne | No
            | Ra | Ri | Ru | Re | Ro
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::word_info::WordInfoType;

    fn wi(text: &str, kana: &str) -> WordInfo {
        WordInfo {
            kind: WordInfoType::Kanji,
            text: text.to_string(),
            kana: Some(WordInfoKana::Single(kana.to_string())),
            ..Default::default()
        }
    }

    #[test]
    fn nan_branch_voiced_t() {
        let list = process_word_info(vec![wi("何", "なに"), wi("で", "で")]);
        assert_eq!(list[0].kana, Some(WordInfoKana::Single("なん".to_string())));
    }

    #[test]
    fn nani_branch_unvoiced_k() {
        let list = process_word_info(vec![wi("何", "なん"), wi("か", "か")]);
        assert_eq!(list[0].kana, Some(WordInfoKana::Single("なに".to_string())));
    }

    #[test]
    fn nani_branch_vowel() {
        let list = process_word_info(vec![wi("何", "なん"), wi("ある", "ある")]);
        assert_eq!(list[0].kana, Some(WordInfoKana::Single("なに".to_string())));
    }

    #[test]
    fn ni_treated_as_nani() {
        let list = process_word_info(vec![wi("何", "なん"), wi("人", "にん")]);
        assert_eq!(list[0].kana, Some(WordInfoKana::Single("なに".to_string())));
    }

    #[test]
    fn no_next_word_unchanged() {
        let list = process_word_info(vec![wi("何", "なん")]);
        assert_eq!(list[0].kana, Some(WordInfoKana::Single("なん".to_string())));
    }

    #[test]
    fn non_target_text_unchanged() {
        let list = process_word_info(vec![wi("猫", "ねこ"), wi("で", "で")]);
        assert_eq!(list[0].kana, Some(WordInfoKana::Single("ねこ".to_string())));
    }

    #[test]
    fn multi_kana_mixed_picks_nani() {
        let mut next_wi = wi("X", "");
        next_wi.kana = Some(WordInfoKana::Multi(vec![
            Some(WordInfoKana::Single("で".to_string())),
            Some(WordInfoKana::Single("か".to_string())),
        ]));
        let list = process_word_info(vec![wi("何", "なん"), next_wi]);
        assert_eq!(list[0].kana, Some(WordInfoKana::Single("なに".to_string())));
    }

    #[test]
    fn empty_kana_no_change() {
        let mut next_wi = wi("X", "");
        next_wi.kana = Some(WordInfoKana::Multi(Vec::new()));
        let list = process_word_info(vec![wi("何", "なん"), next_wi]);
        assert_eq!(list[0].kana, Some(WordInfoKana::Single("なん".to_string())));
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod _star_score_cutoff_star__inner {
pub const SCORE_CUTOFF: i32 = 5;
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod _star_identical_word_score_cutoff_star__inner {
pub const IDENTICAL_WORD_SCORE_CUTOFF: (i64, i64) = (1, 2);
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod _star_segment_score_cutoff_star__inner {
pub const SEGMENT_SCORE_CUTOFF: (i64, i64) = (2, 3);
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod _star_suffix_map_temp_star__inner {
use crate::dict::dao::KanaText;
use std::collections::HashMap;

pub type SuffixMapTemp = HashMap<usize, Vec<(String, String, Option<KanaText>)>>;
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod _star_suffix_next_end_star__inner {

}
