//! Port of `ichiran/kanji:kanji-word-stats` (`kanji.lisp:316`).
//!
//! For every kanji-bearing common word that contains `char`,
//! aligns the kanji writing with its kana reading via
//! [`super::match_readings`] and tallies how many words ascribe each
//! `(reading, type)` pair to `char`. Words whose alignment falls
//! through to an `"irr"` reading (or fails to align altogether) are
//! counted into the `irregular` tally. Returns the tally alist,
//! the irregular count, and the total word count.

use super::get_original_reading::get_original_reading;
use super::get_reading_alternatives::ReadingTag;
use super::match_readings::{match_readings, MatchedSegment};
use crate::conn::kani_context::KaniranContext;
use crate::dict::get_kanji_words::get_kanji_words;
use std::collections::HashMap;

pub async fn kanji_word_stats(
    ctx: &KaniranContext,
    char: &str,
) -> Result<(Vec<((String, String), i32)>, i32, usize), sqlx::Error> {
    let str = char;
    let words = get_kanji_words(ctx, str).await?;
    let mut r_stat: HashMap<(String, String), i32> = HashMap::new();
    let mut irregular: i32 = 0;

    // kanji.lisp:321-329 (loop for (seq k r common) in words …)
    for (_seq, k, r, _common) in &words {
        let mr = match_readings(ctx, k, r).await?;
        // kanji.lisp:322 (assoc str (remove-if-not 'listp (match-readings k r)) :test 'equal)
        let reading = mr.as_deref().and_then(|segs| {
            segs.iter().find(|seg| match seg {
                MatchedSegment::Kanji { kanji, .. } => kanji == str,
                MatchedSegment::NonKanji(_) => false,
            })
        });
        match reading {
            Some(MatchedSegment::Kanji { reading: kr, .. }) => {
                // kanji.lisp:324-328 (destructuring-bind (rtext rtype &rest options) (cdr reading) …)
                let rtext = &kr.reading;
                let rtype = &kr.r#type;
                if rtype == "irr" {
                    irregular += 1;
                } else {
                    let rendaku = matches!(kr.tag, Some(ReadingTag::Rendaku));
                    let geminated = kr.gem.as_deref();
                    let key_text = get_original_reading(rtext, rendaku, geminated);
                    let key = (key_text, rtype.clone());
                    *r_stat.entry(key).or_insert(0) += 1;
                }
            }
            _ => {
                // kanji.lisp:329 (else do (incf irregular))
                irregular += 1;
            }
        }
    }
    // kanji.lisp:330 (values (alexandria:hash-table-alist r-stat) irregular (length words))
    let stats: Vec<((String, String), i32)> = r_stat.into_iter().collect();
    Ok((stats, irregular, words.len()))
}

#[cfg(test)]
mod tests {
    //! Fixtures captured via `(ichiran/kanji:kanji-word-stats …)` on
    //! the .103 SBCL REPL (2026-06-01). The upstream returns an
    //! `alexandria:hash-table-alist`, so each assertion sorts both
    //! sides by key before comparing — iteration order is hash-defined
    //! and not part of the contract.
    use super::*;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
    }

    fn entry(rtext: &str, rtype: &str, count: i32) -> ((String, String), i32) {
        ((rtext.to_string(), rtype.to_string()), count)
    }

    #[tokio::test]
    async fn kanji_word_stats_fixtures() {
        let ctx = ctx().await;
        let cases: &[(&str, Vec<((String, String), i32)>, i32, usize)] = &[
            (
                "山",
                vec![entry("さん", "ja_on", 53), entry("やま", "ja_kun", 49)],
                2,
                104,
            ),
            (
                "水",
                vec![entry("すい", "ja_on", 136), entry("みず", "ja_kun", 48)],
                1,
                185,
            ),
            (
                "火",
                vec![
                    entry("び", "ja_kun", 10),
                    entry("ひ", "ja_kun", 17),
                    entry("か", "ja_on", 45),
                ],
                3,
                75,
            ),
            (
                "心",
                vec![
                    entry("こころ", "ja_kun", 25),
                    entry("ごころ", "ja_kun", 8),
                    entry("しん", "ja_on", 101),
                ],
                6,
                140,
            ),
            (
                "学",
                vec![entry("まな", "ja_kun", 2), entry("がく", "ja_on", 214)],
                0,
                216,
            ),
            (
                "国",
                vec![entry("くに", "ja_kun", 10), entry("こく", "ja_on", 195)],
                1,
                206,
            ),
            ("電", vec![entry("でん", "ja_on", 90)], 0, 90),
            (
                "鯨",
                vec![entry("げい", "ja_on", 2), entry("くじら", "ja_kun", 1)],
                0,
                3,
            ),
        ];
        for (kanji, expected_stats, expected_irregular, expected_total) in cases {
            let (mut stats, irregular, total) =
                kanji_word_stats(&ctx, kanji).await.unwrap();
            stats.sort();
            let mut want = expected_stats.clone();
            want.sort();
            assert_eq!(stats, want, "kanji={kanji:?} stats");
            assert_eq!(irregular, *expected_irregular, "kanji={kanji:?} irregular");
            assert_eq!(total, *expected_total, "kanji={kanji:?} total");
        }
    }
}
