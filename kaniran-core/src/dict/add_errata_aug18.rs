//! Port of `ichiran/dict:add-errata-aug18` (`dict-errata.lisp:743`).
//!
//! Applies the August-2018 batch of JMdict overrides: `common`
//! adjustments on `kana-text` / `kanji-text` rows, sense-prop tweaks,
//! a new reading for オケ together with its `primary-nokanji` flag,
//! and a `misc` "uk" delete.
//!
//! Diverges from the upstream lambda list `()` only by taking
//! `&KaniranContext` for the database handle, replacing the upstream
//! dynamic `*connection*` per [`crate::conn::kani_context`].

use super::add_primary_nokanji::add_primary_nokanji;
use super::add_reading::add_reading;
use super::add_sense_prop::add_sense_prop;
use super::delete_sense_prop::delete_sense_prop;
use super::kani_reading_table::KaniReadingTable;
use super::set_common::set_common;
use crate::conn::kani_context::KaniranContext;

pub async fn add_errata_aug18(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    set_common(ctx, KaniReadingTable::Kana, 1593870, "さらう", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2141690, "ふざけんな", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1214770, "かん", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1214770, "観", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 2082780, "意味深", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2209180, "とて", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1574640, "ロバ", Some(0)).await?;

    add_reading(ctx, 2722640, "オケ", None, true, None).await?;
    add_primary_nokanji(ctx, 2722640, "オケ").await?;
    set_common(ctx, KaniReadingTable::Kana, 2722640, "オケ", Some(0)).await?;
    add_sense_prop(ctx, 2722640, 0, "misc", "uk").await?;
    add_sense_prop(ctx, 1527140, 0, "misc", "uk").await?;

    add_sense_prop(ctx, 1208870, 0, "misc", "uk").await?;

    delete_sense_prop(ctx, 1598660, "misc", "uk").await?;
    Ok(())
}
