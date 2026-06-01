//! Port of `ichiran/dict:insert-senses` (`dict-load.lisp:71`).
//!
//! For each element of `node_list` (the JMdict `<sense>` nodes of one
//! entry), INSERTs a `sense` row, then INSERTs each child `<gloss>` as
//! a `gloss` row.

use crate::conn::kani_context::KaniranContext;
use crate::dict::insert_sense_traits::insert_sense_traits;
use crate::dict::node_text::node_text;
use roxmltree::Node;

const SENSE_PROP_TAGS: &[&str] = &[
    "pos", "misc", "dial", "field", "s_inf", "stagk", "stagr",
];

pub async fn insert_senses(
    ctx: &KaniranContext,
    node_list: &[Node<'_, '_>],
    seq: i32,
) -> Result<(), sqlx::Error> {
    for (ord, node) in node_list.iter().enumerate() {
        let sense_id: i32 = sqlx::query_scalar(
            "INSERT INTO sense (seq, ord) VALUES ($1, $2) RETURNING id",
        )
        .bind(seq)
        .bind(ord as i32)
        .fetch_one(&ctx.pool)
        .await?;
        for (gord, gloss_node) in node
            .descendants()
            .filter(|n| *n != *node && n.is_element() && n.has_tag_name("gloss"))
            .enumerate()
        {
            let text = node_text(gloss_node, None);
            sqlx::query(
                "INSERT INTO gloss (sense_id, text, ord) VALUES ($1, $2, $3)",
            )
            .bind(sense_id)
            .bind(&text)
            .bind(gord as i32)
            .execute(&ctx.pool)
            .await?;
        }
        for tag in SENSE_PROP_TAGS {
            insert_sense_traits(ctx, *node, tag, sense_id, seq).await?;
        }
    }
    Ok(())
}
