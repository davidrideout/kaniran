//! Port of `ichiran/dict:find-conj` (`dict-errata.lisp:3`).
//!
//! Returns conjugation ids whose `(conj-type, pos, neg, fml)` quadruple
//! matches `options`. Used by [`super::add_conj::add_conj`] to skip
//! re-creating an existing conjugation.
//!
//! Diverges from the upstream lambda list `(seq-from options)` only by
//! taking `&KaniranContext` for the database handle, replacing the
//! upstream dynamic `*connection*` per [`crate::conn::kani_context`].
//! `options` is the 4-tuple `(conj-type, pos, neg, fml)`; the upstream
//! `:===` null-safe equality on `neg` / `fml` maps to `IS NOT DISTINCT
//! FROM`.

use crate::conn::kani_context::KaniranContext;

pub async fn find_conj(
    ctx: &KaniranContext,
    seq_from: i32,
    options: (i32, &str, Option<bool>, Option<bool>),
) -> Result<Vec<i32>, sqlx::Error> {
    let (conj_type, pos, neg, fml) = options;
    let rows: Vec<(i32,)> = sqlx::query_as(
        r#"SELECT conj.id
           FROM conjugation AS conj
           INNER JOIN conj_prop AS prop ON prop.conj_id = conj.id
           WHERE conj."from" = $1
             AND prop.conj_type = $2
             AND prop.pos = $3
             AND prop.neg IS NOT DISTINCT FROM $4
             AND prop.fml IS NOT DISTINCT FROM $5"#,
    )
    .bind(seq_from)
    .bind(conj_type)
    .bind(pos)
    .bind(neg)
    .bind(fml)
    .fetch_all(&ctx.pool)
    .await?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}
