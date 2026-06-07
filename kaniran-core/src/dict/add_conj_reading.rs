//! Port of `ichiran/dict:add-conj-reading` (`dict-errata.lisp:109`).
//!
//! Builds a [`root_diff_fn`] from `seq`'s headword to `reading`, then
//! for every entry conjugated from `seq` inserts a parallel reading
//! row + matching `conj_source_reading` row and bumps the conjugated
//! entry's `n_kana` / `n_kanji`.

use super::conjugation_dao::Conjugation;
use super::entry_dao::Entry;
use super::root_diff_fn::root_diff_fn;
use crate::characters::char_class::CharClass;
use crate::characters::char_class::test_word;
use crate::conn::kani_context::KaniranContext;

pub async fn add_conj_reading(
    ctx: &KaniranContext,
    seq: i32,
    reading: &str,
) -> Result<(), sqlx::Error> {
    let is_kana = test_word(reading, CharClass::Kana);
    let tname = if is_kana { "kana_text" } else { "kanji_text" };
    let base_text: String = sqlx::query_scalar(&format!(
        "SELECT text FROM {} WHERE seq = $1 AND ord = 0",
        tname
    ))
    .bind(seq)
    .fetch_one(&ctx.pool)
    .await?;
    let diff_fn = root_diff_fn(&base_text, reading);
    let conjs: Vec<Conjugation> =
        sqlx::query_as("SELECT * FROM conjugation WHERE \"from\" = $1")
            .bind(seq)
            .fetch_all(&ctx.pool)
            .await?;
    for conj in &conjs {
        let mut entry: Entry =
            sqlx::query_as("SELECT * FROM entry WHERE seq = $1")
                .bind(conj.seq)
                .fetch_one(&ctx.pool)
                .await?;
        // Upstream (text (car (select-dao …))) errors on a missing
        // base row; mirror with fetch_one → RowNotFound.
        let (base_text_conj, base_conjugate_p): (String, bool) = sqlx::query_as(&format!(
            "SELECT text, conjugate_p FROM {} WHERE seq = $1 AND ord = 0 LIMIT 1",
            tname
        ))
        .bind(conj.seq)
        .fetch_one(&ctx.pool)
        .await?;
        let new_text = diff_fn(&base_text_conj);
        let exists: Option<i32> = sqlx::query_scalar(&format!(
            "SELECT id FROM {} WHERE seq = $1 AND text = $2 LIMIT 1",
            tname
        ))
        .bind(conj.seq)
        .bind(&new_text)
        .fetch_optional(&ctx.pool)
        .await?;
        if exists.is_some() {
            continue;
        }
        let maxord: Option<i32> = sqlx::query_scalar(&format!(
            "SELECT MAX(ord) FROM {} WHERE seq = $1",
            tname
        ))
        .bind(conj.seq)
        .fetch_one(&ctx.pool)
        .await?;
        let source_text: String = sqlx::query_scalar(
            "SELECT source_text FROM conj_source_reading \
             WHERE conj_id = $1 AND text = $2",
        )
        .bind(conj.id)
        .bind(&base_text_conj)
        .fetch_one(&ctx.pool)
        .await?;
        if is_kana {
            // kana-text initforms (dict.lisp:128): common-tags "",
            // nokanji nil, best-kanji :null.
            sqlx::query(
                "INSERT INTO kana_text \
                 (seq, text, ord, common, common_tags, conjugate_p, nokanji, best_kanji) \
                 VALUES ($1, $2, $3, NULL, '', $4, FALSE, NULL)",
            )
            .bind(conj.seq)
            .bind(&new_text)
            .bind(maxord)
            .bind(base_conjugate_p)
            .execute(&ctx.pool)
            .await?;
        } else {
            // kanji-text initforms (dict.lisp:86): common-tags "",
            // nokanji nil, best-kana :null.
            sqlx::query(
                "INSERT INTO kanji_text \
                 (seq, text, ord, common, common_tags, conjugate_p, nokanji, best_kana) \
                 VALUES ($1, $2, $3, NULL, '', $4, FALSE, NULL)",
            )
            .bind(conj.seq)
            .bind(&new_text)
            .bind(maxord)
            .bind(base_conjugate_p)
            .execute(&ctx.pool)
            .await?;
        }
        let new_source_text = diff_fn(&source_text);
        sqlx::query(
            "INSERT INTO conj_source_reading (conj_id, text, source_text) \
             VALUES ($1, $2, $3)",
        )
        .bind(conj.id)
        .bind(&new_text)
        .bind(&new_source_text)
        .execute(&ctx.pool)
        .await?;
        if is_kana {
            entry.n_kana += 1;
        } else {
            entry.n_kanji += 1;
        }
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
    }
    Ok(())
}
