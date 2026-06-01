//! Port of `ichiran/dict:add-primary-nokanji` (`dict-errata.lisp:251`).
//!
//! Sets the entry's `primary_nokanji` flag and marks every matching
//! `kana_text` row (same `seq`, exact `text` = `reading`) as
//! `nokanji = TRUE`. The `do-readings` macro is inlined here.

use super::kana_text_dao::KanaText;
use super::set_primary_nokanji::set_primary_nokanji;
use crate::conn::kani_context::KaniranContext;

pub async fn add_primary_nokanji(
    ctx: &KaniranContext,
    seq: i32,
    reading: &str,
) -> Result<(), sqlx::Error> {
    // dict-errata.lisp:252 (set-primary-nokanji seq t)
    set_primary_nokanji(ctx, seq, true).await?;
    // dict-errata.lisp:253-255 (do-readings 'kana-text seq reading (kt) (setf (slot-value kt 'nokanji) t) (update-dao kt))
    let kts: Vec<KanaText> =
        sqlx::query_as("SELECT * FROM kana_text WHERE seq = $1 AND text = $2")
            .bind(seq)
            .bind(reading)
            .fetch_all(&ctx.pool)
            .await?;
    for mut kt in kts {
        kt.nokanji = true;
        sqlx::query(
            "UPDATE kana_text SET seq = $2, text = $3, ord = $4, common = $5, \
             common_tags = $6, conjugate_p = $7, nokanji = $8, best_kanji = $9 \
             WHERE id = $1",
        )
        .bind(kt.id)
        .bind(kt.seq)
        .bind(&kt.text)
        .bind(kt.ord)
        .bind(kt.common)
        .bind(&kt.common_tags)
        .bind(kt.conjugate_p)
        .bind(kt.nokanji)
        .bind(&kt.best_kanji)
        .execute(&ctx.pool)
        .await?;
    }
    Ok(())
}
