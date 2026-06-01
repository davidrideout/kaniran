//! Port of `ichiran/dict:add-errata-feb17` (`dict-errata.lisp:584`).
//!
//! Applies the February-2017 batch of JMdict overrides: `common`
//! adjustments on `kana-text` / `kanji-text` rows, sense-prop
//! tweaks, `primary-nokanji` flips, and two new readings.
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

pub async fn add_errata_feb17(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    set_common(ctx, KaniReadingTable::Kana, 2136890, "とする", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2100900, "となる", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1006200, "すべき", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2683060, "なのです", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2683060, "なんです", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1001200, "おい", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1001200, "おおい", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1441840, "伝い", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1409140, "身体", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 2830705, "身体", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1009040, "どきっと", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2261300, "するべき", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2215430, "には", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2210140, "まい", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2192950, "なさい", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2143350, "かも", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2106890, "そのよう", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2084040, "すれば", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2036080, "うつ", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1922760, "という", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1632520, "ふん", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1631750, "がる", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1394680, "そういう", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1208840, "かつ", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1011430, "べき", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1008340, "である", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1007960, "ちんちん", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1301230, "さんなん", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1311010, "氏", Some(20)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1311010, "うじ", Some(20)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 2101130, "氏", Some(21)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1155180, "いない", Some(10)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1609450, "思いきって", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1309320, "思いきる", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1312880, "メス", Some(15)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1312880, "めす", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2061540, "ぶっちゃける", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2034520, "ですら", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1566210, "いずれ", Some(9)).await?;

    delete_sense_prop(ctx, 2021030, "misc", "uk").await?;
    delete_sense_prop(ctx, 1586730, "misc", "uk").await?;
    delete_sense_prop(ctx, 1441400, "misc", "uk").await?;

    add_sense_prop(ctx, 1569590, 0, "misc", "uk").await?;
    add_sense_prop(ctx, 1590540, 0, "misc", "uk").await?;
    add_sense_prop(ctx, 1430200, 0, "misc", "uk").await?;

    set_primary_nokanji(ctx, 1374550, false).await?;
    set_primary_nokanji(ctx, 1591900, false).await?;
    set_primary_nokanji(ctx, 1000230, false).await?;
    set_primary_nokanji(ctx, 1517810, false).await?;
    set_primary_nokanji(ctx, 1585410, false).await?;

    add_reading(ctx, 1029150, "えっち", None, true, None).await?;
    add_reading(ctx, 1363740, "マネ", None, true, None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1363740, "マネ", Some(9)).await?;

    set_common(ctx, KaniReadingTable::Kanji, 1000420, "彼の", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 2219590, "元", Some(10)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2219590, "もと", Some(10)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1394760, "さほど", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1529560, "なし", Some(10)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1436830, "ていない", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1057580, "さぼる", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1402420, "走り", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1402420, "はしり", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1209540, "かる", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1244840, "かる", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1280640, "こうは", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1158960, "いほう", Some(0)).await?;

    delete_sense_prop(ctx, 2122310, "pos", "prt").await?;
    Ok(())
}
