//! Port of `ichiran/dict:conjugate-da` (`dict-errata.lisp:280`).
//!
//! Ensures the entry at `seq` carries a `pos = "cop-da"` sense-prop —
//! adding it and running [`conjugate_entry_outer`] when it's missing.
//! `seq` defaults to 2089020 (the copula `だ`) when `None`.
//!
//! [`conjugate_entry_outer`]: super::conjugate_entry_outer::conjugate_entry_outer

use super::add_sense_prop::add_sense_prop;
use super::conjugate_entry_outer::conjugate_entry_outer;
use crate::conn::kani_context::KaniranContext;

pub async fn conjugate_da(
    ctx: &KaniranContext,
    seq: Option<i32>,
) -> Result<(), sqlx::Error> {
    let seq = seq.unwrap_or(2089020);
    // dict-errata.lisp:283 (unless (select-dao 'sense-prop (:and (:= 'seq seq) (:= 'tag "pos") (:= 'text "cop-da"))) …)
    let existing: Option<i32> = sqlx::query_scalar(
        "SELECT id FROM sense_prop \
         WHERE seq = $1 AND tag = 'pos' AND text = 'cop-da' LIMIT 1",
    )
    .bind(seq)
    .fetch_optional(&ctx.pool)
    .await?;
    if existing.is_some() {
        return Ok(());
    }
    // dict-errata.lisp:284 (add-sense-prop seq 0 "pos" "cop-da")
    add_sense_prop(ctx, seq, 0, "pos", "cop-da").await?;
    // dict-errata.lisp:285 (conjugate-entry-outer seq)
    conjugate_entry_outer(ctx, seq, None, None, None).await?;
    Ok(())
}
