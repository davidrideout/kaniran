//! Port of `ichiran/dict:select-conjs-and-props` (`dict.lisp:1638`).
//!
//! Looks up the conjugations for `seq`, pairs each with its filtered
//! conjugation properties, and sorts by (root-vs-via, conj-type-order).

use std::cmp::Ordering;

use crate::conn::kani_context::KaniranContext;

use super::conj_prop_dao::ConjProp;
use super::conj_type_order::conj_type_order;
use super::conjugation_dao::Conjugation;
use super::filter_props::{filter_props, FilterPropsText};
use super::lex_compare::lex_compare;
use super::select_conjs::select_conjs;
use super::simple_text_class::WordConjugations;

pub async fn select_conjs_and_props(
    ctx: &KaniranContext,
    seq: i32,
    conj_ids: Option<&WordConjugations>,
    text: FilterPropsText<'_>,
) -> Result<Vec<(Conjugation, Vec<ConjProp>, [i32; 2])>, sqlx::Error> {
    let mut result: Vec<(Conjugation, Vec<ConjProp>, [i32; 2])> = Vec::new();
    // (loop for conj in (select-conjs seq conj-ids) …)
    for conj in select_conjs(ctx, seq, conj_ids).await? {
        // (select-dao 'conj-prop (:= 'conj-id (id conj)))
        let props: Vec<ConjProp> = sqlx::query_as("SELECT * FROM conj_prop WHERE conj_id = $1")
            .bind(conj.id)
            .fetch_all(&ctx.pool)
            .await?;
        // (loop for prop in props minimizing (conj-type-order (conj-type prop)))
        let val = props
            .iter()
            .map(|prop| conj_type_order(prop.conj_type))
            .min()
            .unwrap_or(0);
        // (filter-props props text)
        let fprops: Vec<ConjProp> = filter_props(&props, text).into_iter().cloned().collect();
        // (list (if (eql (seq-via conj) :null) 0 1) val)
        let key = [if conj.seq_via.is_none() { 0 } else { 1 }, val];
        result.push((conj, fprops, key));
    }
    // (sort … (lex-compare '<) :key 'third)
    let key_cmp = lex_compare(|left: &i32, right: &i32| left < right);
    result.sort_by(|left, right| {
        if key_cmp(&left.2, &right.2) {
            Ordering::Less
        } else if key_cmp(&right.2, &left.2) {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    });
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    async fn ctx_from_env() -> Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    type FpropRow = (i32, i32, i32, String, Option<bool>, Option<bool>);
    type ConjRow = (i32, i32, i32, Option<i32>, [i32; 2], Vec<FpropRow>);

    fn project(rows: &[(Conjugation, Vec<ConjProp>, [i32; 2])]) -> Vec<ConjRow> {
        rows.iter()
            .map(|(conj, fprops, key)| {
                (
                    conj.id,
                    conj.seq,
                    conj.seq_from,
                    conj.seq_via,
                    *key,
                    fprops
                        .iter()
                        .map(|p| (p.id, p.conj_id, p.conj_type, p.pos.clone(), p.neg, p.fml))
                        .collect(),
                )
            })
            .collect()
    }

    /// REPL: `(select-conjs-and-props 1156880)` → two via-null
    /// conjugations sorted by `(0 val)`. conj 661748 has prop type 13
    /// → `conj-type-order` 10 → key `(0 10)`, sorts ahead of conj
    /// 366552 (prop type 10 → `conj-type-order` 13 → key `(0 13)`),
    /// reordering the `select-conjs` input. Exercises the val swap and
    /// the sort, with nil text (all props kept).
    #[tokio::test]
    async fn via_null_sorted_by_val() {
        let ctx = ctx_from_env().await;
        let rows = select_conjs_and_props(&ctx, 1156880, None, FilterPropsText::None)
            .await
            .unwrap();
        let expected: Vec<ConjRow> = vec![
            (
                661748,
                1156880,
                1156890,
                None,
                [0, 10],
                vec![(676835, 661748, 13, "v1".to_string(), None, None)],
            ),
            (
                366552,
                1156880,
                1156870,
                None,
                [0, 13],
                vec![(374822, 366552, 10, "v5m".to_string(), Some(false), Some(false))],
            ),
        ];
        assert_eq!(project(&rows), expected);
    }

    /// REPL: `(select-conjs-and-props 1257260)` → no via-null rows, so
    /// `select-conjs` falls back to all rows; both have non-null via →
    /// key first element 1. Sorted `(1 10)` before `(1 13)`. Exercises
    /// the via-flag=1 branch and the or-fallback path.
    #[tokio::test]
    async fn via_not_null_flag_one() {
        let ctx = ctx_from_env().await;
        let rows = select_conjs_and_props(&ctx, 1257260, None, FilterPropsText::None)
            .await
            .unwrap();
        let expected: Vec<ConjRow> = vec![
            (
                1239109,
                1257260,
                1609260,
                Some(10036077),
                [1, 10],
                vec![(1254564, 1239109, 13, "v1".to_string(), None, None)],
            ),
            (
                1239126,
                1257260,
                1609260,
                Some(10036081),
                [1, 13],
                vec![(1254581, 1239126, 10, "v5s".to_string(), Some(false), Some(false))],
            ),
        ];
        assert_eq!(project(&rows), expected);
    }

    /// REPL fixtures (.103, `ichiran/dict::select-conjs-and-props`),
    /// 2026-05-24. seq 1232500 has one via-null conjugation (159588)
    /// with a passive prop (type 6, pos v1). The key stays `(0 6)` and
    /// `val` stays 6 across every text — `val` reads the *unfiltered*
    /// props — while `fprops` drops the passive prop exactly when
    /// `filter-props` would: text non-nil, not a rareru form. Covers
    /// nil / single rareru / single non-rareru / list with a rareru /
    /// list without a rareru.
    #[tokio::test]
    async fn text_threads_to_filter_props() {
        let ctx = ctx_from_env().await;
        let prop = (163127, 159588, 6, "v1".to_string(), Some(false), Some(false));
        let kept: Vec<ConjRow> =
            vec![(159588, 1232500, 2864818, None, [0, 6], vec![prop.clone()])];
        let dropped: Vec<ConjRow> = vec![(159588, 1232500, 2864818, None, [0, 6], vec![])];

        let rareru = ["食べる", "見られる"];
        let no_rareru = ["食べる", "飲む"];
        let cases: &[(FilterPropsText, &Vec<ConjRow>)] = &[
            (FilterPropsText::None, &kept),
            (FilterPropsText::One("見られる"), &kept),
            (FilterPropsText::One("食べる"), &dropped),
            (FilterPropsText::Many(&rareru), &kept),
            (FilterPropsText::Many(&no_rareru), &dropped),
        ];
        for (text, expected) in cases {
            let rows = select_conjs_and_props(&ctx, 1232500, None, *text)
                .await
                .unwrap();
            assert_eq!(&project(&rows), *expected, "text variant mismatch");
        }
    }

    /// REPL: `(select-conjs-and-props 2028980 :root)` → `NIL`
    /// (`select-conjs … :root` returns no conjugations).
    #[tokio::test]
    async fn root_conj_ids_empty() {
        let ctx = ctx_from_env().await;
        let rows = select_conjs_and_props(&ctx, 2028980, Some(&WordConjugations::Root), FilterPropsText::None)
            .await
            .unwrap();
        assert!(rows.is_empty());
    }
}
