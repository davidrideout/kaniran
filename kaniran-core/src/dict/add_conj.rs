//! Port of `ichiran/dict:add-conj` (`dict-errata.lisp:17`).
//!
//! If no conjugation matching `options` already links `seq-from`,
//! mints a fresh entry plus its kana/kanji readings, conjugation row,
//! `conj-prop` row, and `conj-source-reading` rows from `reading-map`.
//! `options` is the 4-tuple `(conj-type, pos, neg, fml)`; `reading-map`
//! is a slice of `(src-reading, reading)` pairs.

use crate::characters::char_class_type::CharClass;
use crate::characters::test_word::test_word;
use crate::conn::kani_context::KaniranContext;

use super::find_conj::find_conj;
use super::next_seq::next_seq;

pub async fn add_conj(
    ctx: &KaniranContext,
    seq_from: i32,
    options: (i32, &str, Option<bool>, Option<bool>),
    reading_map: &[(String, String)],
) -> Result<(), sqlx::Error> {
    // dict-errata.lisp:18 (unless (find-conj seq-from options) …)
    if !find_conj(ctx, seq_from, options).await?.is_empty() {
        return Ok(());
    }
    let (conj_type, pos, neg, fml) = options;
    let next_seq = next_seq(ctx).await?;
    // dict-errata.lisp:21 (make-dao 'entry :seq next-seq :content "")
    // entry initforms (dict.lisp:26): root-p nil, n-kanji 0, n-kana 0,
    // primary-nokanji nil.
    sqlx::query(
        "INSERT INTO entry (seq, content, root_p, n_kanji, n_kana, primary_nokanji) \
         VALUES ($1, '', FALSE, 0, 0, FALSE)",
    )
    .bind(next_seq)
    .execute(&ctx.pool)
    .await?;
    // dict-errata.lisp:22-28 — per (src-reading reading), insert into
    // the kana_text / kanji_text table with the per-table ord counter.
    let mut ord_r: i32 = 0;
    let mut ord_k: i32 = 0;
    for (_src_reading, reading) in reading_map {
        let is_kana = test_word(reading, CharClass::Kana);
        let ord = if is_kana { ord_r } else { ord_k };
        if is_kana {
            // kana-text initforms (dict.lisp:128): common-tags "",
            // nokanji nil, best-kanji :null.
            sqlx::query(
                "INSERT INTO kana_text \
                 (seq, text, ord, common, common_tags, conjugate_p, nokanji, best_kanji) \
                 VALUES ($1, $2, $3, NULL, '', TRUE, FALSE, NULL)",
            )
            .bind(next_seq)
            .bind(reading)
            .bind(ord)
            .execute(&ctx.pool)
            .await?;
            ord_r += 1;
        } else {
            // kanji-text initforms (dict.lisp:86): common-tags "",
            // nokanji nil, best-kana :null.
            sqlx::query(
                "INSERT INTO kanji_text \
                 (seq, text, ord, common, common_tags, conjugate_p, nokanji, best_kana) \
                 VALUES ($1, $2, $3, NULL, '', TRUE, FALSE, NULL)",
            )
            .bind(next_seq)
            .bind(reading)
            .bind(ord)
            .execute(&ctx.pool)
            .await?;
            ord_k += 1;
        }
    }
    // dict-errata.lisp:29 (make-dao 'conjugation :seq next-seq :from seq-from)
    // conjugation initform (dict.lisp:238): via :null.
    let conj_id: i32 = sqlx::query_scalar(
        r#"INSERT INTO conjugation (seq, "from", via) VALUES ($1, $2, NULL) RETURNING id"#,
    )
    .bind(next_seq)
    .bind(seq_from)
    .fetch_one(&ctx.pool)
    .await?;
    // dict-errata.lisp:30-31 (make-dao 'conj-prop …)
    sqlx::query(
        "INSERT INTO conj_prop (conj_id, pos, conj_type, neg, fml) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(conj_id)
    .bind(pos)
    .bind(conj_type)
    .bind(neg)
    .bind(fml)
    .execute(&ctx.pool)
    .await?;
    // dict-errata.lisp:32-35 — per (src-reading reading), insert conj-source-reading.
    for (src_reading, reading) in reading_map {
        sqlx::query(
            "INSERT INTO conj_source_reading (conj_id, text, source_text) \
             VALUES ($1, $2, $3)",
        )
        .bind(conj_id)
        .bind(reading)
        .bind(src_reading)
        .execute(&ctx.pool)
        .await?;
    }
    Ok(())
}
