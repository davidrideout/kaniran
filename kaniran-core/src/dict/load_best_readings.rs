//! Port of `ichiran/dict:load-best-readings` (`dict-load.lisp:532`).
//!
//! Refreshes the cached `best_kana` / `best_kanji` cross-references on
//! every root entry's kanji/kana rows by streaming them through
//! [`set_reading`]. With `reset`, every row's cached column is first
//! NULLed so the second pass recomputes from scratch.

use crate::conn::kani_context::KaniranContext;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kanji_text_dao::KanjiText;
use crate::dict::set_reading::{set_reading, SetReadingObj};

pub async fn load_best_readings(
    ctx: &KaniranContext,
    reset: bool,
) -> Result<(), sqlx::Error> {
    // dict-load.lisp:534-536 (when reset ...)
    if reset {
        sqlx::query("UPDATE kanji_text SET best_kana = NULL")
            .execute(&ctx.pool)
            .await?;
        sqlx::query("UPDATE kana_text SET best_kanji = NULL")
            .execute(&ctx.pool)
            .await?;
    }
    // dict-load.lisp:537-542 (loop for kanji in (query-dao 'kanji-text ...))
    let kanji_rows: Vec<KanjiText> = sqlx::query_as(
        "SELECT kt.* FROM kanji_text kt, entry \
         WHERE kt.seq = entry.seq AND kt.best_kana IS NULL AND entry.root_p",
    )
    .fetch_all(&ctx.pool)
    .await?;
    for mut kanji in kanji_rows {
        set_reading(ctx, SetReadingObj::Kanji(&mut kanji)).await?;
    }
    // dict-load.lisp:543-548 (loop for kana in (query-dao 'kana-text ...))
    let kana_rows: Vec<KanaText> = sqlx::query_as(
        "SELECT kt.* FROM kana_text kt, entry \
         WHERE kt.seq = entry.seq AND kt.best_kanji IS NULL AND entry.root_p",
    )
    .fetch_all(&ctx.pool)
    .await?;
    for mut kana in kana_rows {
        set_reading(ctx, SetReadingObj::Kana(&mut kana)).await?;
    }
    Ok(())
}
