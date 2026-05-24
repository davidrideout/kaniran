//! Port of `ichiran/dict:select-conjs` (`dict.lisp:1603`).
//!
//! ```lisp
//! (defun select-conjs (seq &optional conj-ids)
//!   (if conj-ids
//!       (unless (eql conj-ids :root)
//!         (select-dao 'conjugation (:and (:= 'seq seq) (:in 'id (:set conj-ids)))))
//!       (or
//!        (select-dao 'conjugation (:and (:= 'seq seq) (:is-null 'via)))
//!        (select-dao 'conjugation (:= 'seq seq)))))
//! ```
//!
//! Diverges by taking `&KaniranContext` for the DB handle (upstream
//! `*connection*`) and modeling `conj-ids` as `Option<&WordConjugations>`
//! (`None` = nil, `Some(Root)` = `:root`, `Some(Ids)` = list).

use crate::conn::kani_context::KaniranContext;

use super::conjugation_dao::Conjugation;
use super::simple_text_class::WordConjugations;

pub async fn select_conjs(
    ctx: &KaniranContext,
    seq: i32,
    conj_ids: Option<&WordConjugations>,
) -> Result<Vec<Conjugation>, sqlx::Error> {
    match conj_ids {
        // (if conj-ids …) truthy branch
        Some(WordConjugations::Root) => Ok(Vec::new()),
        // (select-dao 'conjugation (:and (:= 'seq seq) (:in 'id (:set conj-ids))))
        Some(WordConjugations::Ids(ids)) => {
            sqlx::query_as("SELECT * FROM conjugation WHERE seq = $1 AND id = ANY($2)")
                .bind(seq)
                .bind(ids)
                .fetch_all(&ctx.pool)
                .await
        }
        // (or (select-dao … (:is-null 'via)) (select-dao … seq))
        None => {
            let with_null_via: Vec<Conjugation> =
                sqlx::query_as("SELECT * FROM conjugation WHERE seq = $1 AND via IS NULL")
                    .bind(seq)
                    .fetch_all(&ctx.pool)
                    .await?;
            if with_null_via.is_empty() {
                sqlx::query_as("SELECT * FROM conjugation WHERE seq = $1")
                    .bind(seq)
                    .fetch_all(&ctx.pool)
                    .await
            } else {
                Ok(with_null_via)
            }
        }
    }
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

    /// REPL fixtures (.103, `ichiran/dict::select-conjs`), 2026-05-24.
    /// - 2028980: single via-null conjugation (id 2343254, from 2089020) —
    ///   mirrors `tests.lisp:651`.
    /// - 1156880: via-null branch returns two rows (366552, 661748); the
    ///   seq's via-not-null row (705712) is excluded.
    /// - 1257260: no via-null rows, so the `or` falls back to all rows
    ///   (1239109, 1239126), both via-not-null.
    #[tokio::test]
    async fn select_conjs_nil_conj_ids() {
        let ctx = ctx_from_env().await;

        let r2028980 = select_conjs(&ctx, 2028980, None).await.unwrap();
        let mut ids: Vec<i32> = r2028980.iter().map(|c| c.id).collect();
        ids.sort_unstable();
        assert_eq!(ids, vec![2343254]);
        assert_eq!(r2028980[0].seq_from, 2089020);
        assert_eq!(r2028980[0].seq_via, None);

        let r1156880 = select_conjs(&ctx, 1156880, None).await.unwrap();
        let mut ids: Vec<i32> = r1156880.iter().map(|c| c.id).collect();
        ids.sort_unstable();
        assert_eq!(ids, vec![366552, 661748]);
        assert!(r1156880.iter().all(|c| c.seq_via.is_none()));

        // or-fallback: no via-null rows → all rows (both via-not-null).
        let r1257260 = select_conjs(&ctx, 1257260, None).await.unwrap();
        let mut ids: Vec<i32> = r1257260.iter().map(|c| c.id).collect();
        ids.sort_unstable();
        assert_eq!(ids, vec![1239109, 1239126]);
        assert!(r1257260.iter().all(|c| c.seq_via.is_some()));
    }

    /// REPL: `(select-conjs 2028980 :root)` → `NIL`.
    #[tokio::test]
    async fn select_conjs_root_is_empty() {
        let ctx = ctx_from_env().await;
        let result = select_conjs(&ctx, 2028980, Some(&WordConjugations::Root))
            .await
            .unwrap();
        assert!(result.is_empty());
    }

    /// REPL: `(select-conjs 1156880 (list 366552))` → only the requested
    /// id, regardless of the via-null preference (no `or` fallback). The
    /// via-not-null row (705712) is reachable through an explicit id list.
    #[tokio::test]
    async fn select_conjs_explicit_ids() {
        let ctx = ctx_from_env().await;

        let one = select_conjs(&ctx, 1156880, Some(&WordConjugations::Ids(vec![366552])))
            .await
            .unwrap();
        let ids: Vec<i32> = one.iter().map(|c| c.id).collect();
        assert_eq!(ids, vec![366552]);

        // The via-not-null row is selectable by id even though the
        // nil-conj-ids path filters it out.
        let via_row = select_conjs(&ctx, 1156880, Some(&WordConjugations::Ids(vec![705712])))
            .await
            .unwrap();
        assert_eq!(via_row.len(), 1);
        assert_eq!(via_row[0].seq_via, Some(1156890));

        // ids that don't belong to the seq are filtered by the `seq =` clause.
        let none = select_conjs(&ctx, 1156880, Some(&WordConjugations::Ids(vec![1])))
            .await
            .unwrap();
        assert!(none.is_empty());
    }
}
