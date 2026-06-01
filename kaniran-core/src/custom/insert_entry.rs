//! Port of `ichiran/custom:insert-entry` (gf — `dict-custom.lisp:30`).

use crate::conn::kani_context::KaniranContext;
use crate::dict::load_entry::{load_entry, LoadEntryIfExists, LoadEntrySeq};

use super::as_xml::as_xml;
use super::custom_source_class::{CustomEntry, CustomLoader};

pub async fn insert_entry(
    ctx: &KaniranContext,
    _source: &CustomLoader,
    entry: &CustomEntry,
    seq: i32,
) -> Result<(), sqlx::Error> {
    // dict-custom.lisp:252 (ichiran/dict::load-entry (as-xml entry) :seq seq)
    // dict-custom.lisp:308 (ichiran/dict::load-entry (as-xml entry) :seq seq)
    let content = as_xml(entry);
    load_entry(
        ctx,
        &content,
        LoadEntryIfExists::None,
        None,
        LoadEntrySeq::Int(seq),
        false,
    )
    .await?;
    Ok(())
}
