//! Port of `ichiran/dict:add-errata-counters` (`dict-errata.lisp:1068`).
//!
//! Applies counter-word corrections to JMdict: reading edits, new
//! senses/glosses, and `pos`=`ctr` sense-prop tagging.

use super::add_gloss::add_gloss;
use super::add_new_sense_star_::add_new_sense_star_;
use super::add_reading::add_reading;
use super::add_sense_prop::add_sense_prop;
use super::delete_reading::delete_reading;
use super::kanji_text_dao::KanjiText;
use super::set_reading::{set_reading, SetReadingObj};
use crate::conn::kani_context::KaniranContext;

pub async fn add_errata_counters(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    delete_reading(ctx, 1299960, "さんかい", None).await?;
    // dict-errata.lisp:1070 (mapc 'set-reading (select-dao 'kanji-text (:= 'seq 1299960)))
    let mut kanji_rows: Vec<KanjiText> =
        sqlx::query_as("SELECT * FROM kanji_text WHERE seq = $1")
            .bind(1299960)
            .fetch_all(&ctx.pool)
            .await?;
    for row in kanji_rows.iter_mut() {
        set_reading(ctx, SetReadingObj::Kanji(row)).await?;
    }

    add_reading(ctx, 2081610, "タテ", None, true, None).await?;

    add_sense_prop(ctx, 1427420, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1397450, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1397450, 1, "pos", "ctr").await?;
    add_sense_prop(ctx, 1351270, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1351270, 1, "pos", "n").await?;
    add_sense_prop(ctx, 1490430, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1490430, 1, "pos", "ctr").await?;
    add_sense_prop(ctx, 2020680, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1502840, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1502840, 1, "pos", "ctr").await?;
    add_sense_prop(ctx, 1373990, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1281690, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1281690, 1, "pos", "n").await?;
    add_sense_prop(ctx, 1042610, 1, "pos", "ctr").await?;
    add_sense_prop(ctx, 1042610, 2, "pos", "ctr").await?;
    add_sense_prop(ctx, 1100610, 0, "pos", "ctr").await?;

    add_new_sense_star_(ctx, 1583470, "ctr", &["counter for dishes".to_string()]).await?;

    add_sense_prop(ctx, 1411070, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1411070, 1, "pos", "n").await?;

    add_sense_prop(ctx, 1328810, 0, "pos", "ctr").await?;

    add_sense_prop(ctx, 1284220, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1284220, 1, "pos", "n").await?;
    add_sense_prop(ctx, 1284220, 1, "pos", "n-suf").await?;
    add_sense_prop(ctx, 1482360, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 2022640, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1175570, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1175570, 1, "pos", "n").await?;
    add_sense_prop(ctx, 1315130, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1315130, 1, "pos", "n").await?;
    add_sense_prop(ctx, 1199640, 0, "pos", "ctr").await?;

    add_sense_prop(ctx, 1047880, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1047880, 1, "pos", "n").await?;

    add_sense_prop(ctx, 1244080, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1244080, 1, "pos", "ctr").await?;
    add_sense_prop(ctx, 1239700, 0, "pos", "ctr").await?;

    add_sense_prop(ctx, 1294940, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1294940, 1, "pos", "suf").await?;

    add_sense_prop(ctx, 1575510, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1575510, 1, "pos", "n").await?;

    add_sense_prop(ctx, 1505390, 0, "pos", "ctr").await?;

    add_sense_prop(ctx, 1101700, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1120410, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1956400, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1333450, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1480050, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1480050, 1, "pos", "ctr").await?;
    add_sense_prop(ctx, 1480050, 2, "pos", "ctr").await?;

    add_sense_prop(ctx, 1956530, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1324110, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1324110, 1, "pos", "n").await?;
    add_sense_prop(ctx, 1382450, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1382450, 1, "pos", "ctr").await?;

    add_sense_prop(ctx, 1253800, 1, "pos", "ctr").await?;

    add_sense_prop(ctx, 1297240, 0, "pos", "ctr").await?;

    add_new_sense_star_(ctx, 2262420, "ctr", &["counter for strings".to_string()]).await?;

    add_sense_prop(ctx, 1368480, 0, "pos", "ctr").await?;
    add_gloss(ctx, 1368480, 0, &["for N people"]).await?;

    add_sense_prop(ctx, 1732510, 1, "pos", "ctr").await?;
    add_sense_prop(ctx, 1732510, 2, "pos", "ctr").await?;
    add_sense_prop(ctx, 2086480, 1, "pos", "ctr").await?;

    add_sense_prop(ctx, 1331080, 0, "pos", "ctr").await?;
    Ok(())
}
