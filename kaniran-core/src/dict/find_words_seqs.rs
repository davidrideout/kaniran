//! Transliteration of `ichiran/dict:find-words-seqs` (`dict.lisp:520`).
//!
//! ```lisp
//! (defun find-words-seqs (words seqs)
//!   "generalized version of find-word-seq from dict-grammar"
//!   (unless (listp words) (setf words (list words)))
//!   (unless (listp seqs) (setf seqs (list seqs)))
//!   (loop for word in words
//!      if (test-word word :kana) collect word into kana-words
//!      else collect word into kanji-words
//!      finally
//!        (let ((kw (when kanji-words (select-dao 'kanji-text (:and (:in 'text (:set kanji-words)) (:in 'seq (:set seqs))))))
//!              (rw (when kana-words (select-dao 'kana-text (:and (:in 'text (:set kana-words)) (:in 'seq (:set seqs)))))))
//!          (return (nconc kw rw)))))
//! ```
//!
//! Partitions `words` into kana vs kanji by [`test_word`], fetches the
//! `kanji_text` rows whose text is among the kanji words and the
//! `kana_text` rows whose text is among the kana words (both restricted to
//! `seqs`), and returns the kanji rows followed by the kana rows.
//!
//! Diverges from the upstream lambda list `(words seqs)` by:
//! - taking `&KaniranContext` for the DB handle, replacing Lisp's
//!   `*connection*`;
//! - taking `words: &[&str]` and `seqs: &[i32]` — the Lisp coerces a lone
//!   word / seq to a one-element list internally, the Rust caller wraps;
//! - returning `Vec<KaniWordDispatchEnum>`, tagging each heterogeneous
//!   `(nconc kw rw)` element as its kanji-text / kana-text variant.

use crate::characters::char_classes::CharClass;
use crate::characters::test_word::test_word;
use crate::conn::kani_context::KaniranContext;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kani_word::KaniWordDispatchEnum;
use crate::dict::kanji_text_dao::KanjiText;

pub async fn find_words_seqs(
    ctx: &KaniranContext,
    words: &[&str],
    seqs: &[i32],
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    let mut kana_words: Vec<&str> = Vec::new();
    let mut kanji_words: Vec<&str> = Vec::new();
    for &word in words {
        if test_word(word, CharClass::Kana) {
            kana_words.push(word);
        } else {
            kanji_words.push(word);
        }
    }

    let mut out: Vec<KaniWordDispatchEnum> = Vec::new();
    // dict.lisp:532 (when kanji-words (select-dao 'kanji-text ...))
    if !kanji_words.is_empty() {
        let kw: Vec<KanjiText> = sqlx::query_as::<_, KanjiText>(
            "SELECT * FROM kanji_text WHERE text = ANY($1) AND seq = ANY($2)",
        )
        .bind(kanji_words.as_slice())
        .bind(seqs)
        .fetch_all(&ctx.pool)
        .await?;
        out.extend(kw.into_iter().map(KaniWordDispatchEnum::Kanji));
    }
    // dict.lisp:533 (when kana-words (select-dao 'kana-text ...))
    if !kana_words.is_empty() {
        let rw: Vec<KanaText> = sqlx::query_as::<_, KanaText>(
            "SELECT * FROM kana_text WHERE text = ANY($1) AND seq = ANY($2)",
        )
        .bind(kana_words.as_slice())
        .bind(seqs)
        .fetch_all(&ctx.pool)
        .await?;
        out.extend(rw.into_iter().map(KaniWordDispatchEnum::Kana));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
    }

    fn describe(word: &KaniWordDispatchEnum) -> (&'static str, i32, &str) {
        match word {
            KaniWordDispatchEnum::Kanji(k) => ("kanji", k.seq, k.text.as_str()),
            KaniWordDispatchEnum::Kana(k) => ("kana", k.seq, k.text.as_str()),
            _ => panic!("find_words_seqs must only return kanji-text / kana-text"),
        }
    }

    /// REPL (.103, `ichiran/dict::find-words-seqs`), 2026-05-24. Each case
    /// returns one row: a kanji word fills `kw` (kana-words empty), a kana
    /// word fills `rw` (kanji-words empty).
    #[tokio::test]
    async fn single_row_fixtures() {
        let ctx = ctx().await;
        let cases: &[(&[&str], &[i32], (&str, i32, &str))] = &[
            (&["食べる"], &[1358280], ("kanji", 1358280, "食べる")),
            (&["たべる"], &[1358280], ("kana", 1358280, "たべる")),
            (&["見る"], &[1259290], ("kanji", 1259290, "見る")),
        ];
        for (words, seqs, expected) in cases {
            let result = find_words_seqs(&ctx, words, seqs).await.unwrap();
            assert_eq!(result.len(), 1, "words={words:?}");
            assert_eq!(describe(&result[0]), *expected, "words={words:?}");
        }
    }

    /// REPL: `(find-words-seqs "みる" '(1213770 1259290 1365450 1772790
    /// 2255060 10553286))` → 6 KANA-TEXT rows, one per matching seq.
    #[tokio::test]
    async fn kana_multi_seq() {
        let ctx = ctx().await;
        let seqs = [1213770, 1259290, 1365450, 1772790, 2255060, 10553286];
        let result = find_words_seqs(&ctx, &["みる"], &seqs).await.unwrap();
        assert_eq!(result.len(), 6);
        let mut got: Vec<i32> = result
            .iter()
            .map(|word| {
                let (kind, seq, text) = describe(word);
                assert_eq!((kind, text), ("kana", "みる"));
                seq
            })
            .collect();
        got.sort_unstable();
        assert_eq!(got, seqs);
    }

    /// REPL: `(find-words-seqs '("食べる" "たべる") '(1358280))` → KANJI-TEXT
    /// 食べる then KANA-TEXT たべる. Exercises `(nconc kw rw)` with both
    /// partitions non-empty.
    #[tokio::test]
    async fn mixed_two_words() {
        let ctx = ctx().await;
        let result = find_words_seqs(&ctx, &["食べる", "たべる"], &[1358280])
            .await
            .unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(describe(&result[0]), ("kanji", 1358280, "食べる"));
        assert_eq!(describe(&result[1]), ("kana", 1358280, "たべる"));
    }

    /// REPL: `(find-words-seqs '("見る" "みる" "食べる") '(1259290 1358280))`
    /// → KANJI 見る, KANJI 食べる, KANA みる. `(nconc kw rw)` guarantees all
    /// kanji rows precede all kana rows; intra-partition order is the DB's.
    #[tokio::test]
    async fn mixed_partition() {
        let ctx = ctx().await;
        let result = find_words_seqs(&ctx, &["見る", "みる", "食べる"], &[1259290, 1358280])
            .await
            .unwrap();
        assert_eq!(result.len(), 3);
        let kana_start = result
            .iter()
            .position(|word| matches!(word, KaniWordDispatchEnum::Kana(_)))
            .unwrap();
        assert!(result[..kana_start]
            .iter()
            .all(|word| matches!(word, KaniWordDispatchEnum::Kanji(_))));
        assert!(result[kana_start..]
            .iter()
            .all(|word| matches!(word, KaniWordDispatchEnum::Kana(_))));
        let mut got: Vec<(&str, i32, &str)> = result.iter().map(describe).collect();
        got.sort_unstable();
        let mut expected = vec![
            ("kanji", 1259290, "見る"),
            ("kanji", 1358280, "食べる"),
            ("kana", 1259290, "みる"),
        ];
        expected.sort_unstable();
        assert_eq!(got, expected);
    }

    /// REPL: `(find-words-seqs "食べる" 9999999)` → NIL. Word matches a
    /// row but no row carries the seq, so the `seq = ANY` filter empties it.
    #[tokio::test]
    async fn no_match_seq() {
        let ctx = ctx().await;
        let result = find_words_seqs(&ctx, &["食べる"], &[9999999]).await.unwrap();
        assert!(result.is_empty());
    }

    /// Empty `words` leaves both `kanji-words` and `kana-words` nil, so the
    /// two `(when ...)` guards skip every query and `(nconc nil nil)` is nil.
    #[tokio::test]
    async fn empty_words() {
        let ctx = ctx().await;
        let result = find_words_seqs(&ctx, &[], &[1358280]).await.unwrap();
        assert!(result.is_empty());
    }
}
