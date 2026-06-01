//! Port of `ichiran/custom:update-entry-gloss` (gf — `dict-custom.lisp:36`).

use crate::conn::kani_context::KaniranContext;

use super::custom_source_class::{CustomEntry, CustomLoader};

pub async fn update_entry_gloss(
    ctx: &KaniranContext,
    _source: &CustomLoader,
    entry: &CustomEntry,
    seq: i32,
    gloss: &str,
) -> Result<(), sqlx::Error> {
    // dict-custom.lisp:260 (new-gloss (municipality-definition entry))
    let new_gloss = match entry {
        CustomEntry::Municipality(m) => m.definition.clone(),
        CustomEntry::Ward(_) | CustomEntry::Xml(_) => {
            panic!(
                "update-entry-gloss: no upstream method for non-municipality entry (dict-custom.lisp:36)"
            )
        }
    };
    // dict-custom.lisp:263-264 (postmodern:query (:update 'gloss :set 'text new-gloss :from 'sense :where (:and (:= 'gloss.sense-id 'sense.id) (:= 'sense.seq seq) (:= 'gloss.text gloss))))
    sqlx::query(
        "UPDATE gloss SET text = $1 FROM sense \
         WHERE gloss.sense_id = sense.id AND sense.seq = $2 AND gloss.text = $3",
    )
    .bind(&new_gloss)
    .bind(seq)
    .bind(gloss)
    .execute(&ctx.pool)
    .await?;
    Ok(())
}
