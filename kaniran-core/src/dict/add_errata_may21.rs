//! Port of `ichiran/dict:add-errata-may21` (`dict-errata.lisp:921`).
//!
//! Applies the May-2021 batch of JMdict corrections (common-flag
//! adjustments, sense-prop deletes, new readings).

use super::add_reading::add_reading;
use super::delete_sense_prop::delete_sense_prop;
use super::kani_reading_table::KaniReadingTable;
use super::set_common::set_common;
use crate::conn::kani_context::KaniranContext;

pub async fn add_errata_may21(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    add_reading(ctx, 1089590, "どんまい", None, true, None).await?;

    set_common(ctx, KaniReadingTable::Kana, 2848303, "てか", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1979920, "貴方", None).await?;

    delete_sense_prop(ctx, 1547720, "misc", "uk").await?;
    delete_sense_prop(ctx, 1495770, "misc", "uk").await?;
    delete_sense_prop(ctx, 2611890, "misc", "uk").await?;
    Ok(())
}
