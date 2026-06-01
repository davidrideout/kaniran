//! Port of `ichiran/dict:add-errata-jan21` (`dict-errata.lisp:901`).
//!
//! Diverges from the upstream lambda list `()` only by taking
//! `&KaniranContext` for the database handle, replacing the upstream
//! dynamic `*connection*` per [`crate::conn::kani_context`].

use super::add_sense_prop::add_sense_prop;
use super::delete_sense_prop::delete_sense_prop;
use super::kani_reading_table::KaniReadingTable;
use super::replace_reading::replace_reading;
use super::set_common::set_common;
use crate::conn::kani_context::KaniranContext;

pub async fn add_errata_jan21(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    set_common(ctx, KaniReadingTable::Kana, 2124820, "コロナウイルス", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2846738, "なん", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2083720, "っぽい", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1012980, "遣る", None).await?;

    add_sense_prop(ctx, 1411570, 0, "pos", "vs").await?;
    add_sense_prop(ctx, 1613860, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1613860, 1, "pos", "ctr").await?;

    add_sense_prop(ctx, 2679820, 0, "misc", "uk").await?;
    delete_sense_prop(ctx, 1426680, "misc", "uk").await?;
    add_sense_prop(ctx, 1590390, 0, "misc", "uk").await?;

    delete_sense_prop(ctx, 1215240, "pos", "ctr").await?;
    add_sense_prop(ctx, 2145410, 0, "pos", "ctr").await?;

    replace_reading(
        ctx,
        2847494,
        "いきはよいといかえりはこわい",
        "いきはよいよいかえりはこわい",
    )
    .await?;
    Ok(())
}
