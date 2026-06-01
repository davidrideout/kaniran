//! Port of `ichiran/dict:add-errata-apr20` (`dict-errata.lisp:852`).
//!
//! Diverges from the upstream lambda list `()` only by taking
//! `&KaniranContext` for the database handle, replacing the upstream
//! dynamic `*connection*` per [`crate::conn::kani_context`].

use super::add_new_sense_star_::add_new_sense_star_;
use super::add_sense_prop::add_sense_prop;
use super::kani_reading_table::KaniReadingTable;
use super::set_common::set_common;
use crate::conn::kani_context::KaniranContext;

pub async fn add_errata_apr20(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    set_common(ctx, KaniReadingTable::Kana, 1225940, "アリ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1568080, "ふくろう", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1025450, "ウイルス", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1025450, "ウィルス", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1004320, "こうゆう", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1580290, "おとめ", Some(0)).await?;

    add_sense_prop(ctx, 1219510, 0, "misc", "uk").await?;
    add_sense_prop(ctx, 1616370, 0, "misc", "uk").await?;

    add_new_sense_star_(ctx, 1315920, "ctr", &["hours (period of)".to_string()]).await?;

    add_sense_prop(ctx, 1220540, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1220540, 3, "pos", "ctr").await?;
    add_sense_prop(ctx, 1220540, 4, "pos", "ctr").await?;
    add_sense_prop(ctx, 1220540, 5, "pos", "ctr").await?;
    add_sense_prop(ctx, 1220540, 6, "pos", "ctr").await?;

    add_sense_prop(ctx, 2842087, 0, "pos", "ctr").await?;
    set_common(ctx, KaniReadingTable::Kana, 2842087, "パー", Some(0)).await?;

    add_sense_prop(ctx, 1956530, 1, "pos", "n").await?;
    Ok(())
}
