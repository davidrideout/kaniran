//! Port of `ichiran/dict:find-word-full` (`dict.lisp:1052`).
//!
//! ```lisp
//! (defun find-word-full (word &key as-hiragana counter)
//!   (let ((simple-words (find-word word)))
//!     (nconc simple-words
//!            (find-word-suffix word :matches simple-words)
//!            (when as-hiragana
//!              (find-word-as-hiragana word :exclude (mapcar 'seq simple-words)))
//!            (when counter
//!              (case counter
//!                (:auto
//!                 (let ((groups (consecutive-char-groups :number word)))
//!                   (when groups
//!                     (find-counter (subseq word (caar groups) (cdar groups))
//!                                   (subseq word (cdar groups) (length word))))))
//!                (t (let ((number (subseq word 0 counter))
//!                         (counter (subseq word counter (length word))))
//!                     (find-counter number counter :unique (not simple-words)))))))))
//! ```
//!
//! Composes [`find_word`], [`find_word_suffix`],
//! [`find_word_as_hiragana`], and [`find_counter`] into a single
//! heterogeneous result list — the segmenter / scoring layer's main
//! entry point for "give me every reading I can think of for this
//! substring". Order is load-bearing (the upstream `nconc` preserves
//! the simple-words first, suffix expansions second, hiragana proxy
//! third, counter candidates last).
//!
//! ## Divergences from Lisp
//!
//! - **Ctx-injected** per CONVENTIONS §4.8.
//! - **`as_hiragana: bool`** per CONVENTIONS §4.4. Caller-readable
//!   at the callsite (`true` ↔ `:as-hiragana t`).
//! - **`counter: Option<CounterArg>`** per CONVENTIONS §4.3 (closed
//!   tagged shape: the upstream `:auto` keyword vs. integer index vs.
//!   absent). [`CounterArg::Auto`] mirrors `:auto`; [`CounterArg::At`]
//!   carries the character index where the number ends and the
//!   counter unit begins.
//! - **Return type `Vec<KaniWordDispatchEnum>`.** The four
//!   sub-results — simple-text rows, suffix-expansion compounds,
//!   hiragana proxies, counter-text candidates — are all wrapped
//!   into the top-level enum so callers iterate uniformly.
//!
//! ## Counter argument
//!
//! `CounterArg::At(n)` interprets `n` as a **character** position per
//! CONVENTIONS §4.5 — `word[0..n]` is the number text,
//! `word[n..word_len]` the counter. `:unique` is passed as `Some(!
//! simple_words.is_empty()` ↔ Lisp `:unique (not simple-words)`).
//! `CounterArg::Auto` finds the first run of `:number` characters
//! via [`consecutive_char_groups`] and uses the first group's
//! `(start, end)` to slice the number; the counter is whatever
//! follows up to the end of `word`.
//!
//! [`find_word`]: super::find_word::find_word
//! [`find_word_suffix`]: super::find_word_suffix::find_word_suffix
//! [`find_word_as_hiragana`]: super::find_word_as_hiragana::find_word_as_hiragana
//! [`find_counter`]: super::find_counter::find_counter
//! [`consecutive_char_groups`]: crate::characters::text_utils::consecutive_char_groups

use crate::characters::char_classes::CharClass;
use crate::characters::text_utils::consecutive_char_groups;
use crate::conn::kani_context::KaniranContext;
use crate::dict::find_counter::find_counter;
use crate::dict::find_word::{find_word, FindWordRows};
use crate::dict::find_word_as_hiragana::find_word_as_hiragana;
use crate::dict::find_word_suffix::find_word_suffix;
use crate::dict::kani::KaniWordDispatchEnum;
use crate::dict::subseq_slice::subseq_slice;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::compound_text_class::ScoreMod;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
    }

    /// REPL: `(find-word-full "区別")` → 1 KANJI-TEXT (seq=1244250).
    /// Single simple-text, no suffix / hiragana / counter branches.
    #[tokio::test]
    async fn t1_simple_kanji_word() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "区別", false, None).await.unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Kanji(k) = &r[0] else {
            panic!("expected KANJI-TEXT");
        };
        assert_eq!(k.seq, 1244250);
        assert_eq!(k.text, "区別");
    }

    /// REPL: `(find-word-full "私")` → 14 KANJI-TEXT rows (polysemous
    /// 私). Exercises multi-row simple-words.
    #[tokio::test]
    async fn t2_polysemous_kanji() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "私", false, None).await.unwrap();
        assert_eq!(r.len(), 14);
        for w in &r {
            assert!(matches!(w, KaniWordDispatchEnum::Kanji(_)));
        }
    }

    /// REPL: `(find-word-full "勉強する")` → 1 COMPOUND-TEXT.
    /// simple-words for 勉強する is empty; suffix-suru fires through
    /// the partial `*suffix-list*` (suru row registered) and produces
    /// 1 compound (勉強+する).
    #[tokio::test]
    async fn t3_suru_suffix_via_registered_row() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "勉強する", false, None).await.unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Compound(c) = &r[0] else {
            panic!("expected COMPOUND-TEXT");
        };
        assert_eq!(c.text, "勉強する");
        assert_eq!(c.kana, "べんきょう する");
    }

    /// REPL: `(find-word-full "我々ら")` → 1 COMPOUND-TEXT via the
    /// `ra` suffix row.
    #[tokio::test]
    async fn t4_ra_suffix_via_registered_row() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "我々ら", false, None).await.unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Compound(c) = &r[0] else {
            panic!("expected COMPOUND-TEXT");
        };
        assert_eq!(c.text, "我々ら");
        assert_eq!(c.kana, "われわれら");
    }

    /// REPL: `(find-word-full "食べてる")` → 1 COMPOUND-TEXT
    /// text="食べてる" kana="たべてる" via `suffix-teiru`. primary =
    /// KANJI-TEXT 食べて (seq=10092233), words = (primary, KANA-TEXT
    /// いる seq=1577980), score_mod=3, score_base=nil.
    #[tokio::test]
    async fn t5_teiru_suffix_compound() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "食べてる", false, None).await.unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Compound(c) = &r[0] else {
            panic!("expected Compound, got {:?}", r[0]);
        };
        assert_eq!(c.text, "食べてる");
        assert_eq!(c.kana, "たべてる");
        assert!(matches!(c.score_mod, ScoreMod::Single(3)));
        assert!(c.score_base.is_none());
        let KaniWordDispatchEnum::Kanji(primary) = &*c.primary else {
            panic!("expected Kanji primary, got {:?}", c.primary);
        };
        assert_eq!(primary.seq, 10092233);
        assert_eq!(primary.text, "食べて");
        assert_eq!(c.words.len(), 2);
        let KaniWordDispatchEnum::Kanji(w0) = &c.words[0] else {
            panic!("expected Kanji words[0]");
        };
        assert_eq!(w0.seq, 10092233);
        let KaniWordDispatchEnum::Kana(w1) = &c.words[1] else {
            panic!("expected Kana words[1]");
        };
        assert_eq!(w1.seq, 1577980);
        assert_eq!(w1.text, "いる");
    }

    /// REPL: `(find-word-full "xyzabc")` → NIL. No simple-text, no
    /// suffix expansion via the cache.
    #[tokio::test]
    async fn t6_no_match() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "xyzabc", false, None).await.unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(find-word-full "ジャバスクリプト" :as-hiragana t)` → 1
    /// (the existing kana_text row 2302400; the hiragana fallback
    /// excludes the same seq, so no proxies added).
    #[tokio::test]
    async fn t7_as_hiragana_with_existing_kana_match() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "ジャバスクリプト", true, None).await.unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Kana(k) = &r[0] else {
            panic!("expected KANA-TEXT");
        };
        assert_eq!(k.seq, 2302400);
    }

    /// REPL: `(find-word-full "ハイ" :as-hiragana t)` → 14:
    ///   1 KANA-TEXT (the existing ハイ row) + 13 PROXY-TEXT (the
    ///   13 はい kana_text root rows wrapped as proxies).
    #[tokio::test]
    async fn t8_as_hiragana_with_proxy_fallback() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "ハイ", true, None).await.unwrap();
        assert_eq!(r.len(), 14);
        let kana_count = r
            .iter()
            .filter(|w| matches!(w, KaniWordDispatchEnum::Kana(_)))
            .count();
        let proxy_count = r
            .iter()
            .filter(|w| matches!(w, KaniWordDispatchEnum::Proxy(_)))
            .count();
        assert_eq!(kana_count, 1);
        assert_eq!(proxy_count, 13);
    }

    /// REPL: `(find-word-full "三本" :counter :auto)` → 3:
    ///   1 KANJI-TEXT (existing 三本) + 1 COUNTER-TEXT + 1
    ///   COUNTER-HIFUMI. Exercises the `:auto` branch through
    ///   `consecutive-char-groups`.
    #[tokio::test]
    async fn t9_counter_auto_with_simple_match() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "三本", false, Some(CounterArg::Auto))
            .await
            .unwrap();
        assert_eq!(r.len(), 3);
        assert!(matches!(r[0], KaniWordDispatchEnum::Kanji(_)));
        assert!(matches!(r[1], KaniWordDispatchEnum::Counter(_)));
        assert!(matches!(r[2], KaniWordDispatchEnum::Counter(_)));
    }

    /// REPL: `(find-word-full "5本" :counter 1)` → 2 COUNTER-TEXT
    /// (number text "5", counter unit "本"). Integer-index branch;
    /// simple-words is empty so `:unique` resolves to T.
    #[tokio::test]
    async fn t10_counter_explicit_index() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "5本", false, Some(CounterArg::At(1)))
            .await
            .unwrap();
        assert_eq!(r.len(), 2);
        for w in &r {
            assert!(matches!(w, KaniWordDispatchEnum::Counter(_)));
        }
    }

    /// REPL: `(find-word-full "区別" :counter :auto)` → 1 (just the
    /// kanji-text; `consecutive-char-groups :number` returns NIL for
    /// 区別 → counter branch contributes nothing).
    #[tokio::test]
    async fn t11_counter_auto_no_number_group() {
        let ctx = ctx().await;
        let r = find_word_full(&ctx, "区別", false, Some(CounterArg::Auto))
            .await
            .unwrap();
        assert_eq!(r.len(), 1);
        assert!(matches!(r[0], KaniWordDispatchEnum::Kanji(_)));
    }

    /// REPL: `(find-word-full <long>)` → 0 (full result, not just the
    /// `find-word` branch). The `*max-word-length*` gate inside
    /// `find-word` short-circuits the simple-words path; the
    /// `find-word-suffix` branch still runs but finds no cache hit on
    /// this specific 51-char hiragana run — REPL-verified against
    /// both the random-hiragana string below and a realistic
    /// over-length sentence.
    #[tokio::test]
    async fn t12_over_length_short_circuit() {
        let ctx = ctx().await;
        let long = "あいうえおかきくけこさしすせそたちつてとなにぬねのはひふへほまみむめもやゆよらりるれろわをんがぎぐげござ";
        let r = find_word_full(&ctx, long, false, None).await.unwrap();
        assert!(r.is_empty());
    }
}
