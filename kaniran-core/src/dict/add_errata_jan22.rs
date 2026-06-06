//! Port of `ichiran/dict:add-errata-jan22` (`dict-errata.lisp:932`).
//!
//! Applies the January-2022 batch of JMdict corrections (common-flag
//! adjustments, sense-prop tweaks, new readings, conj readings).

use super::add_conj_reading::add_conj_reading;
use super::add_reading::add_reading;
use super::add_sense_prop::add_sense_prop;
use super::delete_sense_prop::delete_sense_prop;
use super::kani_reading_table::KaniReadingTable;
use super::set_common::set_common;
use crate::conn::kani_context::KaniranContext;

pub async fn add_errata_jan22(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    add_reading(ctx, 1566420, "ハメる", None, true, None).await?;
    add_conj_reading(ctx, 1566420, "ハメる").await?;

    add_reading(ctx, 1161240, "いっかねん", None, true, None).await?;

    set_common(ctx, KaniReadingTable::Kana, 2008650, "そうした", None).await?;
    add_sense_prop(ctx, 1188270, 0, "pos", "n").await?;
    delete_sense_prop(ctx, 1188270, "pos", "pn").await?;

    delete_sense_prop(ctx, 1240530, "pos", "ctr").await?;

    add_sense_prop(ctx, 1247260, 0, "pos", "n-suf").await?;

    set_common(ctx, KaniReadingTable::Kana, 1001840, "おにいちゃん", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1806840, "がいそう", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1639750, "こだから", None).await?;
    Ok(())
}
