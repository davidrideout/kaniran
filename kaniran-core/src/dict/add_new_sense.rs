//! Port of `ichiran/dict:add-new-sense` (`dict-load.lisp:91`).
//!
//! Adds a sense to the entry. Inserts the sense row, its glosses, and
//! the pos sense-props (only when they differ from the entry's last
//! seen pos). Returns `None` if a matching sense already exists.
//!
//! Multi-value return `(values sense-id ord)` collapses to `Option<(i32, i32)>`.

use crate::conn::kani_context::KaniranContext;
use crate::dict::get_senses_raw::get_senses_raw;
use crate::dict::sense_exists_p::sense_exists_p;

pub async fn add_new_sense(
    ctx: &KaniranContext,
    seq: i32,
    positions: &[String],
    glosses: &[String],
) -> Result<Option<(i32, i32)>, sqlx::Error> {
    let senses = get_senses_raw(ctx, seq).await?;
    if sense_exists_p(&senses, positions, glosses) {
        return Ok(None);
    }
    let last_sense = senses.last().expect("add_new_sense: entry has no senses");
    let ord = last_sense.ord + 1;
    // dict-load.lisp:98-101 (loop for s in (reverse senses) ... thereis pos)
    let last_pos: Option<&[String]> = senses
        .iter()
        .rev()
        .find_map(|s| {
            s.props
                .iter()
                .find(|(tag, _)| tag == "pos")
                .map(|(_, vals)| vals.as_slice())
        });
    let sense_id: i32 = sqlx::query_scalar(
        "INSERT INTO sense (seq, ord) VALUES ($1, $2) RETURNING id",
    )
    .bind(seq)
    .bind(ord)
    .fetch_one(&ctx.pool)
    .await?;
    for (gord, gloss) in glosses.iter().enumerate() {
        sqlx::query("INSERT INTO gloss (sense_id, text, ord) VALUES ($1, $2, $3)")
            .bind(sense_id)
            .bind(gloss)
            .bind(gord as i32)
            .execute(&ctx.pool)
            .await?;
    }
    let last_pos_matches = match last_pos {
        Some(lp) => lp == positions,
        None => positions.is_empty(),
    };
    if !last_pos_matches {
        for (sord, pos) in positions.iter().enumerate() {
            sqlx::query(
                "INSERT INTO sense_prop (sense_id, tag, text, ord, seq) \
                 VALUES ($1, 'pos', $2, $3, $4)",
            )
            .bind(sense_id)
            .bind(pos)
            .bind(sord as i32)
            .bind(seq)
            .execute(&ctx.pool)
            .await?;
        }
    }
    Ok(Some((sense_id, ord)))
}
