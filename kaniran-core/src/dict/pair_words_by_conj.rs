//! Port of `ichiran/dict:pair-words-by-conj` (`dict-grammar.lisp:58`).
//!
//! Group N word-lists into buckets where words share the same
//! conjugation-source signature, placing each word at its source-list
//! index inside the bucket. The signature is the sorted list of
//! `(seq-from, via-or-0)` pairs from the word's conjugations; bucket
//! cell `[idx]` holds the word from input list `idx` (or `None`).

use std::cmp::Ordering;
use std::collections::HashMap;

use crate::conn::kani_context::KaniranContext;
use crate::dict::conjugation_dao::Conjugation;
use crate::dict::kani_word::KaniWordDispatchEnum;
use crate::dict::load::conjugate::lex_compare;
use crate::dict::simple_text_class::WordConjugations;
use crate::dict::word_conjugations::word_conjugations;

pub async fn pair_words_by_conj(
    ctx: &KaniranContext,
    word_groups: &[Vec<KaniWordDispatchEnum>],
) -> Result<Vec<Vec<Option<KaniWordDispatchEnum>>>, sqlx::Error> {
    // dict-grammar.lisp:64 — (sort … (lex-compare '<))
    let pair_cmp = lex_compare(|a: &i32, b: &i32| a < b);
    let mut bag: HashMap<Vec<i32>, Vec<Option<KaniWordDispatchEnum>>> = HashMap::new();

    // dict-grammar.lisp:65-72 — outer loop walks word-groups with idx.
    for (idx, wg) in word_groups.iter().enumerate() {
        for word in wg {
            let key = compute_key(ctx, word, &pair_cmp).await?;
            // dict-grammar.lisp:70 — (or (gethash key bag) (loop … collect nil)).
            let arr = bag
                .entry(key)
                .or_insert_with(|| vec![None; word_groups.len()]);
            // dict-grammar.lisp:71-72 — (setf (elt arr idx) word).
            arr[idx] = Some(word.clone());
        }
    }
    // dict-grammar.lisp:73 — (alexandria:hash-table-values bag).
    Ok(bag.into_values().collect())
}

async fn compute_key(
    ctx: &KaniranContext,
    word: &KaniWordDispatchEnum,
    pair_cmp: &impl Fn(&[i32], &[i32]) -> bool,
) -> Result<Vec<i32>, sqlx::Error> {
    // dict-grammar.lisp:60-63 — (mapcar (lambda (conj-id) …) (word-conjugations word)).
    // CL `(mapcar f nil)` → nil; `(mapcar f :root)` would TYPE-ERROR.
    let conj_ids: Vec<i32> = match word_conjugations(word) {
        Some(WordConjugations::Ids(ids)) => ids,
        None => Vec::new(),
        Some(WordConjugations::Root) => {
            unreachable!(
                "pair-words-by-conj received a :root-tagged word; upstream \
                 (mapcar f :root) signals a TYPE-ERROR and no producer in the \
                 call graph (find-word-with-conj-prop) sets word-conjugations \
                 to :root"
            )
        }
    };
    let mut pairs: Vec<[i32; 2]> = Vec::with_capacity(conj_ids.len());
    for cid in &conj_ids {
        // dict-grammar.lisp:61 — (get-dao 'conjugation conj-id)
        let conj: Conjugation = sqlx::query_as("SELECT * FROM conjugation WHERE id = $1")
            .bind(cid)
            .fetch_one(&ctx.pool)
            .await?;
        // dict-grammar.lisp:62 — (let ((via (seq-via conj))) (if (eql via :null) 0 via)).
        let via = conj.seq_via.unwrap_or(0);
        pairs.push([conj.seq_from, via]);
    }
    // dict-grammar.lisp:64 — sort the pair list with the lex-compare comparator.
    pairs.sort_by(|a, b| {
        if pair_cmp(a, b) {
            Ordering::Less
        } else if pair_cmp(b, a) {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    });
    // Hash key: flatten the sorted pair list. Inner length is fixed at 2,
    // so the flat sequence is a deterministic bijection of the nested form
    // and matches CL `equal` on the original list-of-2-element-lists.
    Ok(pairs.into_iter().flatten().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::kana_text_dao::KanaText;
    use crate::dict::simple_text_class::SimpleText;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    fn kana(seq: i32, text: &str, conj_ids: Vec<i32>) -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(KanaText {
            id: 0,
            seq,
            text: text.to_string(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: true,
            nokanji: false,
            best_kanji: None,
            state: SimpleText {
                conjugations: Some(WordConjugations::Ids(conj_ids)),
                hintedp: false,
            },
        })
    }

    fn seq_of(w: &KaniWordDispatchEnum) -> i32 {
        match w {
            KaniWordDispatchEnum::Kana(k) => k.seq,
            KaniWordDispatchEnum::Kanji(k) => k.seq,
            _ => panic!("test fixture only uses simple-text"),
        }
    }

    /// Sort buckets by `(idx0_seq, idx1_seq, …)` so we can deterministically
    /// compare against the REPL-captured pairing despite HashMap order.
    fn canonical(
        buckets: Vec<Vec<Option<KaniWordDispatchEnum>>>,
    ) -> Vec<Vec<Option<i32>>> {
        let mut rows: Vec<Vec<Option<i32>>> = buckets
            .into_iter()
            .map(|b| b.into_iter().map(|c| c.map(|w| seq_of(&w))).collect())
            .collect();
        rows.sort();
        rows
    }

    /// REPL: `(pair-words-by-conj)` → `NIL`. Length 0.
    #[tokio::test]
    async fn no_args_returns_empty() {
        let ctx = ctx_from_env().await;
        let result = pair_words_by_conj(&ctx, &[]).await.unwrap();
        assert!(result.is_empty());
    }

    /// REPL: `(pair-words-by-conj nil nil nil)` → `NIL`. Length 0
    /// (no words anywhere → no bucket ever created).
    #[tokio::test]
    async fn all_empty_groups_returns_empty() {
        let ctx = ctx_from_env().await;
        let result = pair_words_by_conj(&ctx, &[vec![], vec![], vec![]]).await.unwrap();
        assert!(result.is_empty());
    }

    /// REPL: `(pair-words-by-conj (find-word-with-conj-type "あった" 2))`
    /// → 3 buckets, each of length 1 holding one of the three あった
    /// readings. Each word has a distinct (seq-from, via) signature so
    /// no merging happens within the single group.
    #[tokio::test]
    async fn single_group_three_distinct_keys() {
        let ctx = ctx_from_env().await;
        // Conjugations: 87667 → from=1198180/via=NULL,
        //               227649 → from=1284430/via=NULL,
        //               475105 → from=1296400/via=NULL.
        let g1 = vec![
            kana(10087210, "あった", vec![87667]),
            kana(10226124, "あった", vec![227649]),
            kana(10470714, "あった", vec![475105]),
        ];
        let result = pair_words_by_conj(&ctx, std::slice::from_ref(&g1)).await.unwrap();
        let expected: Vec<Vec<Option<i32>>> = vec![
            vec![Some(10087210)],
            vec![Some(10226124)],
            vec![Some(10470714)],
        ];
        assert_eq!(canonical(result), expected);
    }

    /// REPL: `(pair-words-by-conj
    ///        (find-word-with-conj-type "あった" 2)
    ///        (find-word-with-conj-type "あったら" 11))`
    /// → 3 buckets pairing each あった with the matching あったら whose
    /// conj chain shares the same (seq-from, via).
    #[tokio::test]
    async fn rashii_callsite_three_pairs() {
        let ctx = ctx_from_env().await;
        let g1 = vec![
            kana(10087210, "あった", vec![87667]),     // (1198180, 0)
            kana(10226124, "あった", vec![227649]),    // (1284430, 0)
            kana(10470714, "あった", vec![475105]),    // (1296400, 0)
        ];
        let g2 = vec![
            kana(10087250, "あったら", vec![87707]),   // (1198180, 0)
            kana(10226164, "あったら", vec![227689]),  // (1284430, 0)
            kana(10470753, "あったら", vec![475145]),  // (1296400, 0)
        ];
        let result = pair_words_by_conj(&ctx, &[g1, g2]).await.unwrap();
        let expected: Vec<Vec<Option<i32>>> = vec![
            vec![Some(10087210), Some(10087250)],
            vec![Some(10226124), Some(10226164)],
            vec![Some(10470714), Some(10470753)],
        ];
        assert_eq!(canonical(result), expected);
    }

    /// REPL: same callsite as `rashii_callsite_three_pairs`, but g2
    /// is empty → 3 buckets each holding `[Some(あった), None]`.
    #[tokio::test]
    async fn second_group_empty_yields_none_slot() {
        let ctx = ctx_from_env().await;
        let g1 = vec![kana(10087210, "あった", vec![87667])];
        let g2: Vec<KaniWordDispatchEnum> = vec![];
        let result = pair_words_by_conj(&ctx, &[g1, g2]).await.unwrap();
        let expected: Vec<Vec<Option<i32>>> = vec![vec![Some(10087210), None]];
        assert_eq!(canonical(result), expected);
    }

    /// REPL: `(pair-words-by-conj nil (list w) nil)` where w has 1
    /// conjugation → 1 bucket of `[None, Some(w), None]`.
    #[tokio::test]
    async fn middle_group_only_word_padding_on_both_sides() {
        let ctx = ctx_from_env().await;
        let g2 = vec![kana(10087210, "あった", vec![87667])];
        let result = pair_words_by_conj(&ctx, &[vec![], g2, vec![]]).await.unwrap();
        let expected: Vec<Vec<Option<i32>>> = vec![vec![None, Some(10087210), None]];
        assert_eq!(canonical(result), expected);
    }

    /// REPL: 立てた (conjs=[371171, 1210719]) and 立てたら
    /// (conjs=[371207, 1210739]) both reduce to the key
    /// [(1551530,0), (1597040,1551530)] → flatten [1551530,0,1597040,1551530]
    /// → single bucket containing the pair. Exercises the multi-conjugation
    /// sort path.
    #[tokio::test]
    async fn multi_conjugation_words_share_a_bucket() {
        let ctx = ctx_from_env().await;
        let g1 = vec![kana(10368067, "立てた", vec![371171, 1210719])];
        let g2 = vec![kana(10368102, "立てたら", vec![371207, 1210739])];
        let result = pair_words_by_conj(&ctx, &[g1, g2]).await.unwrap();
        let expected: Vec<Vec<Option<i32>>> = vec![vec![Some(10368067), Some(10368102)]];
        assert_eq!(canonical(result), expected);
    }
}
