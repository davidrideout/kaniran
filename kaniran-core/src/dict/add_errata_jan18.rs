//! Port of `ichiran/dict:add-errata-jan18` (`dict-errata.lisp:660`).
//!
//! Applies the January-2018 batch of JMdict overrides: `common`
//! adjustments on `kana-text` / `kanji-text` rows, sense-prop
//! tweaks, two `primary-nokanji` flips, and one new reading.
//!
//! Diverges from the upstream lambda list `()` only by taking
//! `&KaniranContext` for the database handle, replacing the upstream
//! dynamic `*connection*` per [`crate::conn::kani_context`].

use super::add_reading::add_reading;
use super::add_sense_prop::add_sense_prop;
use super::delete_sense_prop::delete_sense_prop;
use super::kani_reading_table::KaniReadingTable;
use super::set_common::set_common;
use super::set_primary_nokanji::set_primary_nokanji;
use crate::conn::kani_context::KaniranContext;

pub async fn add_errata_jan18(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    set_common(ctx, KaniReadingTable::Kanji, 2067770, "等", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2067770, "ら", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1242230, "近よる", Some(38)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1315120, "字", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1315120, "あざ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1315130, "字", Some(5)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1315130, "じ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1005530, "しっくり", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1554850, "りきむ", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2812650, "ゲー", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2083340, "やろう", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2083340, "やろ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1008730, "とろ", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1457840, "ないかい", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2829697, "いかん", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2157330, "おじゃま", Some(9)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1199800, "かいらん", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2719580, "いらん", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1808040, "めちゃ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1277450, "すき", Some(9)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1006460, "ズレる", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1522290, "本会議", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1522290, "ほんかいぎ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1220570, "きたい", Some(10)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1221020, "きたい", Some(11)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2083990, "ならん", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 2518850, "切れ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1221900, "基地外", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1379380, "せいと", Some(10)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1203280, "外に", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1383690, "後継ぎ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2083600, "すまん", Some(0)).await?;

    add_reading(ctx, 1384840, "キレ", Some(0), true, None).await?;

    delete_sense_prop(ctx, 1303400, "misc", "uk").await?;
    delete_sense_prop(ctx, 1434020, "misc", "uk").await?;
    delete_sense_prop(ctx, 1196520, "misc", "uk").await?;
    delete_sense_prop(ctx, 1414190, "misc", "uk").await?;

    add_sense_prop(ctx, 1188380, 0, "misc", "uk").await?;
    add_sense_prop(ctx, 1258330, 0, "misc", "uk").await?;
    add_sense_prop(ctx, 2217330, 0, "misc", "uk").await?;

    set_primary_nokanji(ctx, 1258330, false).await?;
    set_primary_nokanji(ctx, 1588930, false).await?;

    add_sense_prop(ctx, 1445160, 0, "pos", "ctr").await?;
    Ok(())
}
