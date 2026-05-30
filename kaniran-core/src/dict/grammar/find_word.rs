//! Port of the dict-grammar.lisp find-word + kana-form layer.

pub use get_kana_forms_conj_data_filter_inner::*;
pub use get_kana_forms_star__inner::*;
pub use get_kana_forms_inner::*;
pub use get_kana_form_inner::*;
pub use find_word_with_conj_prop_inner::*;
pub use find_word_with_conj_type_inner::*;
pub use pair_words_by_conj_inner::*;
pub use find_word_seq_inner::*;
pub use find_word_conj_of_inner::*;
pub use find_word_with_pos_inner::*;
pub use or_as_hiragana_inner::*;
pub use find_word_with_suffix_inner::*;

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod get_kana_forms_conj_data_filter_inner {
use crate::dict::errata::WEAK_CONJ_FORMS;
use crate::dict::conj_data::ConjData;
use crate::dict::errata::skip_by_conj_data;
use crate::dict::errata::test_conj_prop;

pub fn get_kana_forms_conj_data_filter(conj_data: &[ConjData]) -> Vec<i32> {
    if skip_by_conj_data(conj_data) {
        return Vec::new();
    }
    conj_data
        .iter()
        .filter_map(|cd| {
            let prop = cd.prop.as_ref()?;
            if test_conj_prop(prop, WEAK_CONJ_FORMS) {
                None
            } else {
                Some(prop.conj_id)
            }
        })
        .collect()
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod get_kana_forms_star__inner {
use crate::conn::kani_context::KaniranContext;
use crate::dict::conj_data::{get_conj_data, FromOrConjIds};
use super::get_kana_forms_conj_data_filter;
use crate::dict::dao::KanaText;
use crate::dict::text_classes::WordConjugations;

pub async fn get_kana_forms_star_(
    ctx: &KaniranContext,
    seq: i32,
) -> Result<Vec<KanaText>, sqlx::Error> {
    let kts: Vec<KanaText> = sqlx::query_as(
        "SELECT kt.* FROM kana_text kt WHERE kt.seq = $1 \
         UNION \
         SELECT kt.* FROM kana_text kt \
         LEFT JOIN conjugation conj ON conj.seq = kt.seq \
         WHERE conj.\"from\" = $1",
    )
    .bind(seq)
    .fetch_all(&ctx.pool)
    .await?;

    let mut out: Vec<KanaText> = Vec::with_capacity(kts.len());
    for mut kt in kts {
        if kt.seq == seq {
            kt.state.conjugations = Some(WordConjugations::Root);
            out.push(kt);
        } else {
            let cd = get_conj_data(ctx, kt.seq, FromOrConjIds::From(seq), &[]).await?;
            let conj_ids = get_kana_forms_conj_data_filter(&cd);
            if !conj_ids.is_empty() {
                kt.state.conjugations = Some(WordConjugations::Ids(conj_ids));
                out.push(kt);
            }
        }
    }
    Ok(out)
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod get_kana_forms_inner {
use crate::conn::kani_context::KaniranContext;
use super::get_kana_forms_star_;
use crate::dict::dao::KanaText;

pub async fn get_kana_forms(
    ctx: &KaniranContext,
    seq: i32,
) -> Result<Vec<KanaText>, sqlx::Error> {
    let result = get_kana_forms_star_(ctx, seq).await?;
    if result.is_empty() {
        eprintln!("kaniran: No kana forms found for: {seq}");
    }
    Ok(result)
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod get_kana_form_inner {
use crate::conn::kani_context::KaniranContext;
use crate::dict::dao::KanaText;
use crate::dict::text_classes::WordConjugations;

pub async fn get_kana_form(
    ctx: &KaniranContext,
    seq: i32,
    text: &str,
    conj: Option<WordConjugations>,
) -> Result<Option<KanaText>, sqlx::Error> {
    let row = sqlx::query_as::<_, KanaText>(
        "SELECT * FROM kana_text WHERE text = $1 AND seq = $2",
    )
    .bind(text)
    .bind(seq)
    .fetch_all(&ctx.pool)
    .await?
    .into_iter()
    .next();
    Ok(row.map(|mut r| {
        if let Some(c) = conj {
            r.state.conjugations = Some(c);
        }
        r
    }))
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod find_word_with_conj_prop_inner {
use crate::conn::kani_context::KaniranContext;
use crate::dict::conj_data::ConjData;
use crate::dict::find_word::find_word_full;
use crate::dict::kani::KaniWordDispatchEnum;
use crate::dict::text_classes::set_word_conjugations;
use crate::dict::text_classes::WordConjugations;
use crate::dict::word_info::word_conj_data;

pub async fn find_word_with_conj_prop<F>(
    ctx: &KaniranContext,
    wordstr: &str,
    mut filter_fn: F,
    allow_root: bool,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error>
where
    F: FnMut(&ConjData) -> bool,
{
    let words = find_word_full(ctx, wordstr, false, None).await?;
    let mut out: Vec<KaniWordDispatchEnum> = Vec::new();
    for mut word in words {
        // dict-grammar.lisp:45 (word-conj-data word)
        let conj_data = word_conj_data(ctx, &word).await?;
        // dict-grammar.lisp:46 (remove-if-not filter-fn conj-data)
        let filtered: Vec<&ConjData> = conj_data.iter().filter(|cd| filter_fn(cd)).collect();
        // dict-grammar.lisp:47 (mapcar (lambda (cdata) (conj-id (conj-data-prop cdata))) ...)
        let conj_ids: Vec<i32> = filtered
            .iter()
            .filter_map(|cd| cd.prop.as_ref().map(|p| p.conj_id))
            .collect();
        // dict-grammar.lisp:48 (when (or conj-data-filtered (and (null conj-data) allow-root))
        let allow_root_path = conj_data.is_empty() && allow_root;
        if !filtered.is_empty() || allow_root_path {
            // dict-grammar.lisp:49 (setf (word-conjugations word) conj-ids)
            // conj_ids may be empty (mapcar over nil = nil) — preserve
            // the upstream "nil-set" by passing None.
            let new_value = if conj_ids.is_empty() {
                None
            } else {
                Some(WordConjugations::Ids(conj_ids))
            };
            set_word_conjugations(&mut word, new_value);
            out.push(word);
        }
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

    /// REPL: `(find-word-with-conj-prop "食べて" (lambda (cd) t))` →
    /// 1 word, allow_root=nil. Filter accepts every cdata.
    #[tokio::test]
    async fn t1_all_pass_no_allow_root() {
        let ctx = ctx().await;
        let r = find_word_with_conj_prop(&ctx, "食べて", |_| true, false)
            .await
            .unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Kanji(k) = &r[0] else {
            panic!("expected KANJI-TEXT");
        };
        assert_eq!(k.seq, 10092233);
        assert_eq!(
            k.state.conjugations,
            Some(WordConjugations::Ids(vec![92707]))
        );
    }

    /// REPL: `(find-word-with-conj-prop "食べて" (lambda (cd) t)
    /// :allow-root t)` → 1 word. allow_root doesn't change the
    /// outcome when conj-data is non-empty.
    #[tokio::test]
    async fn t2_all_pass_with_allow_root() {
        let ctx = ctx().await;
        let r = find_word_with_conj_prop(&ctx, "食べて", |_| true, true)
            .await
            .unwrap();
        assert_eq!(r.len(), 1);
    }

    /// REPL: `(find-word-with-conj-prop "食べる" (lambda (cd) t)
    /// :allow-root t)` → 1 word, wc=NIL. 食べる is a root: empty
    /// conj-data + allow_root → collect with conj_ids=nil.
    #[tokio::test]
    async fn t3_root_passthrough_with_allow_root() {
        let ctx = ctx().await;
        let r = find_word_with_conj_prop(&ctx, "食べる", |_| true, true)
            .await
            .unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Kanji(k) = &r[0] else {
            panic!("expected KANJI-TEXT");
        };
        assert_eq!(k.seq, 1358280);
        // Empty mapcar over nil → setter called with nil → None.
        assert_eq!(k.state.conjugations, None);
    }

    /// REPL: `(find-word-with-conj-prop "食べる" (lambda (cd) t))` →
    /// NIL. Without allow_root, root word is filtered out.
    #[tokio::test]
    async fn t4_root_dropped_without_allow_root() {
        let ctx = ctx().await;
        let r = find_word_with_conj_prop(&ctx, "食べる", |_| true, false)
            .await
            .unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(find-word-with-conj-prop "食べて" (lambda (cd) nil))` →
    /// NIL. Filter rejects everything; without allow_root, no
    /// collection.
    #[tokio::test]
    async fn t5_reject_all() {
        let ctx = ctx().await;
        let r = find_word_with_conj_prop(&ctx, "食べて", |_| false, false)
            .await
            .unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(find-word-with-conj-prop "食べて" (lambda (cd) nil)
    /// :allow-root t)` → NIL. Filter rejects all + word has conj-data
    /// → not the empty-conj-data-allow-root branch.
    #[tokio::test]
    async fn t6_reject_all_with_allow_root_keeps_filtering() {
        let ctx = ctx().await;
        let r = find_word_with_conj_prop(&ctx, "食べて", |_| false, true)
            .await
            .unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(find-word-with-conj-prop "食べる" (lambda (cd) nil)
    /// :allow-root t)` → 1 word. Reject-all but conj-data empty +
    /// allow_root fires.
    #[tokio::test]
    async fn t7_reject_all_empty_conj_data_allow_root() {
        let ctx = ctx().await;
        let r = find_word_with_conj_prop(&ctx, "食べる", |_| false, true)
            .await
            .unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Kanji(k) = &r[0] else {
            panic!("expected KANJI-TEXT");
        };
        assert_eq!(k.seq, 1358280);
        assert_eq!(k.state.conjugations, None);
    }

    /// REPL: `(find-word-with-conj-prop "食べなくて"
    ///   (lambda (cd) (conj-neg (conj-data-prop cd))))` → 1 word
    /// (neg = T, BOOLEAN).
    /// Filter mirrors Lisp truthiness for `(conj-neg ...)`: in CL only
    /// `nil` is falsy, so both `t` and `:NULL` count. Translated to
    /// `Option<bool>` that means `p.neg != Some(false)` (None / :NULL
    /// → truthy per memory `feedback_null_nil_truthy`).
    #[tokio::test]
    async fn t8_neg_filter_matches() {
        let ctx = ctx().await;
        let r = find_word_with_conj_prop(
            &ctx,
            "食べなくて",
            |cd| cd.prop.as_ref().is_some_and(|p| p.neg != Some(false)),
            false,
        )
        .await
        .unwrap();
        assert_eq!(r.len(), 1);
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod find_word_with_conj_type_inner {
use crate::conn::kani_context::KaniranContext;
use crate::dict::grammar::find_word::find_word_with_conj_prop;
use crate::dict::kani::KaniWordDispatchEnum;

pub async fn find_word_with_conj_type(
    ctx: &KaniranContext,
    word: &str,
    conj_types: &[i32],
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    find_word_with_conj_prop(
        ctx,
        word,
        // dict-grammar.lisp:54-56 — (lambda (cdata) (member (conj-type (conj-data-prop cdata)) conj-types))
        // Lisp `(member x nil)` is nil — closing over an empty slice
        // returns false for every cdata, mirroring (member … '()).
        |cd| {
            cd.prop
                .as_ref()
                .is_some_and(|p| conj_types.contains(&p.conj_type))
        },
        false,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
    }

    /// REPL: `(find-word-with-conj-type "食べて" 3)` → 1 word
    /// text=食べて seq=10092233 wc=(92707).
    #[tokio::test]
    async fn t1_conj_type_3_matches() {
        let ctx = ctx().await;
        let r = find_word_with_conj_type(&ctx, "食べて", &[3]).await.unwrap();
        assert_eq!(r.len(), 1);
        let crate::dict::kani::KaniWordDispatchEnum::Kanji(k) = &r[0] else {
            panic!("expected KANJI-TEXT");
        };
        assert_eq!(k.seq, 10092233);
        assert_eq!(
            k.state.conjugations,
            Some(crate::dict::text_classes::WordConjugations::Ids(vec![92707]))
        );
    }

    /// REPL: `(find-word-with-conj-type "食べ" 13)` → 1 word
    /// text=食べ seq=10092273 wc=(92747). Type 13 is ren'youkei stem.
    #[tokio::test]
    async fn t2_conj_type_13() {
        let ctx = ctx().await;
        let r = find_word_with_conj_type(&ctx, "食べ", &[13]).await.unwrap();
        assert_eq!(r.len(), 1);
        let crate::dict::kani::KaniWordDispatchEnum::Kanji(k) = &r[0] else {
            panic!("expected KANJI-TEXT");
        };
        assert_eq!(k.seq, 10092273);
    }

    /// REPL: `(find-word-with-conj-type "食べる" 3)` → NIL. 食べる is a
    /// root, not a -te form.
    #[tokio::test]
    async fn t3_no_match_for_root() {
        let ctx = ctx().await;
        let r = find_word_with_conj_type(&ctx, "食べる", &[3]).await.unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(find-word-with-conj-type "食べ" 3 13)` → 1 word (type
    /// 13 hits; type 3 doesn't). Exercises the multi-conj-type set
    /// `(member x '(3 13))` arm.
    #[tokio::test]
    async fn t4_multi_conj_types() {
        let ctx = ctx().await;
        let r = find_word_with_conj_type(&ctx, "食べ", &[3, 13])
            .await
            .unwrap();
        assert_eq!(r.len(), 1);
    }

    /// REPL: `(find-word-with-conj-type "ジャバスクリプト" 3)` → NIL.
    /// Word with no conjugations.
    #[tokio::test]
    async fn t5_no_conj_data() {
        let ctx = ctx().await;
        let r = find_word_with_conj_type(&ctx, "ジャバスクリプト", &[3])
            .await
            .unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(find-word-with-conj-type "abc" 3)` → NIL. No dictionary
    /// entry at all.
    #[tokio::test]
    async fn t6_no_entry() {
        let ctx = ctx().await;
        let r = find_word_with_conj_type(&ctx, "abc", &[3]).await.unwrap();
        assert!(r.is_empty());
    }

    /// Empty `conj_types` mirrors `(find-word-with-conj-type "食べて")`
    /// — the closure `(member x nil)` is nil for every cdata; filter
    /// drops everything; allow_root=false; nothing collected.
    #[tokio::test]
    async fn t7_empty_conj_types() {
        let ctx = ctx().await;
        let r = find_word_with_conj_type(&ctx, "食べて", &[]).await.unwrap();
        assert!(r.is_empty());
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod pair_words_by_conj_inner {
use std::cmp::Ordering;
use std::collections::HashMap;

use crate::conn::kani_context::KaniranContext;
use crate::dict::dao::Conjugation;
use crate::dict::kani::KaniWordDispatchEnum;
use crate::dict::load::lex_compare;
use crate::dict::text_classes::WordConjugations;
use crate::dict::counters::dispatchers::word_conjugations;

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
    use crate::dict::dao::KanaText;
    use crate::dict::text_classes::SimpleText;

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
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod find_word_seq_inner {
use crate::characters::char_classes::CharClass;
use crate::characters::char_classes::test_word;
use crate::conn::kani_context::KaniranContext;
use crate::dict::dao::KanaText;
use crate::dict::kani::KaniWordDispatchEnum;
use crate::dict::dao::KanjiText;

#[derive(Debug, Clone)]
pub enum WordSeqRows {
    Kana(Vec<KanaText>),
    Kanji(Vec<KanjiText>),
}

impl WordSeqRows {
    /// `(car ...)` over the row vector, wrapping the row as a
    /// [`KaniWordDispatchEnum`] for downstream dispatchers. Empty
    /// vector yields `None`. Used by `*split-map*` entries that
    /// resolve a part via `(car (apply find-word-seq ...))`.
    pub(crate) fn first_word(self) -> Option<KaniWordDispatchEnum> {
        match self {
            Self::Kana(v) => v.into_iter().next().map(KaniWordDispatchEnum::Kana),
            Self::Kanji(v) => v.into_iter().next().map(KaniWordDispatchEnum::Kanji),
        }
    }

    /// First row's `seq`, used by the `("text" seq) part-seq` form
    /// of `def-simple-split` to compute the dynamic pseq via
    /// `(seq (car (find-word-conj-of "text" seq)))`. Empty vector
    /// yields `None`.
    pub(crate) fn first_seq(&self) -> Option<i32> {
        match self {
            Self::Kana(v) => v.first().map(|r| r.seq),
            Self::Kanji(v) => v.first().map(|r| r.seq),
        }
    }
}

pub async fn find_word_seq(
    ctx: &KaniranContext,
    word: &str,
    seqs: &[i32],
) -> Result<WordSeqRows, sqlx::Error> {
    if test_word(word, CharClass::Kana) {
        let rows = sqlx::query_as::<_, KanaText>(
            "SELECT * FROM kana_text WHERE text = $1 AND seq = ANY($2)",
        )
        .bind(word)
        .bind(seqs)
        .fetch_all(&ctx.pool)
        .await?;
        Ok(WordSeqRows::Kana(rows))
    } else {
        let rows = sqlx::query_as::<_, KanjiText>(
            "SELECT * FROM kanji_text WHERE text = $1 AND seq = ANY($2)",
        )
        .bind(word)
        .bind(seqs)
        .fetch_all(&ctx.pool)
        .await?;
        Ok(WordSeqRows::Kanji(rows))
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod find_word_conj_of_inner {
use crate::characters::char_classes::CharClass;
use crate::characters::char_classes::test_word;
use crate::conn::kani_context::KaniranContext;
use crate::dict::grammar::find_word::{find_word_seq, WordSeqRows};
use crate::dict::dao::KanaText;
use crate::dict::dao::KanjiText;
use std::collections::HashSet;

pub async fn find_word_conj_of(
    ctx: &KaniranContext,
    word: &str,
    seqs: &[i32],
) -> Result<WordSeqRows, sqlx::Error> {
    let primary = find_word_seq(ctx, word, seqs).await?;
    if test_word(word, CharClass::Kana) {
        let conj_rows: Vec<KanaText> = sqlx::query_as::<_, KanaText>(
            "SELECT kt.* FROM kana_text kt, conjugation conj \
             WHERE kt.seq = conj.seq AND conj.\"from\" = ANY($1) AND kt.text = $2",
        )
        .bind(seqs)
        .bind(word)
        .fetch_all(&ctx.pool)
        .await?;
        let primary_rows = match primary {
            WordSeqRows::Kana(v) => v,
            WordSeqRows::Kanji(_) => unreachable!(
                "test_word dispatch must agree between find-word-seq and find-word-conj-of"
            ),
        };
        Ok(WordSeqRows::Kana(union_by_id(primary_rows, conj_rows, |r| r.id)))
    } else {
        let conj_rows: Vec<KanjiText> = sqlx::query_as::<_, KanjiText>(
            "SELECT kt.* FROM kanji_text kt, conjugation conj \
             WHERE kt.seq = conj.seq AND conj.\"from\" = ANY($1) AND kt.text = $2",
        )
        .bind(seqs)
        .bind(word)
        .fetch_all(&ctx.pool)
        .await?;
        let primary_rows = match primary {
            WordSeqRows::Kanji(v) => v,
            WordSeqRows::Kana(_) => unreachable!(
                "test_word dispatch must agree between find-word-seq and find-word-conj-of"
            ),
        };
        Ok(WordSeqRows::Kanji(union_by_id(primary_rows, conj_rows, |r| r.id)))
    }
}

/// `(union list1 list2 :key id)` for SBCL semantics — NOT a generic
/// set union. SBCL picks the longer list (list1 wins length-tie),
/// starts a fresh result, copies the shorter list in, then walks the
/// longer list left-to-right `cons`-pushing each non-duplicate. The
/// final shape is `reverse(<longer's uniques>) ++ <shorter>`.
///
/// Empirically verified on SBCL 2.2.9:
/// - `(union '() '(1 2 3))`            → `(3 2 1)`
/// - `(union '(1 2 3) '())`            → `(3 2 1)`
/// - `(union '(1 2 3) '(4 5 6))`       → `(3 2 1 4 5 6)` (list1 wins tie)
/// - `(union '(4) '(1 2 3))`           → `(3 2 1 4)` (list2 longer)
/// - `(union '(1 2 3) '(2))`           → `(3 1 2)` (skip dup 2)
fn union_by_id<T>(list1: Vec<T>, list2: Vec<T>, id: impl Fn(&T) -> i32) -> Vec<T> {
    let (shorter, longer) = if list1.len() >= list2.len() {
        (list2, list1)
    } else {
        (list1, list2)
    };
    let shorter_keys: HashSet<i32> = shorter.iter().map(&id).collect();
    let mut uniques: Vec<T> = Vec::new();
    for elt in longer {
        if !shorter_keys.contains(&id(&elt)) {
            uniques.push(elt);
        }
    }
    uniques.reverse();
    uniques.extend(shorter);
    uniques
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod find_word_with_pos_inner {
use crate::characters::char_classes::CharClass;
use crate::characters::char_classes::test_word;
use crate::conn::kani_context::KaniranContext;
use crate::dict::dao::KanaText;
use crate::dict::dao::KanjiText;

#[derive(Debug, Clone)]
pub enum WordWithPosRows {
    Kana(Vec<KanaText>),
    Kanji(Vec<KanjiText>),
}

pub async fn find_word_with_pos(
    ctx: &KaniranContext,
    word: &str,
    posi: &[&str],
) -> Result<WordWithPosRows, sqlx::Error> {
    // s-sql `:in 'sp.text (:set posi)` expands to multiple `?` binds;
    // Postgres' `sp.text = ANY($2)` is the array-bound equivalent. The
    // sqlx Encode impl for `&[&str]` over Postgres requires owned
    // String elements, so allocate a Vec<String> for the bind (see
    // dict/get_conj_data.rs:67 for the same pattern).
    let posi_owned: Vec<String> = posi.iter().map(|s| (*s).to_string()).collect();
    if test_word(word, CharClass::Kana) {
        let rows = sqlx::query_as::<_, KanaText>(
            "SELECT DISTINCT kt.* FROM kana_text kt \
             INNER JOIN sense_prop sp ON sp.seq = kt.seq AND sp.tag = 'pos' \
             WHERE kt.text = $1 AND sp.text = ANY($2)",
        )
        .bind(word)
        .bind(&posi_owned)
        .fetch_all(&ctx.pool)
        .await?;
        Ok(WordWithPosRows::Kana(rows))
    } else {
        let rows = sqlx::query_as::<_, KanjiText>(
            "SELECT DISTINCT kt.* FROM kanji_text kt \
             INNER JOIN sense_prop sp ON sp.seq = kt.seq AND sp.tag = 'pos' \
             WHERE kt.text = $1 AND sp.text = ANY($2)",
        )
        .bind(word)
        .bind(&posi_owned)
        .fetch_all(&ctx.pool)
        .await?;
        Ok(WordWithPosRows::Kanji(rows))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Kanji input → `kanji_text` dispatch with a single matching row.
    /// REPL: `(find-word-with-pos "区別" "vs")` → 1 KANJI-TEXT row
    /// id=13731, seq=1244250, common=10, best_kana=くべつ.
    #[tokio::test]
    async fn kanji_single_match() {
        let ctx = KaniranContext::from_env().await.unwrap();
        let rows = find_word_with_pos(&ctx, "区別", &["vs"]).await.unwrap();
        let kanji = match rows {
            WordWithPosRows::Kanji(v) => v,
            WordWithPosRows::Kana(_) => panic!("expected Kanji variant"),
        };
        assert_eq!(kanji.len(), 1);
        let row = &kanji[0];
        assert_eq!(row.id, 13731);
        assert_eq!(row.seq, 1244250);
        assert_eq!(row.text, "区別");
        assert_eq!(row.ord, 0);
        assert_eq!(row.common, Some(10));
        assert_eq!(row.common_tags, "[ichi1][news1][nf10]");
        assert!(row.conjugate_p);
        assert!(!row.nokanji);
        assert_eq!(row.best_kana.as_deref(), Some("くべつ"));
    }

    /// Pure-katakana input → `test_word :kana` true → `kana_text`
    /// dispatch. REPL: `(find-word-with-pos "ジョギング" "vs")` →
    /// 1 KANA-TEXT row id=9654, seq=1066360, best_kanji = :NULL (the
    /// Lisp `:NULL` sentinel maps to Rust `None`).
    #[tokio::test]
    async fn kana_single_match() {
        let ctx = KaniranContext::from_env().await.unwrap();
        let rows = find_word_with_pos(&ctx, "ジョギング", &["vs"]).await.unwrap();
        let kana = match rows {
            WordWithPosRows::Kana(v) => v,
            WordWithPosRows::Kanji(_) => panic!("expected Kana variant"),
        };
        assert_eq!(kana.len(), 1);
        let row = &kana[0];
        assert_eq!(row.id, 9654);
        assert_eq!(row.seq, 1066360);
        assert_eq!(row.text, "ジョギング");
        assert_eq!(row.ord, 0);
        assert_eq!(row.common, Some(0));
        assert_eq!(row.common_tags, "[gai1][ichi1]");
        assert!(row.conjugate_p);
        assert!(!row.nokanji);
        assert_eq!(row.best_kanji, None);
    }

    /// Kanji word with no matching pos → empty `Kanji` result. REPL:
    /// `(find-word-with-pos "青空" "vs")` → 0 rows.
    #[tokio::test]
    async fn kanji_no_match() {
        let ctx = KaniranContext::from_env().await.unwrap();
        let rows = find_word_with_pos(&ctx, "青空", &["vs"]).await.unwrap();
        match rows {
            WordWithPosRows::Kanji(v) => assert!(v.is_empty()),
            WordWithPosRows::Kana(_) => panic!("expected Kanji variant"),
        }
    }

    /// `adj-i` pos tag. REPL: `(find-word-with-pos "赤い" "adj-i")` →
    /// 1 KANJI-TEXT row id=31416, seq=1383240.
    #[tokio::test]
    async fn kanji_adj_i_match() {
        let ctx = KaniranContext::from_env().await.unwrap();
        let rows = find_word_with_pos(&ctx, "赤い", &["adj-i"]).await.unwrap();
        let kanji = match rows {
            WordWithPosRows::Kanji(v) => v,
            WordWithPosRows::Kana(_) => panic!("expected Kanji variant"),
        };
        assert_eq!(kanji.len(), 1);
        assert_eq!(kanji[0].id, 31416);
        assert_eq!(kanji[0].seq, 1383240);
        assert_eq!(kanji[0].common, Some(15));
        assert_eq!(kanji[0].best_kana.as_deref(), Some("あかい"));
    }

    /// `adj-na` pos tag. REPL: `(find-word-with-pos "好き" "adj-na")` →
    /// 1 KANJI-TEXT row id=17991, seq=1277450.
    #[tokio::test]
    async fn kanji_adj_na_match() {
        let ctx = KaniranContext::from_env().await.unwrap();
        let rows = find_word_with_pos(&ctx, "好き", &["adj-na"]).await.unwrap();
        let kanji = match rows {
            WordWithPosRows::Kanji(v) => v,
            WordWithPosRows::Kana(_) => panic!("expected Kanji variant"),
        };
        assert_eq!(kanji.len(), 1);
        assert_eq!(kanji[0].id, 17991);
        assert_eq!(kanji[0].seq, 1277450);
        assert_eq!(kanji[0].common, Some(0));
        assert_eq!(kanji[0].best_kana.as_deref(), Some("すき"));
    }

    /// `pn` (pronoun) tag with a polysemous word → many rows. REPL:
    /// `(find-word-with-pos "私" "pn")` → 13 KANJI-TEXT rows. Pinned
    /// `(seq, id)` set captured from the REPL; row order is unspecified
    /// by the SQL (no ORDER BY upstream), so sort before comparison.
    #[tokio::test]
    async fn kanji_pn_thirteen_rows() {
        let ctx = KaniranContext::from_env().await.unwrap();
        let rows = find_word_with_pos(&ctx, "私", &["pn"]).await.unwrap();
        let kanji = match rows {
            WordWithPosRows::Kanji(v) => v,
            WordWithPosRows::Kana(_) => panic!("expected Kanji variant"),
        };
        assert_eq!(kanji.len(), 13);
        let mut got: Vec<(i32, i32)> = kanji.iter().map(|r| (r.seq, r.id)).collect();
        got.sort();
        let expected: Vec<(i32, i32)> = vec![
            (1311110, 22264),
            (1311125, 22265),
            (1347580, 26861),
            (2015370, 108229),
            (2079310, 114743),
            (2217330, 129111),
            (2217340, 129112),
            (2842390, 197077),
            (2845454, 199954),
            (2858221, 211749),
            (2858384, 211905),
            (2858397, 211916),
            (2864027, 217322),
        ];
        assert_eq!(got, expected);
        for row in &kanji {
            assert_eq!(row.text, "私");
        }
    }

    /// ASCII input → not all kana → `kanji_text` dispatch, 0 rows.
    /// REPL: `(find-word-with-pos "nonsense" "vs")` → 0 rows.
    #[tokio::test]
    async fn ascii_kanji_no_match() {
        let ctx = KaniranContext::from_env().await.unwrap();
        let rows = find_word_with_pos(&ctx, "nonsense", &["vs"]).await.unwrap();
        match rows {
            WordWithPosRows::Kanji(v) => assert!(v.is_empty()),
            WordWithPosRows::Kana(_) => panic!("expected Kanji variant"),
        }
    }

    /// Multiple posi (exercise the `&rest` arity). REPL:
    /// `(find-word-with-pos "食べる" "v1" "vs")` → 1 KANJI-TEXT row
    /// id=28271, seq=1358280 (matches the `v1` pos).
    #[tokio::test]
    async fn multi_pos_match() {
        let ctx = KaniranContext::from_env().await.unwrap();
        let rows = find_word_with_pos(&ctx, "食べる", &["v1", "vs"]).await.unwrap();
        let kanji = match rows {
            WordWithPosRows::Kanji(v) => v,
            WordWithPosRows::Kana(_) => panic!("expected Kanji variant"),
        };
        assert_eq!(kanji.len(), 1);
        assert_eq!(kanji[0].id, 28271);
        assert_eq!(kanji[0].seq, 1358280);
        assert_eq!(kanji[0].common, Some(25));
        assert_eq!(kanji[0].best_kana.as_deref(), Some("たべる"));
    }

    /// Kana word with multiple posi → `kana_text` dispatch, single row.
    /// REPL: `(find-word-with-pos "する" "vs-i" "vs-s")` →
    /// 1 KANA-TEXT row id=22268, seq=1157170.
    #[tokio::test]
    async fn kana_multi_pos_match() {
        let ctx = KaniranContext::from_env().await.unwrap();
        let rows = find_word_with_pos(&ctx, "する", &["vs-i", "vs-s"]).await.unwrap();
        let kana = match rows {
            WordWithPosRows::Kana(v) => v,
            WordWithPosRows::Kanji(_) => panic!("expected Kana variant"),
        };
        assert_eq!(kana.len(), 1);
        assert_eq!(kana[0].id, 22268);
        assert_eq!(kana[0].seq, 1157170);
        assert_eq!(kana[0].common, Some(0));
        assert_eq!(kana[0].best_kanji.as_deref(), Some("為る"));
    }

    /// Polysemous kana word with three posi — exercises both the
    /// multi-posi `ANY` and the multi-row `SELECT DISTINCT` paths.
    /// REPL: `(find-word-with-pos "そう" "adv" "n" "aux-v")` → 26
    /// KANA-TEXT rows. Pinned `(seq, id)` set; sort before comparison.
    #[tokio::test]
    async fn kana_three_pos_twentysix_rows() {
        let ctx = KaniranContext::from_env().await.unwrap();
        let rows = find_word_with_pos(&ctx, "そう", &["adv", "n", "aux-v"]).await.unwrap();
        let kana = match rows {
            WordWithPosRows::Kana(v) => v,
            WordWithPosRows::Kanji(_) => panic!("expected Kana variant"),
        };
        assert_eq!(kana.len(), 26);
        let mut got: Vec<(i32, i32)> = kana.iter().map(|r| (r.seq, r.id)).collect();
        got.sort();
        let expected: Vec<(i32, i32)> = vec![
            (1241450, 30916),
            (1398030, 47020),
            (1398670, 47082),
            (1399250, 47140),
            (1399540, 47168),
            (1399590, 47172),
            (1399990, 47213),
            (1400810, 47298),
            (2027990, 110259),
            (2033880, 110867),
            (2137720, 122367),
            (2249280, 136151),
            (2253390, 136639),
            (2406720, 153533),
            (2414580, 154361),
            (2414600, 154363),
            (2639080, 181268),
            (2681340, 185752),
            (2843362, 222959),
            (2843365, 222962),
            (2843386, 222983),
            (2843387, 222984),
            (2843388, 222985),
            (2843390, 222987),
            (2843391, 222988),
            (2844287, 224036),
        ];
        assert_eq!(got, expected);
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod or_as_hiragana_inner {
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::conn::kani_context::KaniranContext;
use crate::dict::find_word::FindWordRows;
use crate::dict::find_word::{find_word_as_hiragana, HiraganaFinder};
use crate::dict::text_classes::ProxyText;

/// Cloneable async closure called both directly and as the
/// `:finder` re-entry through [`find_word_as_hiragana`].
pub type OrAsHiraganaFinder<'a> = Arc<
    dyn Fn(String) -> Pin<Box<dyn Future<Output = Result<FindWordRows, sqlx::Error>> + Send + 'a>>
        + Send
        + Sync
        + 'a,
>;

#[derive(Debug, Clone)]
pub enum OrAsHiraganaRows {
    Direct(FindWordRows),
    AsHiragana(Vec<ProxyText>),
}

pub async fn or_as_hiragana<'a>(
    ctx: &'a KaniranContext,
    word: &str,
    fn_: OrAsHiraganaFinder<'a>,
) -> Result<Option<OrAsHiraganaRows>, sqlx::Error> {
    // dict-grammar.lisp:98 (let ((result (apply fn word args))) …)
    let result = fn_(word.to_string()).await?;
    let result_empty = match &result {
        FindWordRows::Kana(rows) => rows.is_empty(),
        FindWordRows::Kanji(rows) => rows.is_empty(),
    };
    if !result_empty {
        return Ok(Some(OrAsHiraganaRows::Direct(result)));
    }
    // dict-grammar.lisp:100 (find-word-as-hiragana word :finder (lambda (w) (apply fn w args)))
    let fn_clone = Arc::clone(&fn_);
    let finder: HiraganaFinder<'a> = Box::new(move |w| fn_clone(w));
    let proxies = find_word_as_hiragana(ctx, word, &[], Some(finder)).await?;
    if proxies.is_empty() {
        Ok(None)
    } else {
        Ok(Some(OrAsHiraganaRows::AsHiragana(proxies)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::grammar::find_word::{find_word_with_pos, WordWithPosRows};
    use crate::dict::kani::KaniSimpleTextDispatchEnum;

    // dict-grammar.lisp:506 (or-as-hiragana 'find-word-with-pos root …)
    fn make_pos_finder<'a>(
        ctx: &'a KaniranContext,
        posi: &'a [&'a str],
    ) -> OrAsHiraganaFinder<'a> {
        Arc::new(move |word: String| {
            Box::pin(async move {
                let rows = find_word_with_pos(ctx, &word, posi).await?;
                Ok(match rows {
                    WordWithPosRows::Kana(v) => FindWordRows::Kana(v),
                    WordWithPosRows::Kanji(v) => FindWordRows::Kanji(v),
                })
            })
        })
    }

    /// Path 1, kanji branch. REPL:
    /// `(or-as-hiragana 'find-word-with-pos "私" "pn")` → 13
    /// KANJI-TEXT rows (same as
    /// `(find-word-with-pos "私" "pn")` because "私" has no kana-only
    /// variant to displace it).
    #[tokio::test]
    async fn kanji_direct_pn_thirteen_rows() {
        let ctx = KaniranContext::from_env().await.unwrap();
        let posi = ["pn"];
        let finder = make_pos_finder(&ctx, &posi);
        let result = or_as_hiragana(&ctx, "私", finder).await.unwrap();
        let direct = match result {
            Some(OrAsHiraganaRows::Direct(r)) => r,
            other => panic!("expected Direct, got {:?}", other),
        };
        let kanji = match direct {
            FindWordRows::Kanji(v) => v,
            FindWordRows::Kana(_) => panic!("expected Kanji variant"),
        };
        assert_eq!(kanji.len(), 13);
        let mut got: Vec<(i32, i32)> = kanji.iter().map(|r| (r.seq, r.id)).collect();
        got.sort();
        let expected: Vec<(i32, i32)> = vec![
            (1311110, 22264),
            (1311125, 22265),
            (1347580, 26861),
            (2015370, 108229),
            (2079310, 114743),
            (2217330, 129111),
            (2217340, 129112),
            (2842390, 197077),
            (2845454, 199954),
            (2858221, 211749),
            (2858384, 211905),
            (2858397, 211916),
            (2864027, 217322),
        ];
        assert_eq!(got, expected);
    }

    /// Path 1, kana branch (katakana that has a direct katakana
    /// kana-text row → short-circuit, no fallback). REPL:
    /// `(or-as-hiragana 'find-word-with-pos "ジョギング" "vs")` →
    /// 1 KANA-TEXT row id=9654 seq=1066360.
    #[tokio::test]
    async fn katakana_direct_vs_match() {
        let ctx = KaniranContext::from_env().await.unwrap();
        let posi = ["vs"];
        let finder = make_pos_finder(&ctx, &posi);
        let result = or_as_hiragana(&ctx, "ジョギング", finder).await.unwrap();
        let direct = match result {
            Some(OrAsHiraganaRows::Direct(r)) => r,
            other => panic!("expected Direct, got {:?}", other),
        };
        let kana = match direct {
            FindWordRows::Kana(v) => v,
            FindWordRows::Kanji(_) => panic!("expected Kana variant"),
        };
        assert_eq!(kana.len(), 1);
        let row = &kana[0];
        assert_eq!(row.id, 9654);
        assert_eq!(row.seq, 1066360);
        assert_eq!(row.text, "ジョギング");
        assert_eq!(row.ord, 0);
        assert_eq!(row.common, Some(0));
        assert_eq!(row.common_tags, "[gai1][ichi1]");
        assert!(row.conjugate_p);
        assert!(!row.nokanji);
        assert_eq!(row.best_kanji, None);
    }

    /// Path 1, hiragana branch (pure hiragana → `as-hiragana` is
    /// identity → fallback can't fire; only the direct call can
    /// match). REPL: `(or-as-hiragana 'find-word-with-pos "わたし"
    /// "pn")` → 1 KANA-TEXT row id=38072 seq=1311110.
    #[tokio::test]
    async fn hiragana_direct_pn_match() {
        let ctx = KaniranContext::from_env().await.unwrap();
        let posi = ["pn"];
        let finder = make_pos_finder(&ctx, &posi);
        let result = or_as_hiragana(&ctx, "わたし", finder).await.unwrap();
        let direct = match result {
            Some(OrAsHiraganaRows::Direct(r)) => r,
            other => panic!("expected Direct, got {:?}", other),
        };
        let kana = match direct {
            FindWordRows::Kana(v) => v,
            FindWordRows::Kanji(_) => panic!("expected Kana variant"),
        };
        assert_eq!(kana.len(), 1);
        let row = &kana[0];
        assert_eq!(row.id, 38072);
        assert_eq!(row.seq, 1311110);
        assert_eq!(row.text, "わたし");
        assert_eq!(row.ord, 0);
        assert_eq!(row.common, Some(1));
        assert_eq!(row.common_tags, "[ichi1][news1][nf01]");
        assert!(row.conjugate_p);
        assert!(!row.nokanji);
        assert_eq!(row.best_kanji.as_deref(), Some("私"));
    }

    /// Path 2a — katakana input with empty direct lookup but
    /// non-empty hiragana lookup. The fallback wraps each kana-text
    /// row in a proxy-text whose `text`/`kana` carry the original
    /// katakana surface form. REPL:
    /// `(or-as-hiragana 'find-word-with-pos "アナタ" "pn")` →
    /// 2 PROXY-TEXT rows; both wrap kana-text rows for "あなた"
    /// (ids 29081 / 55771, seqs 1223615 / 1483180).
    #[tokio::test]
    async fn katakana_hiragana_fallback_two_proxies() {
        let ctx = KaniranContext::from_env().await.unwrap();
        let posi = ["pn"];
        let finder = make_pos_finder(&ctx, &posi);
        let result = or_as_hiragana(&ctx, "アナタ", finder).await.unwrap();
        let proxies = match result {
            Some(OrAsHiraganaRows::AsHiragana(p)) => p,
            other => panic!("expected AsHiragana, got {:?}", other),
        };
        assert_eq!(proxies.len(), 2);
        for proxy in &proxies {
            assert_eq!(proxy.text, "アナタ");
            assert_eq!(proxy.kana, "アナタ");
        }
        let mut sources: Vec<(i32, i32, String)> = proxies
            .iter()
            .map(|p| match p.source.as_ref() {
                KaniSimpleTextDispatchEnum::Kana(row) => (row.seq, row.id, row.text.clone()),
                KaniSimpleTextDispatchEnum::Kanji(row) => (row.seq, row.id, row.text.clone()),
                KaniSimpleTextDispatchEnum::Proxy(_) => {
                    panic!("REPL pinned source to KANA-TEXT; got nested PROXY-TEXT")
                }
            })
            .collect();
        sources.sort();
        assert_eq!(
            sources,
            vec![
                (1223615, 29081, "あなた".to_string()),
                (1483180, 55771, "あなた".to_string()),
            ]
        );
    }

    /// Path None — both direct and hiragana lookup empty. REPL:
    /// `(or-as-hiragana 'find-word-with-pos "コノ" "pn")` → NIL
    /// (no kana-text or kanji-text rows for either katakana
    /// "コノ" or its hiragana form "この" with the "pn" pos tag).
    #[tokio::test]
    async fn katakana_both_empty_yields_none() {
        let ctx = KaniranContext::from_env().await.unwrap();
        let posi = ["pn"];
        let finder = make_pos_finder(&ctx, &posi);
        let result = or_as_hiragana(&ctx, "コノ", finder).await.unwrap();
        assert!(result.is_none());
    }

    /// Path None — kanji input with no pos match; `as-hiragana`
    /// leaves kanji intact, so the fallback path also produces
    /// nothing (the str/as-hiragana equality short-circuit inside
    /// `find_word_as_hiragana` returns an empty Vec). REPL:
    /// `(or-as-hiragana 'find-word-with-pos "青空" "vs")` → NIL.
    #[tokio::test]
    async fn kanji_no_match_yields_none() {
        let ctx = KaniranContext::from_env().await.unwrap();
        let posi = ["vs"];
        let finder = make_pos_finder(&ctx, &posi);
        let result = or_as_hiragana(&ctx, "青空", finder).await.unwrap();
        assert!(result.is_none());
    }
}
}

#[allow(clippy::module_inception, dead_code, unused_imports)]
mod find_word_with_suffix_inner {
use crate::conn::kani_context::KaniranContext;
use crate::dict::grammar::suffix_init::suffix_class;
use crate::dict::find_word::find_word_full;
use crate::dict::kani::KaniWordDispatchEnum;
use crate::dict::counters::dispatchers::seq;
use crate::dict::word_info::WordInfoSeq;

pub async fn find_word_with_suffix(
    ctx: &KaniranContext,
    wordstr: &str,
    suffix_classes: &[&str],
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    let words = find_word_full(ctx, wordstr, false, None).await?;
    let class_map = suffix_class(ctx);
    let mut out: Vec<KaniWordDispatchEnum> = Vec::new();
    for word in words {
        // dict-grammar.lisp:104 (seq word)
        let word_seq = seq(&word);
        // dict-grammar.lisp:105 (and (listp seq) (gethash (car (last seq)) *suffix-class*))
        let class: Option<&String> = match word_seq {
            Some(WordInfoSeq::Multi(elems)) => match elems.last() {
                Some(Some(WordInfoSeq::Single(i))) => class_map.get(i),
                // nested compound or nil last element — hash lookup misses
                _ => None,
            },
            _ => None,
        };
        // dict-grammar.lisp:106 (when (and suffix-class (find suffix-class suffix-classes)) collect word)
        if let Some(cls) = class {
            if suffix_classes.contains(&cls.as_str()) {
                out.push(word);
            }
        }
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

    /// REPL: `(find-word-with-suffix "我々ら" :ra)` → 1 compound
    /// text=我々ら kana=われわれら.
    #[tokio::test]
    async fn t1_warera_ra_match() {
        let ctx = ctx().await;
        let r = find_word_with_suffix(&ctx, "我々ら", &["ra"]).await.unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Compound(c) = &r[0] else {
            panic!("expected COMPOUND");
        };
        assert_eq!(c.text, "我々ら");
        assert_eq!(c.kana, "われわれら");
    }

    /// REPL: `(find-word-with-suffix "勉強する" :suru)` → 1 compound
    /// text=勉強する kana=べんきょう する.
    #[tokio::test]
    async fn t2_benkyousuru_suru_match() {
        let ctx = ctx().await;
        let r = find_word_with_suffix(&ctx, "勉強する", &["suru"])
            .await
            .unwrap();
        assert_eq!(r.len(), 1);
        let KaniWordDispatchEnum::Compound(c) = &r[0] else {
            panic!("expected COMPOUND");
        };
        assert_eq!(c.text, "勉強する");
        assert_eq!(c.kana, "べんきょう する");
    }

    /// REPL: `(find-word-with-suffix "勉強する" :ra)` → NIL (class
    /// mismatch).
    #[tokio::test]
    async fn t3_wrong_class_drops() {
        let ctx = ctx().await;
        let r = find_word_with_suffix(&ctx, "勉強する", &["ra"]).await.unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(find-word-with-suffix "区別" :suru)` → NIL. Simple-text
    /// `seq` is an integer (not listp) — class lookup skipped.
    #[tokio::test]
    async fn t4_simple_text_seq_not_listp() {
        let ctx = ctx().await;
        let r = find_word_with_suffix(&ctx, "区別", &["suru"]).await.unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(find-word-with-suffix "abc" :suru)` → NIL.
    #[tokio::test]
    async fn t5_no_entries() {
        let ctx = ctx().await;
        let r = find_word_with_suffix(&ctx, "abc", &["suru"]).await.unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(find-word-with-suffix "勉強する")` → NIL. Empty
    /// suffix-classes — `(find x nil)` is always nil → no
    /// collection.
    #[tokio::test]
    async fn t6_empty_classes() {
        let ctx = ctx().await;
        let r = find_word_with_suffix(&ctx, "勉強する", &[]).await.unwrap();
        assert!(r.is_empty());
    }

    /// REPL: `(find-word-with-suffix "勉強する" :ra :suru)` → 1
    /// compound (suru matches, ra doesn't). Multi-class set.
    #[tokio::test]
    async fn t7_multi_class_set() {
        let ctx = ctx().await;
        let r = find_word_with_suffix(&ctx, "勉強する", &["ra", "suru"])
            .await
            .unwrap();
        assert_eq!(r.len(), 1);
    }
}
}
