//! Port of `ichiran/dict:query-parents-kanji` (`dict.lisp:404`).
//!
//! For a given `seq` and surface `text`, enumerates the
//! `(kanji-text.id, conjugation.id)` pairs whose `kanji-text` row is the
//! parent reading of the `conj-source-reading` that produced
//! `(seq, text)` — the source `seq` resolves to the conjugation's `via`
//! when set, otherwise its `from`.

use crate::conn::kani_context::KaniranContext;

pub async fn query_parents_kanji(
    ctx: &KaniranContext,
    seq: i32,
    text: &str,
) -> Result<Vec<(i32, i32)>, sqlx::Error> {
    let rows: Vec<(i32, i32)> = sqlx::query_as(
        "SELECT kt.id, conj.id \
         FROM kanji_text kt, conj_source_reading csr, conjugation conj \
         WHERE conj.seq = $1 \
           AND conj.id = csr.conj_id \
           AND csr.text = $2 \
           AND kt.seq = CASE WHEN conj.via IS NOT NULL THEN conj.via ELSE conj.from END \
           AND kt.text = csr.source_text",
    )
    .bind(seq)
    .bind(text)
    .fetch_all(&ctx.pool)
    .await?;
    Ok(rows)
}
