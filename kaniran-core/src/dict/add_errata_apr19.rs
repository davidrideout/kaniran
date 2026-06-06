//! Port of `ichiran/dict:add-errata-apr19` (`dict-errata.lisp:788`).
//!
//! Applies the April-2019 batch of JMdict corrections (common-flag
//! adjustments, sense-prop tweaks, reading deletes).

use super::add_sense_prop::add_sense_prop;
use super::delete_reading::delete_reading;
use super::kani_reading_table::KaniReadingTable;
use super::set_common::set_common;
use super::set_primary_nokanji::set_primary_nokanji;
use crate::conn::kani_context::KaniranContext;

pub async fn add_errata_apr19(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    set_common(ctx, KaniReadingTable::Kanji, 1538750, "癒やす", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1538750, "癒す", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1538750, "いやす", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2147610, "いなくなる", Some(0)).await?;

    set_common(ctx, KaniReadingTable::Kana, 1346290, "マス", Some(37)).await?;
    add_sense_prop(ctx, 1346290, 3, "misc", "uk").await?;
    set_primary_nokanji(ctx, 1346290, true).await?;

    set_primary_nokanji(ctx, 1409110, false).await?;

    delete_reading(ctx, 2081610, "スレ違", None).await?;
    set_primary_nokanji(ctx, 2081610, false).await?;

    add_sense_prop(ctx, 1615340, 0, "misc", "uk").await?;
    add_sense_prop(ctx, 1658480, 0, "pos", "ctr").await?;
    Ok(())
}
