//! Port of `ichiran/dict:add-errata-jan19` (`dict-errata.lisp:763`).
//!
//! Applies the January-2019 batch of JMdict overrides: `common`
//! adjustments, a `misc` "uk" add and delete, three new readings
//! (two with companion `add-conj-reading` calls), five proverb
//! readings dropped, one `primary-nokanji` flip, and one `arch` drop.
//!
//! Diverges from the upstream lambda list `()` only by taking
//! `&KaniranContext` for the database handle, replacing the upstream
//! dynamic `*connection*` per [`crate::conn::kani_context`].

use super::add_conj_reading::add_conj_reading;
use super::add_reading::add_reading;
use super::add_sense_prop::add_sense_prop;
use super::delete_reading::delete_reading;
use super::delete_sense_prop::delete_sense_prop;
use super::kani_reading_table::KaniReadingTable;
use super::set_common::set_common;
use super::set_primary_nokanji::set_primary_nokanji;
use crate::conn::kani_context::KaniranContext;

pub async fn add_errata_jan19(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    set_common(ctx, KaniReadingTable::Kanji, 2017470, "塗れ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2722660, "すげぇ", Some(0)).await?;

    add_sense_prop(ctx, 2756830, 0, "misc", "uk").await?;

    delete_sense_prop(ctx, 1604890, "misc", "uk").await?;

    add_reading(ctx, 1008370, "デカい", Some(0), true, None).await?;
    add_conj_reading(ctx, 1008370, "デカい").await?;
    add_reading(ctx, 1572760, "クドい", None, true, None).await?;
    add_conj_reading(ctx, 1572760, "クドい").await?;
    add_reading(ctx, 1003620, "ギュっと", None, true, None).await?;

    delete_reading(ctx, 2424520, "去る者は追わず、来たる者は拒まず", None).await?;
    delete_reading(ctx, 2570040, "朝焼けは雨、夕焼けは晴れ", None).await?;
    delete_reading(ctx, 2833961, "梅は食うとも核食うな、中に天神寝てござる", None).await?;
    delete_reading(ctx, 2834318, "二人は伴侶、三人は仲間割れ", None).await?;
    delete_reading(ctx, 2834363, "墨は餓鬼に磨らせ、筆は鬼に持たせよ", None).await?;

    set_primary_nokanji(ctx, 1631830, false).await?;

    delete_sense_prop(ctx, 1270350, "misc", "arch").await?;
    Ok(())
}
