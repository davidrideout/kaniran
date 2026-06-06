//! Port of `ichiran/dict:add-errata-dec23` (`dict-errata.lisp:954`).
//!
//! Applies the December-2023 batch of JMdict corrections (common-flag
//! adjustments, sense-prop add/delete pairs).

use super::add_sense_prop::add_sense_prop;
use super::delete_sense_prop::delete_sense_prop;
use super::kani_reading_table::KaniReadingTable;
use super::set_common::set_common;
use crate::conn::kani_context::KaniranContext;

pub async fn add_errata_dec23(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    add_sense_prop(ctx, 1180540, 0, "misc", "uk").await?;
    delete_sense_prop(ctx, 2854117, "misc", "uk").await?;
    delete_sense_prop(ctx, 2859257, "misc", "uk").await?;
    delete_sense_prop(ctx, 1198890, "misc", "uk").await?;

    add_sense_prop(ctx, 2826371, 0, "misc", "uk").await?;
    delete_sense_prop(ctx, 2826371, "misc", "rare").await?;

    set_common(ctx, KaniReadingTable::Kana, 1625620, "はいかん", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1625610, "はいかん", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1681460, "はいかん", None).await?;

    set_common(ctx, KaniReadingTable::Kanji, 2855480, "乙女", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2855480, "おとめ", Some(0)).await?;

    set_common(ctx, KaniReadingTable::Kana, 1930050, "バラす", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1582460, "ないかい", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1202300, "かいが", Some(0)).await?;

    set_common(ctx, KaniReadingTable::Kanji, 1328740, "狩る", Some(0)).await?;

    set_common(ctx, KaniReadingTable::Kana, 1009610, "にも", Some(0)).await?;
    Ok(())
}
