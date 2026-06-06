//! Port of `ichiran/dict:find-conj` (`dict-errata.lisp:3`).
//!
//! Returns conjugation ids from `seq_from` whose `(conj-type, pos, neg,
//! fml)` quadruple matches `options`.

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
