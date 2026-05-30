//! Port of `ichiran/dict:find-word-info` (`dict.lisp:1849`).
//!
//! ```lisp
//! (defun find-word-info (text &key reading root-only &aux (end (length text)))
//!   (with-connection *connection*
//!     (let* ((*suffix-map-temp* (get-suffix-map text))
//!            (*suffix-next-end* end)
//!            (all-words (if root-only
//!                           (find-word text :root-only t)
//!                           (find-word-full text :as-hiragana (test-word text :katakana) :counter :auto)))
//!            (segments (loop for word in all-words
//!                         collect (gen-score (make-segment :start 0 :end end :word word :text text))))
//!            (segments (sort segments #'> :key #'segment-score))
//!            (wis (mapcar #'word-info-from-segment segments)))
//!       (when reading
//!         (setf wis
//!               (loop for wi in wis
//!                  for seq = (word-info-seq wi)
//!                  if (equal (word-info-kana wi) reading)
//!                  collect wi
//!                  else if (and seq (exists-reading seq reading))
//!                  do (setf (word-info-kana wi) reading)
//!                  and collect wi)))
//!       wis)))
//! ```
//!
//! ## Divergences from Lisp
//!
//! - Ctx-injected (`*connection*` → `ctx.pool`); the `*suffix-map-temp*`
//!   / `*suffix-next-end*` rebinds become a sibling ctx built with
//!   [`KaniranContext::with_suffix_map_temp`] /
//!   [`KaniranContext::with_suffix_next_end`].
//! - `reading: Option<&str>` for the `&key reading`; `root_only: bool`
//!   for the `&key root-only`.
//! - `all-words` is `Vec<KaniWordDispatchEnum>`: the root-only
//!   [`find_word`] result ([`FindWordRows`]) is tagged into the same
//!   union [`find_word_full`] already returns so both feed `make-segment`.
//! - A compound's list seq reaching `(exists-reading seq reading)`
//!   reproduces upstream's PostgreSQL `integer = record` error (see
//!   [`exists_reading_seq`]).

use std::sync::Arc;

use crate::characters::char_classes::CharClass;
use crate::characters::char_classes::test_word;
use crate::conn::kani_context::KaniranContext;
use crate::dict::_star_suffix_map_temp_star_::SuffixMapTemp;
use crate::dict::exists_reading::exists_reading;
use crate::dict::find_word::{find_word, FindWordRows};
use crate::dict::find_word_full::{find_word_full, CounterArg};
use crate::dict::gen_score::gen_score;
use crate::dict::grammar::suffix_lookup::get_suffix_map;
use crate::dict::kani::KaniWordDispatchEnum;
use crate::dict::segment_struct::Segment;
use crate::dict::word_info_class::{WordInfo, WordInfoKana, WordInfoSeq};
use crate::dict::word_info_from_segment::word_info_from_segment;

pub async fn find_word_info(
    ctx: &KaniranContext,
    text: &str,
    reading: Option<&str>,
    root_only: bool,
) -> Result<Vec<WordInfo>, sqlx::Error> {
    // &aux (end (length text))
    let end = text.chars().count();

    // (let ((*suffix-map-temp* (get-suffix-map text)) (*suffix-next-end* end)) …)
    // get-suffix-map borrows ctx / text; *suffix-map-temp* owns its data,
    // so materialize owned triples once.
    let suffix_map: Arc<SuffixMapTemp> = Arc::new(
        get_suffix_map(ctx, text)
            .into_iter()
            .map(|(end_pos, items)| {
                let owned: Vec<(String, String, Option<_>)> = items
                    .into_iter()
                    .map(|(substr, key, kf)| (substr.to_string(), key.to_string(), kf.cloned()))
                    .collect();
                (end_pos, owned)
            })
            .collect(),
    );
    let ctx2 = ctx
        .with_suffix_map_temp(Some(suffix_map))
        .with_suffix_next_end(Some(end as i32));

    // (all-words (if root-only (find-word text :root-only t)
    //                (find-word-full text :as-hiragana (test-word text :katakana) :counter :auto)))
    let all_words: Vec<KaniWordDispatchEnum> = if root_only {
        match find_word(&ctx2, text, true).await? {
            FindWordRows::Kana(rows) => rows.into_iter().map(KaniWordDispatchEnum::Kana).collect(),
            FindWordRows::Kanji(rows) => rows.into_iter().map(KaniWordDispatchEnum::Kanji).collect(),
        }
    } else {
        find_word_full(
            &ctx2,
            text,
            test_word(text, CharClass::Katakana),
            Some(CounterArg::Auto),
        )
        .await?
    };

    // (segments (loop for word in all-words
    //              collect (gen-score (make-segment :start 0 :end end :word word :text text))))
    let mut segments: Vec<Segment> = Vec::with_capacity(all_words.len());
    for word in all_words {
        let mut segment = Segment {
            start: 0,
            end,
            word,
            score: None,
            info: None,
            top: None,
            text: Some(text.to_string()),
        };
        gen_score(&ctx2, &mut segment, false, &[]).await?;
        segments.push(segment);
    }

    // (segments (sort segments #'> :key #'segment-score)) — descending by score.
    segments.sort_by(|left, right| right.score.cmp(&left.score));

    // (wis (mapcar #'word-info-from-segment segments))
    let mut wis: Vec<WordInfo> = Vec::with_capacity(segments.len());
    for segment in &mut segments {
        wis.push(word_info_from_segment(&ctx2, segment).await?);
    }

    // (when reading (setf wis (loop …)))
    if let Some(reading) = reading {
        let mut filtered: Vec<WordInfo> = Vec::with_capacity(wis.len());
        for mut wi in wis {
            // for seq = (word-info-seq wi)
            let seq = wi.seq.clone();
            // if (equal (word-info-kana wi) reading) collect wi
            if matches!(&wi.kana, Some(WordInfoKana::Single(kana)) if kana == reading) {
                filtered.push(wi);
            // else if (and seq (exists-reading seq reading))
            } else if let Some(seq) = seq {
                if exists_reading_seq(&ctx2, &seq, reading).await? {
                    // do (setf (word-info-kana wi) reading) and collect wi
                    wi.kana = Some(WordInfoKana::Single(reading.to_string()));
                    filtered.push(wi);
                }
            }
        }
        wis = filtered;
    }

    Ok(wis)
}

/// `(exists-reading seq reading)` (`dict.lisp:1866`) where `seq` is the
/// `word-info-seq` slot — an int for a simple word, a list for a
/// compound. For a single seq this is the ported [`exists_reading`]
/// predicate. For a compound's list seq, postmodern renders the list as
/// a SQL row literal (`seq = (a, b, …)`), which PostgreSQL rejects with
/// `operator does not exist: integer = record` (SQLSTATE 42883);
/// reproduce the same erroring query so the failure propagates
/// identically.
async fn exists_reading_seq(
    ctx: &KaniranContext,
    seq: &WordInfoSeq,
    reading: &str,
) -> Result<bool, sqlx::Error> {
    match seq {
        WordInfoSeq::Single(single_seq) => {
            Ok(!exists_reading(ctx, *single_seq, reading).await?.is_empty())
        }
        WordInfoSeq::Multi(_) => {
            let query = format!(
                "SELECT seq FROM kana_text WHERE seq = {} AND text = $1",
                render_seq_row(seq),
            );
            let rows = sqlx::query(&query)
                .bind(reading)
                .fetch_all(&ctx.pool)
                .await?;
            Ok(!rows.is_empty())
        }
    }
}

/// Render a `word-info-seq` value the way postmodern serializes it into
/// the `(:= 'seq seq)` clause: an int as itself, a list as a
/// parenthesized comma-separated row literal (`nil` → `NULL`).
fn render_seq_row(seq: &WordInfoSeq) -> String {
    match seq {
        WordInfoSeq::Single(single_seq) => single_seq.to_string(),
        WordInfoSeq::Multi(elements) => {
            let rendered: Vec<String> = elements
                .iter()
                .map(|element| match element {
                    Some(inner) => render_seq_row(inner),
                    None => "NULL".to_string(),
                })
                .collect();
            format!("({})", rendered.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    //! Ground truth captured from `ichiran/dict:find-word-info` on .103
    //! (2026-05-25) with `(init-suffixes t t)` forced first so the suffix
    //! cache is fully populated — matching `KaniranContext::from_env`'s
    //! eager populate. Tests run against the local DB per project policy.
    use super::*;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
    }

    fn kana_of(wi: &WordInfo) -> &str {
        match &wi.kana {
            Some(WordInfoKana::Single(k)) => k,
            other => panic!("expected single kana, got {other:?}"),
        }
    }

    fn single_seq(wi: &WordInfo) -> i32 {
        match &wi.seq {
            Some(WordInfoSeq::Single(s)) => *s,
            other => panic!("expected single seq, got {other:?}"),
        }
    }

    /// One-result lookups: every populated WordInfo field per REPL.
    /// Covers the kanji branch, the katakana `:as-hiragana` branch
    /// (ヨーロッパ / コンピューター resolve to a KANA row), and
    /// ASCII-digit absence of the counter path. For all simple words
    /// `true-text` equals the surface text and find-word-info sets
    /// `start`=0 / `end`=(length text).
    #[tokio::test]
    async fn single_result_cases() {
        use crate::dict::word_info_class::WordInfoType;
        let ctx = ctx().await;
        // (text, kana, seq, score, is_kana_type)
        let cases: &[(&str, &str, i32, i32, bool)] = &[
            ("政府", "せいふ", 1376070, 325, false),
            ("経済", "けいざい", 1251320, 325, false),
            ("今日", "きょう", 1579110, 312, false),
            ("明日", "あした", 1584660, 273, false),
            ("ヨーロッパ", "ヨーロッパ", 1137570, 384, true),
            ("コンピューター", "コンピューター", 1053350, 440, true),
        ];
        for (text, kana, seq, score, is_kana) in cases {
            let result = find_word_info(&ctx, text, None, false).await.unwrap();
            assert_eq!(result.len(), 1, "text={text}");
            let wi = &result[0];
            assert_eq!(&wi.text, text, "text={text}");
            assert_eq!(kana_of(wi), *kana, "text={text}");
            assert_eq!(single_seq(wi), *seq, "text={text}");
            assert_eq!(wi.score, Some(*score), "text={text}");
            assert_eq!(
                wi.kind,
                if *is_kana { WordInfoType::Kana } else { WordInfoType::Kanji },
                "text={text}"
            );
            assert_eq!(wi.true_text.as_deref(), Some(*text), "text={text}");
            assert_eq!(wi.start, Some(0), "text={text}");
            assert_eq!(wi.end, Some(text.chars().count()), "text={text}");
            assert!(wi.counter.is_none(), "text={text}");
            assert!(wi.components.is_empty(), "text={text}");
        }
    }

    /// Multi-result lookups with distinct scores: the `(sort … #'>)`
    /// orders strictly descending. 何 (なに 24 / なん 16), 一人 (312 /
    /// 208), 二人 (325 / 208).
    #[tokio::test]
    async fn multi_result_sorted_descending() {
        let ctx = ctx().await;
        // (text, [(kana, seq, score), …] in expected order)
        let cases: &[(&str, &[(&str, i32, i32)])] = &[
            ("何", &[("なに", 1577100, 24), ("なん", 2846738, 16)]),
            ("一人", &[("ひとり", 1576150, 312), ("ひとり", 2149890, 208)]),
            ("二人", &[("ふたり", 1582670, 325), ("ふたり", 2149890, 208)]),
        ];
        for (text, expected) in cases {
            let result = find_word_info(&ctx, text, None, false).await.unwrap();
            assert_eq!(result.len(), expected.len(), "text={text}");
            for (wi, (kana, seq, score)) in result.iter().zip(expected.iter()) {
                assert_eq!(&wi.text, text, "text={text}");
                assert_eq!(kana_of(wi), *kana, "text={text}");
                assert_eq!(single_seq(wi), *seq, "text={text}");
                assert_eq!(wi.score, Some(*score), "text={text}");
            }
        }
    }

    /// 三本 → 3 results, two tied at score 208 (DB-order between the two
    /// ties is unspecified per docs/known_issues.md) then みもと 143. Assert the
    /// descending score sequence and the seq set, not the tie order.
    #[tokio::test]
    async fn score_tie_then_lower() {
        let ctx = ctx().await;
        let result = find_word_info(&ctx, "三本", None, false).await.unwrap();
        assert_eq!(result.len(), 3);
        let scores: Vec<i32> = result.iter().map(|wi| wi.score.unwrap()).collect();
        assert_eq!(scores, vec![208, 208, 143]);
        let mut seqs: Vec<i32> = result.iter().map(single_seq).collect();
        seqs.sort_unstable();
        assert_eq!(seqs, vec![1260670, 1301640, 1522150]);
        assert_eq!(single_seq(&result[2]), 1260670); // the 143 row sorts last
    }

    /// 5個 → 2 counter-text readings (ごこ 128 / ごか 40). Exercises the
    /// `:counter :auto` branch of find-word-full and a Single (source)
    /// seq on a counter word.
    #[tokio::test]
    async fn counter_auto_results() {
        let ctx = ctx().await;
        let result = find_word_info(&ctx, "5個", None, false).await.unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!((kana_of(&result[0]), single_seq(&result[0]), result[0].score), ("ごこ", 1264740, Some(128)));
        assert_eq!((kana_of(&result[1]), single_seq(&result[1]), result[1].score), ("ごか", 2220320, Some(40)));
        // counter-text: true-text nil, counter = (value-string, ordinalp), start/end 0/2.
        for wi in &result {
            assert_eq!(wi.counter, Some(("Value: 5".to_string(), false)));
            assert!(wi.true_text.is_none());
            assert_eq!(wi.start, Some(0));
            assert_eq!(wi.end, Some(2));
        }
    }

    /// `:root-only t` routes all-words through `(find-word text :root-only t)`.
    #[tokio::test]
    async fn root_only_cases() {
        let ctx = ctx().await;
        let cases: &[(&str, &str, i32, i32)] = &[
            ("経済", "けいざい", 1251320, 325),
            ("三本", "さんぼん", 1301640, 208),
            ("一人", "ひとり", 1576150, 312),
        ];
        for (text, kana, seq, score) in cases {
            let result = find_word_info(&ctx, text, None, true).await.unwrap();
            assert_eq!(result.len(), 1, "text={text}");
            assert_eq!(kana_of(&result[0]), *kana, "text={text}");
            assert_eq!(single_seq(&result[0]), *seq, "text={text}");
            assert_eq!(result[0].score, Some(*score), "text={text}");
        }
    }

    /// reading matches the word's kana (`(equal (word-info-kana wi) reading)`)
    /// → collect unchanged. Includes the compound 食べてる whose kana
    /// たべてる matches before the list-seq branch is reached.
    #[tokio::test]
    async fn reading_match_collects() {
        let ctx = ctx().await;
        let seifu = find_word_info(&ctx, "政府", Some("せいふ"), false).await.unwrap();
        assert_eq!(seifu.len(), 1);
        assert_eq!(kana_of(&seifu[0]), "せいふ");
        assert_eq!(single_seq(&seifu[0]), 1376070);

        let taberu = find_word_info(&ctx, "食べてる", Some("たべてる"), false).await.unwrap();
        assert_eq!(taberu.len(), 1);
        assert_eq!(kana_of(&taberu[0]), "たべてる");
        assert_eq!(
            taberu[0].seq,
            Some(WordInfoSeq::Multi(vec![
                Some(WordInfoSeq::Single(10092233)),
                Some(WordInfoSeq::Single(1577980)),
            ]))
        );
    }

    /// reading differs from the kana but `exists-reading` finds it for the
    /// seq → relabel the kana to reading and collect. Mismatched rows whose
    /// seq lacks the reading are dropped (一人 keeps only seq 1576150).
    #[tokio::test]
    async fn reading_relabel_and_drop() {
        let ctx = ctx().await;
        // (text, reading, expected (kana, seq, score))
        let cases: &[(&str, &str, &str, i32, i32)] = &[
            ("一人", "いちにん", "いちにん", 1576150, 312),
            ("今日", "こんにち", "こんにち", 1579110, 312),
            ("今日", "こんじつ", "こんじつ", 1579110, 312),
            ("二人", "ににん", "ににん", 1582670, 325),
            ("何", "なん", "なん", 2846738, 16),
        ];
        for (text, reading, kana, seq, score) in cases {
            let result = find_word_info(&ctx, text, Some(reading), false).await.unwrap();
            assert_eq!(result.len(), 1, "text={text} reading={reading}");
            assert_eq!(kana_of(&result[0]), *kana, "text={text} reading={reading}");
            assert_eq!(single_seq(&result[0]), *seq, "text={text} reading={reading}");
            assert_eq!(result[0].score, Some(*score), "text={text} reading={reading}");
        }
    }

    /// reading matches no row and `exists-reading` is empty for every seq
    /// → every word-info dropped, result empty.
    #[tokio::test]
    async fn reading_drops_all() {
        let ctx = ctx().await;
        assert!(find_word_info(&ctx, "政府", Some("ありえない"), false)
            .await
            .unwrap()
            .is_empty());
        assert!(find_word_info(&ctx, "何", Some("ぜんぜんちがう"), false)
            .await
            .unwrap()
            .is_empty());
    }

    /// Compounds carry a list seq and a per-part `components` list (each
    /// child a WordInfo with `primary` set iff its seq is the compound's
    /// primary). `true-text` is nil, start/end span the whole text.
    #[tokio::test]
    async fn compound_results() {
        let ctx = ctx().await;
        // (text, kana, score, [(comp_text, comp_kana, comp_seq, primary)])
        let cases: &[(&str, &str, i32, &[(&str, &str, i32, bool)])] = &[
            (
                "食べてる",
                "たべてる",
                434,
                &[
                    ("食べて", "たべて", 10092233, true),
                    ("いる", "いる", 1577980, false),
                ],
            ),
            (
                "勉強する",
                "べんきょう する",
                736,
                &[
                    ("勉強", "べんきょう", 1512670, true),
                    ("する", "する", 1157170, false),
                ],
            ),
        ];
        for (text, kana, score, comps) in cases {
            let result = find_word_info(&ctx, text, None, false).await.unwrap();
            assert_eq!(result.len(), 1, "text={text}");
            let wi = &result[0];
            assert_eq!(&wi.text, text, "text={text}");
            assert_eq!(kana_of(wi), *kana, "text={text}");
            assert_eq!(wi.score, Some(*score), "text={text}");
            assert!(wi.true_text.is_none(), "text={text}");
            assert_eq!(wi.start, Some(0), "text={text}");
            assert_eq!(wi.end, Some(text.chars().count()), "text={text}");
            let expected_seq = WordInfoSeq::Multi(
                comps.iter().map(|(_, _, s, _)| Some(WordInfoSeq::Single(*s))).collect(),
            );
            assert_eq!(wi.seq, Some(expected_seq), "text={text}");
            assert_eq!(wi.components.len(), comps.len(), "text={text}");
            for (comp, (comp_text, comp_kana, comp_seq, primary)) in
                wi.components.iter().zip(comps.iter())
            {
                assert_eq!(&comp.text, comp_text, "text={text}");
                assert_eq!(kana_of(comp), *comp_kana, "text={text}");
                assert_eq!(single_seq(comp), *comp_seq, "text={text}");
                assert_eq!(comp.primary, *primary, "text={text}");
            }
        }
    }

    /// A compound whose kana ≠ reading reaches `(exists-reading seq reading)`
    /// with a list seq; upstream errors with `integer = record` (SQLSTATE
    /// 42883), so the port returns the same DB error.
    #[tokio::test]
    async fn compound_reading_mismatch_errors() {
        let ctx = ctx().await;
        let err = find_word_info(&ctx, "食べてる", Some("ちがうよみ"), false)
            .await
            .expect_err("compound list-seq exists-reading must raise a DB error");
        match err {
            sqlx::Error::Database(db) => assert_eq!(db.code().as_deref(), Some("42883")),
            other => panic!("expected SQLSTATE 42883 database error, got {other:?}"),
        }
    }

    /// No dictionary hit → empty result.
    #[tokio::test]
    async fn no_match_is_empty() {
        let ctx = ctx().await;
        assert!(find_word_info(&ctx, "qwxz", None, false)
            .await
            .unwrap()
            .is_empty());
    }
}
