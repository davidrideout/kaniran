//! Port of `ichiran/dict:add-errata` (`dict-errata.lisp:287`).
//!
//! Top-level errata pipeline applied after the JMdict load completes.
//! Calls every errata helper in turn, then dispatches the monthly
//! `add-errata-<tag>` batches.

use super::add_deha_ja_readings::add_deha_ja_readings;
use super::add_errata_apr19::add_errata_apr19;
use super::add_errata_apr20::add_errata_apr20;
use super::add_errata_aug18::add_errata_aug18;
use super::add_errata_counters::add_errata_counters;
use super::add_errata_dec23::add_errata_dec23;
use super::add_errata_feb17::add_errata_feb17;
use super::add_errata_jan18::add_errata_jan18;
use super::add_errata_jan19::add_errata_jan19;
use super::add_errata_jan20::add_errata_jan20;
use super::add_errata_jan21::add_errata_jan21;
use super::add_errata_jan22::add_errata_jan22;
use super::add_errata_jan25::add_errata_jan25;
use super::add_errata_jan26::add_errata_jan26;
use super::add_errata_jul20::add_errata_jul20;
use super::add_errata_mar18::add_errata_mar18;
use super::add_errata_may21::add_errata_may21;
use super::add_gozaimasu_conjs::add_gozaimasu_conjs;
use super::add_primary_nokanji::add_primary_nokanji;
use super::add_reading::add_reading;
use super::add_sense_prop::add_sense_prop;
use super::conjugate_da::conjugate_da;
use super::delete_conjugation::delete_conjugation;
use super::delete_reading::delete_reading;
use super::delete_sense_prop::delete_sense_prop;
use super::delete_senses::delete_senses;
use super::kani_reading_table::KaniReadingTable;
use super::rearrange_readings_conj::rearrange_readings_conj;
use super::remove_hiragana_nokanji::remove_hiragana_nokanji;
use super::set_common::set_common;
use super::set_primary_nokanji::set_primary_nokanji;
use crate::conn::kani_context::KaniranContext;
use crate::custom::get_custom_data::CustomDataKey;
use crate::custom::load_custom_data::{load_custom_data, LoadCustomDataError};

pub async fn add_errata(ctx: &KaniranContext) -> Result<(), LoadCustomDataError> {
    conjugate_da(ctx, None).await?;
    add_deha_ja_readings(ctx).await?;
    remove_hiragana_nokanji(ctx).await?;
    add_gozaimasu_conjs(ctx, None).await?;

    set_primary_nokanji(ctx, 1538900, false).await?;
    set_primary_nokanji(ctx, 1580640, false).await?;
    set_primary_nokanji(ctx, 1289030, false).await?;

    add_primary_nokanji(ctx, 1415510, "タカ").await?;

    delete_reading(ctx, 1247250, "キミ", None).await?;
    add_reading(ctx, 2015370, "ワシ", None, true, None).await?;
    add_reading(ctx, 1202410, "カニ", None, true, None).await?;
    delete_reading(ctx, 1521960, "ボツ", None).await?;
    add_reading(ctx, 2145800, "イラ", None, true, None).await?;
    add_reading(ctx, 1517840, "ハチ", None, true, None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1517840, "ハチ", Some(34)).await?;

    add_reading(ctx, 2029080, "ねぇ", None, true, None).await?;
    // dict-errata.lisp:309 (add-reading 2089020 "じゃ" :common 0 :conjugate-p nil)
    add_reading(ctx, 2089020, "じゃ", Some(0), false, None).await?;

    delete_reading(ctx, 2145800, "いら", None).await?;

    delete_reading(ctx, 2067160, "たも", None).await?;

    delete_reading(ctx, 2423450, "サシ", None).await?;
    delete_reading(ctx, 2574600, "どうなん", None).await?;

    delete_sense_prop(ctx, 1611000, "misc", "uk").await?;
    delete_sense_prop(ctx, 1305070, "misc", "uk").await?;
    delete_sense_prop(ctx, 1583470, "misc", "uk").await?;
    delete_sense_prop(ctx, 1446760, "misc", "uk").await?;
    delete_sense_prop(ctx, 1302910, "misc", "uk").await?;
    delete_sense_prop(ctx, 2802220, "misc", "uk").await?;
    delete_sense_prop(ctx, 1535790, "misc", "uk").await?;
    delete_sense_prop(ctx, 2119750, "misc", "uk").await?;
    delete_sense_prop(ctx, 2220330, "misc", "uk").await?;
    delete_sense_prop(ctx, 1207600, "misc", "uk").await?;
    delete_sense_prop(ctx, 1399970, "misc", "uk").await?;
    delete_sense_prop(ctx, 2094480, "misc", "uk").await?;
    delete_sense_prop(ctx, 2729170, "misc", "uk").await?;
    delete_sense_prop(ctx, 1580640, "misc", "uk").await?;
    delete_sense_prop(ctx, 1569440, "misc", "uk").await?;
    delete_sense_prop(ctx, 2423450, "misc", "uk").await?;
    delete_sense_prop(ctx, 1578850, "misc", "uk").await?;
    delete_sense_prop(ctx, 1609500, "misc", "uk").await?;
    delete_sense_prop(ctx, 1444150, "misc", "uk").await?;
    delete_sense_prop(ctx, 1546640, "misc", "uk").await?;
    delete_sense_prop(ctx, 1314490, "misc", "uk").await?;
    delete_sense_prop(ctx, 2643710, "misc", "uk").await?;
    delete_sense_prop(ctx, 1611260, "misc", "uk").await?;
    delete_sense_prop(ctx, 2208960, "misc", "uk").await?;
    delete_sense_prop(ctx, 1155020, "misc", "uk").await?;
    delete_sense_prop(ctx, 1208240, "misc", "uk").await?;
    delete_sense_prop(ctx, 1207590, "misc", "uk").await?;
    delete_sense_prop(ctx, 1279680, "misc", "uk").await?;
    delete_sense_prop(ctx, 1469810, "misc", "uk").await?;
    delete_sense_prop(ctx, 1474370, "misc", "uk").await?;
    delete_sense_prop(ctx, 1609300, "misc", "uk").await?;
    delete_sense_prop(ctx, 1612920, "misc", "uk").await?;
    delete_sense_prop(ctx, 2827450, "misc", "uk").await?;
    delete_sense_prop(ctx, 1333570, "misc", "uk").await?;
    delete_sense_prop(ctx, 1610400, "misc", "uk").await?;
    delete_sense_prop(ctx, 2097190, "misc", "uk").await?;

    add_sense_prop(ctx, 1394680, 0, "misc", "uk").await?;
    add_sense_prop(ctx, 2272830, 0, "misc", "uk").await?;
    add_sense_prop(ctx, 1270680, 0, "misc", "uk").await?;
    add_sense_prop(ctx, 1541560, 0, "misc", "uk").await?;
    add_sense_prop(ctx, 1739410, 1, "misc", "uk").await?;
    add_sense_prop(ctx, 1207610, 0, "misc", "uk").await?;
    add_sense_prop(ctx, 2424410, 0, "misc", "uk").await?;
    add_sense_prop(ctx, 1387080, 0, "misc", "uk").await?;
    add_sense_prop(ctx, 1509350, 0, "misc", "uk").await?;
    add_sense_prop(ctx, 1637460, 0, "misc", "uk").await?;

    add_sense_prop(ctx, 2425930, 0, "pos", "prt").await?;
    add_sense_prop(ctx, 2457930, 0, "pos", "prt").await?;
    delete_sense_prop(ctx, 2629920, "pos", "adv-to").await?;

    set_common(ctx, KaniReadingTable::Kana, 1310920, "したい", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1159430, "いたい", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1523060, "ほんと", Some(2)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1577100, "なん", Some(2)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1012440, "めく", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1005600, "しまった", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2139720, "ん", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1309910, "してい", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1311320, "してい", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1423310, "なか", Some(1)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1245280, "空", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1308640, "しない", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1579130, "ことし", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2084660, "いなくなった", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1570850, "すね", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1470740, "のうち", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1156100, "いいん", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1472520, "はいいん", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1445000, "としん", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1408100, "たよう", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2409180, "ような", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1524550, "まいそう", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1925750, "そうする", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1587780, "いる", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1322180, "いる", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1391500, "いる", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1606560, "分かる", Some(11)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1606560, "わかる", Some(11)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1547720, "来る", Some(11)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1547720, "くる", Some(11)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2134680, "それは", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2134680, "そりゃ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1409140, "からだ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1552120, "ながす", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1516930, "ほう", Some(1)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1518220, "ほうが", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1603340, "ほうが", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1158400, "いどう", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1157970, "いどう", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1599900, "になう", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1465590, "はいる", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1535930, "とい", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1472480, "はいらん", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 2019640, "杯", Some(20)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1416220, "たち", Some(10)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1402900, "そうなん", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1446980, "いたむ", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1432710, "いたむ", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1632670, "かむ", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1224090, "きが", Some(40)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1534470, "もうこ", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1739410, "わけない", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1416860, "誰も", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2093030, "そっか", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1001840, "お兄ちゃん", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1341350, "旬", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1188790, "いつか", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1582900, "もす", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1577270, "セリフ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1375650, "せいか", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1363540, "真逆", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1632200, "どうか", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1920245, "何の", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2733410, "だよね", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1234260, "ともに", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 2242840, "未", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1246890, "リス", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1257270, "やらしい", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1343100, "とこ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1529930, "むこう", Some(14)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1317910, "自重", Some(30)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1586420, "あったかい", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1214190, "かんない", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1614320, "かんない", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1517220, "ほうがい", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1380990, "せいなん", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1280630, "こうなん", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1289620, "こんなん", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1204090, "がいまい", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1459170, "ないほう", None).await?;

    set_common(ctx, KaniReadingTable::Kana, 2457920, "ですか", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1228390, "すいもの", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1423240, "きもの", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1212110, "かんじ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1516160, "たから", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1575510, "コマ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1603990, "街", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1548520, "からむ", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2174250, "もしや", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1595080, "のく", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1309950, "しどう", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1524860, "まくら", Some(9)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1451770, "同じよう", Some(30)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1244210, "くない", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1898260, "どうし", Some(11)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1407980, "多分", Some(1)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1579630, "なのか", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1371880, "すいてき", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1008420, "でしょ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1928670, "だろ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1000580, "彼", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1546380, "ようと", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2246510, "なさそう", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 2246510, "無さそう", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1579110, "きょう", Some(2)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1235870, "きょう", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1587200, "いこう", Some(11)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1158240, "いこう", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1534440, "もうまく", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1459400, "ないよう", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1590480, "カッコ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1208240, "カッコ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1495770, "つける", Some(11)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1610400, "つける", Some(12)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1495740, "つく", Some(11)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1495740, "付く", Some(11)).await?;

    // dict-errata.lisp:542 (delete-senses 2611370 (constantly t))
    delete_senses(ctx, 2611370, |_prop| true).await?;
    // dict-errata.lisp:543-545 (let ((entry (get-dao 'entry 2611370)))
    //   (setf (slot-value entry 'root-p) nil) (update-dao entry))
    let mut entry: super::entry_dao::Entry =
        sqlx::query_as("SELECT * FROM entry WHERE seq = $1")
            .bind(2611370)
            .fetch_one(&ctx.pool)
            .await?;
    entry.root_p = false;
    sqlx::query(
        "UPDATE entry SET content = $2, root_p = $3, n_kanji = $4, \
         n_kana = $5, primary_nokanji = $6 WHERE seq = $1",
    )
    .bind(entry.seq)
    .bind(&entry.content)
    .bind(entry.root_p)
    .bind(entry.n_kanji)
    .bind(entry.n_kana)
    .bind(entry.primary_nokanji)
    .execute(&ctx.pool)
    .await?;
    delete_reading(ctx, 2611370, "為り", None).await?;

    rearrange_readings_conj(ctx, 1584060, KaniReadingTable::Kana, "つつ").await?;
    set_common(ctx, KaniReadingTable::Kana, 1584060, "つつむ", Some(6)).await?;

    rearrange_readings_conj(ctx, 1602880, KaniReadingTable::Kanji, "増や").await?;

    // dict-errata.lisp:555 (delete-senses 1008490 (lambda (prop) (and (equal (text prop) "n") (equal (tag prop) "pos"))))
    delete_senses(ctx, 1008490, |prop| prop.text == "n" && prop.tag == "pos").await?;

    // dict-errata.lisp:558 (delete-senses 2017560 (lambda (prop) (and (equal (text prop) "prt") (equal (tag prop) "pos"))))
    delete_senses(ctx, 2017560, |prop| prop.text == "prt" && prop.tag == "pos").await?;

    delete_conjugation(ctx, 2029110, 2257550, None).await?;
    delete_conjugation(ctx, 2086640, 2684620, None).await?;

    add_errata_feb17(ctx).await?;
    add_errata_jan18(ctx).await?;
    add_errata_mar18(ctx).await?;
    add_errata_aug18(ctx).await?;
    add_errata_jan19(ctx).await?;
    add_errata_apr19(ctx).await?;
    add_errata_jan20(ctx).await?;
    add_errata_apr20(ctx).await?;
    add_errata_jul20(ctx).await?;
    add_errata_jan21(ctx).await?;
    add_errata_may21(ctx).await?;
    add_errata_jan22(ctx).await?;
    add_errata_dec23(ctx).await?;
    add_errata_jan25(ctx).await?;
    add_errata_jan26(ctx).await?;
    add_errata_counters(ctx).await?;

    // dict-errata.lisp:581 (ichiran/custom:load-custom-data '(:extra) t)
    load_custom_data(ctx, &[CustomDataKey::Extra], true).await?;
    Ok(())
}
