//! Port of `ichiran/dict:delete-senses` (`dict-errata.lisp:131`).
//!
//! Drops every sense (and its glosses and remaining props) whose
//! sense-props on `seq` include any that satisfy `prop_test`. Deletes
//! all rows linked to the matched `sense-id`s, not just the matched
//! props themselves.

use super::sense_prop_dao::SenseProp;
use crate::conn::kani_context::KaniranContext;

pub async fn delete_senses(
    ctx: &KaniranContext,
    seq: i32,
    prop_test: impl Fn(&SenseProp) -> bool,
) -> Result<(), sqlx::Error> {
    let all_props: Vec<SenseProp> =
        sqlx::query_as("SELECT * FROM sense_prop WHERE seq = $1")
            .bind(seq)
            .fetch_all(&ctx.pool)
            .await?;
    let sense_props: Vec<&SenseProp> =
        all_props.iter().filter(|p| prop_test(p)).collect();
    let mut sense_ids: Vec<i32> = Vec::new();
    for prop in &sense_props {
        if !sense_ids.contains(&prop.sense_id) {
            sense_ids.push(prop.sense_id);
        }
    }
    sqlx::query("DELETE FROM sense_prop WHERE sense_id = ANY($1)")
        .bind(&sense_ids)
        .execute(&ctx.pool)
        .await?;
    sqlx::query("DELETE FROM gloss WHERE sense_id = ANY($1)")
        .bind(&sense_ids)
        .execute(&ctx.pool)
        .await?;
    sqlx::query("DELETE FROM sense WHERE id = ANY($1)")
        .bind(&sense_ids)
        .execute(&ctx.pool)
        .await?;
    Ok(())
}
