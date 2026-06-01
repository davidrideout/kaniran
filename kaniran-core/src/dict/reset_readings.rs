//! Port of `ichiran/dict:reset-readings` (`dict-errata.lisp:70`).
//!
//! Re-runs [`set_reading`] over every `kana_text` then `kanji_text`
//! row belonging to any of `seqs`.

use super::kana_text_dao::KanaText;
use super::kanji_text_dao::KanjiText;
use super::set_reading::{set_reading, SetReadingObj};
use crate::conn::kani_context::KaniranContext;

pub async fn reset_readings(
    ctx: &KaniranContext,
    seqs: &[i32],
) -> Result<(), sqlx::Error> {
    let mut kana: Vec<KanaText> =
        sqlx::query_as("SELECT * FROM kana_text WHERE seq = ANY($1)")
            .bind(seqs)
            .fetch_all(&ctx.pool)
            .await?;
    let mut kanji: Vec<KanjiText> =
        sqlx::query_as("SELECT * FROM kanji_text WHERE seq = ANY($1)")
            .bind(seqs)
            .fetch_all(&ctx.pool)
            .await?;
    for row in kana.iter_mut() {
        set_reading(ctx, SetReadingObj::Kana(row)).await?;
    }
    for row in kanji.iter_mut() {
        set_reading(ctx, SetReadingObj::Kanji(row)).await?;
    }
    Ok(())
}
