//! Port of `ichiran/custom:insert` (gf — `dict-custom.lisp:14`).

use crate::conn::kani_context::KaniranContext;
use crate::dict::next_seq::next_seq;

use super::custom_source_class::CustomLoader;
use super::insert_entry::insert_entry;
use super::test_entry::{test_entry, TestEntryResult};
use super::update_entry::update_entry;
use super::update_entry_gloss::update_entry_gloss;

pub async fn insert(ctx: &KaniranContext, loader: &CustomLoader) -> Result<(), sqlx::Error> {
    match loader {
        // dict-custom.lisp:74 (defmethod insert ((loader xml-loader)) ...)
        CustomLoader::Xml(x) => x.insert(ctx).await,
        _ => insert_default(ctx, loader).await,
    }
}

async fn insert_default(
    ctx: &KaniranContext,
    loader: &CustomLoader,
) -> Result<(), sqlx::Error> {
    // dict-custom.lisp:39-51 (defmethod insert (source) (with-connection *connection* (loop with cur-seq = (ichiran/dict::next-seq) for entry in (entries source) do (multiple-value-bind (ok seq) (test-entry source entry) (when ok (cond ((consp seq) (apply 'update-entry-gloss source entry seq)) (seq (update-entry source entry seq)) (t (insert-entry source entry cur-seq) (incf cur-seq))))))))
    let mut cur_seq = next_seq(ctx).await?;
    for entry in &loader.base().entries {
        match test_entry(ctx, loader, entry).await? {
            TestEntryResult::UpdateGloss(seq, gloss) => {
                update_entry_gloss(ctx, loader, entry, seq, &gloss).await?;
            }
            TestEntryResult::Update(seq) => {
                update_entry(ctx, loader, entry, seq).await?;
            }
            TestEntryResult::Insert => {
                insert_entry(ctx, loader, entry, cur_seq).await?;
                cur_seq += 1;
            }
            TestEntryResult::Skip => {}
        }
    }
    Ok(())
}
