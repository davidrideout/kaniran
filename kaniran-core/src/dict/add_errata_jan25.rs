//! Port of `ichiran/dict:add-errata-jan25` (`dict-errata.lisp:986`).
//!
//! Applies the January-2025 batch of JMdict corrections (common-flag
//! adjustments, sense-prop tweaks, reading replacements/deletes).

use super::add_sense_prop::add_sense_prop;
use super::delete_reading::delete_reading;
use super::delete_sense_prop::delete_sense_prop;
use super::kani_reading_table::KaniReadingTable;
use super::replace_reading::replace_reading;
use super::replace_reading_conj::replace_reading_conj;
use super::set_common::set_common;
use crate::conn::kani_context::KaniranContext;

pub async fn add_errata_jan25(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    delete_reading(ctx, 2028930, "ヶ", Some(KaniReadingTable::Kana)).await?;
    delete_reading(ctx, 2028930, "ケ", Some(KaniReadingTable::Kana)).await?;

    delete_sense_prop(ctx, 1138570, "pos", "ctr").await?;
    add_sense_prop(ctx, 1138570, 1, "pos", "ctr").await?;
    add_sense_prop(ctx, 1138570, 2, "pos", "ctr").await?;
    add_sense_prop(ctx, 1138570, 3, "pos", "ctr").await?;

    set_common(ctx, KaniReadingTable::Kana, 1001120, "うんち", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1511600, "かたかな", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1056400, "サウンドトラック", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1510640, "へん", Some(5)).await?;

    replace_reading(
        ctx,
        2860664,
        "こどもはおやのせなかをみてそだう",
        "こどもはおやのせなかをみてそだつ",
    )
    .await?;
    replace_reading_conj(
        ctx,
        2863544,
        KaniReadingTable::Kana,
        "みぎにでるのは",
        "みぎにでるものは",
    )
    .await?;
    Ok(())
}
