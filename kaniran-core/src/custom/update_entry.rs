//! Port of `ichiran/custom:update-entry` (gf — `dict-custom.lisp:33`).

use crate::conn::kani_context::KaniranContext;
use crate::dict::add_new_sense::add_new_sense;

use super::custom_source_class::{CustomEntry, CustomLoader};

pub async fn update_entry(
    ctx: &KaniranContext,
    _source: &CustomLoader,
    entry: &CustomEntry,
    seq: i32,
) -> Result<(), sqlx::Error> {
    // dict-custom.lisp:257 (ichiran/dict::add-new-sense seq '("n") (list (municipality-definition entry)))
    // dict-custom.lisp:313 (ichiran/dict::add-new-sense seq '("n") (list (ward-definition entry)))
    let definition = match entry {
        CustomEntry::Municipality(m) => m.definition.clone(),
        CustomEntry::Ward(w) => w.definition.clone(),
        CustomEntry::Xml(_) => {
            panic!("update-entry: no upstream method for xml-entry (dict-custom.lisp:33)")
        }
    };
    add_new_sense(ctx, seq, &["n".to_string()], &[definition]).await?;
    Ok(())
}
