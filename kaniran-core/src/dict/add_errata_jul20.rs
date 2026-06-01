//! Port of `ichiran/dict:add-errata-jul20` (`dict-errata.lisp:878`).
//!
//! Diverges from the upstream lambda list `()` only by taking
//! `&KaniranContext` for the database handle, replacing the upstream
//! dynamic `*connection*` per [`crate::conn::kani_context`].

use super::add_primary_nokanji::add_primary_nokanji;
use super::add_reading::add_reading;
use super::add_sense_prop::add_sense_prop;
use super::delete_sense_prop::delete_sense_prop;
use super::kani_reading_table::KaniReadingTable;
use super::rearrange_readings_conj::rearrange_readings_conj;
use super::set_common::set_common;
use super::set_primary_nokanji::set_primary_nokanji;
use crate::conn::kani_context::KaniranContext;

pub async fn add_errata_jul20(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    set_common(ctx, KaniReadingTable::Kana, 2101130, "し", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1982860, "代", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1367020, "ひとけ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1002190, "おしり", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2085020, "もどき", Some(0)).await?;

    set_primary_nokanji(ctx, 1756600, false).await?;

    add_reading(ctx, 2217330, "ワイ", None, true, None).await?;
    add_primary_nokanji(ctx, 2217330, "ワイ").await?;
    add_sense_prop(ctx, 2217330, 0, "misc", "uk").await?;
    delete_sense_prop(ctx, 2217330, "misc", "arch").await?;

    add_reading(ctx, 1103270, "ぱんつ", None, true, None).await?;

    add_sense_prop(ctx, 1586290, 0, "misc", "uk").await?;

    add_sense_prop(ctx, 1257260, 0, "misc", "uk").await?;

    rearrange_readings_conj(ctx, 1980880, KaniReadingTable::Kanji, "かけ直").await?;
    Ok(())
}
