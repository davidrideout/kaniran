//! Port of `ichiran/dict:add-errata-mar18` (`dict-errata.lisp:711`).
//!
//! Applies the March-2018 batch of JMdict overrides: `common`
//! adjustments on `kana-text` / `kanji-text` rows, sense-prop tweaks,
//! one `primary-nokanji` flip, and a new sense for な.

use super::add_new_sense_star_::add_new_sense_star_;
use super::add_sense_prop::add_sense_prop;
use super::delete_sense_prop::delete_sense_prop;
use super::kani_reading_table::KaniReadingTable;
use super::set_common::set_common;
use super::set_primary_nokanji::set_primary_nokanji;
use crate::conn::kani_context::KaniranContext;

pub async fn add_errata_mar18(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    set_primary_nokanji(ctx, 1565440, false).await?;

    set_common(ctx, KaniReadingTable::Kana, 1207610, "かける", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1236100, "強いる", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1236100, "しいる", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1451750, "おんなじ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 2068330, "事故る", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1579260, "きのう", Some(2)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 2644980, "柔らかさ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2644980, "やわらかさ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2083610, "ベタ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2083610, "べた", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1119610, "ベタ", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1004840, "コロコロ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1257040, "ケンカ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1633840, "ごとき", Some(0)).await?;

    add_sense_prop(ctx, 1238460, 0, "misc", "uk").await?;

    delete_sense_prop(ctx, 1896380, "misc", "uk").await?;
    delete_sense_prop(ctx, 1157000, "misc", "uk").await?;
    delete_sense_prop(ctx, 1576360, "misc", "uk").await?;

    add_sense_prop(ctx, 1468900, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1241380, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1241380, 1, "pos", "ctr").await?;

    add_new_sense_star_(
        ctx,
        2029110,
        "prt",
        &["indicates な-adjective".to_string()],
    )
    .await?;
    Ok(())
}
