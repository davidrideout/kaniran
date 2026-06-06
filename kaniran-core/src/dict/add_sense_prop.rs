//! Port of `ichiran/dict:add-sense-prop` (`dict-errata.lisp:142`).
//!
//! Looks up the sense at `(seq, sense-ord)` and, when present, inserts
//! one `sense-prop` row `(tag, text)` unless the same `(sense-id, tag,
//! text)` triple already exists.

use crate::conn::kani_context::KaniranContext;

pub async fn add_sense_prop(
    ctx: &KaniranContext,
    seq: i32,
    sense_ord: i32,
    tag: &str,
    text: &str,
) -> Result<(), sqlx::Error> {
    // dict-errata.lisp:143 (car (select-dao 'sense (:and (:= 'seq seq) (:= 'ord sense-ord))))
    let sense_id: Option<i32> = sqlx::query_scalar(
        "SELECT id FROM sense WHERE seq = $1 AND ord = $2 LIMIT 1",
    )
    .bind(seq)
    .bind(sense_ord)
    .fetch_optional(&ctx.pool)
    .await?;
    let Some(sense_id) = sense_id else {
        return Ok(());
    };
    // dict-errata.lisp:145 (unless (select-dao 'sense-prop …))
    let existing: Option<i32> = sqlx::query_scalar(
        "SELECT id FROM sense_prop WHERE sense_id = $1 AND tag = $2 AND text = $3 LIMIT 1",
    )
    .bind(sense_id)
    .bind(tag)
    .bind(text)
    .fetch_optional(&ctx.pool)
    .await?;
    if existing.is_some() {
        return Ok(());
    }
    // dict-errata.lisp:146 (make-dao 'sense-prop :sense-id … :tag tag :text text :ord 0 :seq seq)
    sqlx::query(
        "INSERT INTO sense_prop (sense_id, tag, text, ord, seq) \
         VALUES ($1, $2, $3, 0, $4)",
    )
    .bind(sense_id)
    .bind(tag)
    .bind(text)
    .bind(seq)
    .execute(&ctx.pool)
    .await?;
    Ok(())
}
