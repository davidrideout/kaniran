//! Port of `ichiran/dict:add-errata-jan26` (`dict-errata.lisp:1005`).
//!
//! Applies the January-2026 batch of JMdict corrections (sense-prop
//! deletes, common-flag adjustments, `primary-nokanji` flips).

use super::delete_sense_prop::delete_sense_prop;
use super::kani_reading_table::KaniReadingTable;
use super::set_common::set_common;
use super::set_primary_nokanji::set_primary_nokanji;
use crate::conn::kani_context::KaniranContext;

pub async fn add_errata_jan26(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    delete_sense_prop(ctx, 1236660, "misc", "uk").await?;
    delete_sense_prop(ctx, 2859279, "misc", "uk").await?;
    delete_sense_prop(ctx, 1591420, "misc", "uk").await?;

    set_primary_nokanji(ctx, 1502390, false).await?;
    set_common(ctx, KaniReadingTable::Kana, 1502390, "モノ", Some(0)).await?;

    set_common(ctx, KaniReadingTable::Kana, 1392580, "まえ", Some(5)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1502920, "分かつ", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1169130, "引分ける", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1326660, "取り計らう", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1340420, "出来", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1340430, "出来", Some(9)).await?;

    set_common(ctx, KaniReadingTable::Kanji, 1589320, "思い", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1281000, "考え", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 2862681, "閉まり", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1989500, "開き", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1985020, "気づき", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1180130, "押し", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1216850, "含み", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1231760, "居座り", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1236660, "恐れ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1238660, "驚き", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1259890, "見直し", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1297250, "作り", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1304480, "残り", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1327090, "守り", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1327100, "守り", Some(9)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1396550, "狙い", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1403130, "増やし", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1535930, "問い", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1548390, "頼り", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1609560, "勝ち", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1954660, "聞こえ", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1497960, "負け", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1502940, "分かり", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1917220, "分かれ", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1221250, "帰り", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1351280, "笑い", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1352300, "上げ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1354720, "乗り", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1502990, "分け", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1630270, "脅かし", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1456130, "読み", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1403020, "騒ぎ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1659120, "受け", Some(0)).await?;
    Ok(())
}
