//! Port of `ichiran/dict:insert-sense-traits` (`dict-load.lisp:66`).
//!
//! For every descendant of `sense_node` whose element name matches
//! `tag`, INSERTs a `sense_prop` row carrying that descendant's text
//! content. Used by [`crate::dict::insert_senses`].

use crate::conn::kani_context::KaniranContext;
use crate::dict::node_text::node_text;
use roxmltree::Node;

pub async fn insert_sense_traits(
    ctx: &KaniranContext,
    sense_node: Node<'_, '_>,
    tag: &str,
    sense_id: i32,
    seq: i32,
) -> Result<(), sqlx::Error> {
    for (ord, node) in sense_node
        .descendants()
        .filter(|n| *n != sense_node && n.is_element() && n.has_tag_name(tag))
        .enumerate()
    {
        let text = node_text(node, None);
        sqlx::query(
            "INSERT INTO sense_prop (tag, sense_id, text, ord, seq) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(tag)
        .bind(sense_id)
        .bind(&text)
        .bind(ord as i32)
        .bind(seq)
        .execute(&ctx.pool)
        .await?;
    }
    Ok(())
}
