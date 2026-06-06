//! Port of `ichiran/dict:add-errata-jan20` (`dict-errata.lisp:807`).
//!
//! Applies the January-2020 batch of JMdict corrections (common-flag
//! adjustments, sense-prop tweaks, reading add/delete, conj readings).

use super::add_conj_reading::add_conj_reading;
use super::add_reading::add_reading;
use super::add_sense_prop::add_sense_prop;
use super::delete_reading::delete_reading;
use super::delete_sense_prop::delete_sense_prop;
use super::kani_reading_table::KaniReadingTable;
use super::set_common::set_common;
use super::set_primary_nokanji::set_primary_nokanji;
use crate::conn::kani_context::KaniranContext;

pub async fn add_errata_jan20(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    add_reading(ctx, 2839843, "うえをしたへ", None, true, None).await?;
    delete_reading(ctx, 2839843, "うえをしたえ", None).await?;
    add_reading(ctx, 1593170, "コケる", None, true, None).await?;
    add_conj_reading(ctx, 1593170, "コケる").await?;

    add_sense_prop(ctx, 1565100, 0, "misc", "uk").await?;
    delete_sense_prop(ctx, 1632980, "misc", "uk").await?;
    delete_sense_prop(ctx, 1715710, "misc", "uk").await?;
    set_common(ctx, KaniReadingTable::Kana, 1715710, "みたところ", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2841254, "からって", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2028950, "とは", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1292400, "再開", Some(13)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1292400, "さいかい", Some(13)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1306200, "しよう", Some(10)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2056930, "つまらなさそう", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1164710, "一段落", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1570220, "すくむ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1352130, "うえ", Some(1)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1502390, "もん", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2780660, "もん", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2653620, "がち", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2653620, "ガチ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1135480, "モノ", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1003000, "カラカラ", Some(0)).await?;

    set_primary_nokanji(ctx, 1495000, false).await?;

    add_sense_prop(ctx, 2510160, 0, "misc", "obsc").await?;

    add_sense_prop(ctx, 1468900, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1469050, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1469050, 1, "pos", "ctr").await?;
    add_sense_prop(ctx, 1469050, 2, "pos", "ctr").await?;
    add_sense_prop(ctx, 1284270, 0, "pos", "ctr").await?;

    delete_sense_prop(ctx, 1245280, "pos", "adj-no").await?;
    delete_sense_prop(ctx, 1392570, "pos", "adj-no").await?;

    add_sense_prop(ctx, 1429740, 0, "pos", "suf").await?;
    add_sense_prop(ctx, 1429740, 1, "pos", "n").await?;
    delete_sense_prop(ctx, 2647210, "pos", "suf").await?;
    Ok(())
}
