//! Port of `ichiran/dict:insert-conjugation` (`dict-load.lisp:377`).
//!
//! For one (pos-id, conj-id, neg, fml) cell of the conjugation matrix,
//! resolve an existing or freshly-minted target entry, then upsert the
//! corresponding `conjugation` / `conj_prop` / `conj_source_reading`
//! rows. Returns `true` iff a brand-new target entry was created (the
//! caller increments its `next_seq` counter on a `true` return).
//!
//! Diverges from the upstream lambda list
//! `(readings &key seq from pos conj-type neg fml via)` by taking
//! `&KaniranContext` for the database handle, replacing the upstream
//! dynamic `*connection*` per [`crate::conn::kani_context`], and by
//! representing the keyword arguments as plain typed Rust parameters
//! (with `Option` for the upstream `:null`-bearing `neg` / `fml` /
//! `via` slots).

use std::cmp::Ordering;
use std::collections::HashSet;

use crate::conn::kani_context::KaniranContext;
use sqlx::Row;

use super::_star_secondary_conjugation_types_from_star_::SECONDARY_CONJUGATION_TYPES_FROM;
use super::conjugate_entry_inner::ConjMatrixEntry;
use super::lex_compare::lex_compare;

pub async fn insert_conjugation(
    ctx: &KaniranContext,
    readings: &[ConjMatrixEntry],
    seq: i32,
    from: i32,
    pos: &str,
    conj_type: i32,
    neg: Option<bool>,
    fml: Option<bool>,
    via: Option<i32>,
) -> Result<bool, sqlx::Error> {
    // dict-load.lisp:379 — (sort readings (lex-compare #'<) :key #'cdddr)
    // cdddr of (conj-text kanji-flag reading ord onum) = (ord onum)
    let cmp = lex_compare(|a: &i32, b: &i32| a < b);
    let mut sorted: Vec<ConjMatrixEntry> = readings.to_vec();
    sorted.sort_by(|a, b| {
        let ka = [a.3, a.4];
        let kb = [b.3, b.4];
        if cmp(&ka, &kb) {
            Ordering::Less
        } else if cmp(&kb, &ka) {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    });

    // dict-load.lisp:380-382 — collect source-readings, kanji-readings, kana-readings
    let mut source_readings: Vec<(String, String)> = Vec::new();
    let mut kanji_readings_raw: Vec<String> = Vec::new();
    let mut kana_readings_raw: Vec<String> = Vec::new();
    for (reading, kanji_flag, orig_reading, _ord, _onum) in &sorted {
        source_readings.push((reading.clone(), orig_reading.clone()));
        if *kanji_flag == 1 {
            kanji_readings_raw.push(reading.clone());
        } else {
            kana_readings_raw.push(reading.clone());
        }
    }
    // dict-load.lisp:384 — (unless kana-readings (return nil))
    if kana_readings_raw.is_empty() {
        return Ok(false);
    }

    // dict-load.lisp:385-386 — (remove-duplicates … :test 'equal)
    // CL default keeps the last occurrence; mirror by reversing, keeping
    // first-seen, then reversing back.
    let kanji_readings = dedupe_keep_last(kanji_readings_raw);
    let kana_readings = dedupe_keep_last(kana_readings_raw);

    // dict-load.lisp:387-408 — seq-candidates (sort … '<)
    let seq_candidates: Vec<i32> = if !kanji_readings.is_empty() {
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT seq FROM kanji_text WHERE text = ANY($1) \
             GROUP BY seq HAVING COUNT(id) = $2 \
             INTERSECT \
             SELECT seq FROM kana_text WHERE text = ANY($3) \
             GROUP BY seq HAVING COUNT(id) = $4 \
             ORDER BY seq",
        )
        .bind(&kanji_readings)
        .bind(kanji_readings.len() as i64)
        .bind(&kana_readings)
        .bind(kana_readings.len() as i64)
        .fetch_all(&ctx.pool)
        .await?;
        rows.into_iter().map(|(s,)| s).collect()
    } else {
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT r.seq FROM kana_text r \
             LEFT JOIN kanji_text k ON r.seq = k.seq \
             WHERE k.text IS NULL AND r.text = ANY($1) \
             GROUP BY r.seq HAVING COUNT(r.id) = $2 \
             ORDER BY r.seq",
        )
        .bind(&kana_readings)
        .bind(kana_readings.len() as i64)
        .fetch_all(&ctx.pool)
        .await?;
        rows.into_iter().map(|(s,)| s).collect()
    };

    // dict-load.lisp:409-410 — (when (or (member from seq-candidates) (member via seq-candidates)) (return nil))
    if seq_candidates.contains(&from) || via.is_some_and(|v| seq_candidates.contains(&v)) {
        return Ok(false);
    }

    let mut seq = seq;
    // dict-load.lisp:411 — (if seq-candidates …)
    if let Some(first) = seq_candidates.first() {
        seq = *first;
    } else {
        // dict-load.lisp:414 — (make-dao 'entry :seq seq :content "")
        // entry initforms: root-p nil, n-kanji 0, n-kana 0, primary-nokanji nil.
        sqlx::query(
            "INSERT INTO entry (seq, content, root_p, n_kanji, n_kana, primary_nokanji) \
             VALUES ($1, '', FALSE, 0, 0, FALSE)",
        )
        .bind(seq)
        .execute(&ctx.pool)
        .await?;
        // dict-load.lisp:415 — (let ((conjugate-p (when (member conj-type *secondary-conjugation-types-from*) t))))
        let conjugate_p = SECONDARY_CONJUGATION_TYPES_FROM.contains(&conj_type);
        // dict-load.lisp:416-418 — kanji-text inserts
        // kanji-text initforms (dict.lisp:86): common-tags "", nokanji nil, best-kana :null.
        for (ord, kr) in kanji_readings.iter().enumerate() {
            sqlx::query(
                "INSERT INTO kanji_text \
                 (seq, text, ord, common, common_tags, conjugate_p, nokanji, best_kana) \
                 VALUES ($1, $2, $3, NULL, '', $4, FALSE, NULL)",
            )
            .bind(seq)
            .bind(kr)
            .bind(ord as i32)
            .bind(conjugate_p)
            .execute(&ctx.pool)
            .await?;
        }
        // dict-load.lisp:419-421 — kana-text inserts
        // kana-text initforms (dict.lisp:128): common-tags "", nokanji nil, best-kanji :null.
        for (ord, kr) in kana_readings.iter().enumerate() {
            sqlx::query(
                "INSERT INTO kana_text \
                 (seq, text, ord, common, common_tags, conjugate_p, nokanji, best_kanji) \
                 VALUES ($1, $2, $3, NULL, '', $4, FALSE, NULL)",
            )
            .bind(seq)
            .bind(kr)
            .bind(ord as i32)
            .bind(conjugate_p)
            .execute(&ctx.pool)
            .await?;
        }
    }

    // dict-load.lisp:423-425 — (if via … (:= 'via via) … (:is-null 'via))
    let old_conj_ids: Vec<i32> = if let Some(via_val) = via {
        let rows = sqlx::query(
            r#"SELECT id FROM conjugation WHERE "from" = $1 AND seq = $2 AND via = $3"#,
        )
        .bind(from)
        .bind(seq)
        .bind(via_val)
        .fetch_all(&ctx.pool)
        .await?;
        rows.into_iter().map(|r| r.get::<i32, _>("id")).collect()
    } else {
        let rows = sqlx::query(
            r#"SELECT id FROM conjugation WHERE "from" = $1 AND seq = $2 AND via IS NULL"#,
        )
        .bind(from)
        .bind(seq)
        .fetch_all(&ctx.pool)
        .await?;
        rows.into_iter().map(|r| r.get::<i32, _>("id")).collect()
    };
    // dict-load.lisp:426 — (or (car old-conj) (make-dao 'conjugation :seq seq :from from :via (or via :null)))
    let conj_id: i32 = if let Some(id) = old_conj_ids.first() {
        *id
    } else {
        sqlx::query_scalar(
            r#"INSERT INTO conjugation (seq, "from", via) VALUES ($1, $2, $3) RETURNING id"#,
        )
        .bind(seq)
        .bind(from)
        .bind(via)
        .fetch_one(&ctx.pool)
        .await?
    };

    // dict-load.lisp:428-434 — (unless (select-dao 'conj-prop …) (make-dao 'conj-prop …))
    // `:===` is null-safe equality; map to `IS NOT DISTINCT FROM`.
    let existing_prop: Option<i32> = sqlx::query_scalar(
        "SELECT id FROM conj_prop \
         WHERE conj_id = $1 AND conj_type = $2 AND pos = $3 \
           AND neg IS NOT DISTINCT FROM $4 AND fml IS NOT DISTINCT FROM $5 \
         LIMIT 1",
    )
    .bind(conj_id)
    .bind(conj_type)
    .bind(pos)
    .bind(neg)
    .bind(fml)
    .fetch_optional(&ctx.pool)
    .await?;
    if existing_prop.is_none() {
        sqlx::query(
            "INSERT INTO conj_prop (conj_id, conj_type, pos, neg, fml) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(conj_id)
        .bind(conj_type)
        .bind(pos)
        .bind(neg)
        .bind(fml)
        .execute(&ctx.pool)
        .await?;
    }

    // dict-load.lisp:436 — (let ((old-csr (when old-conj (query …)))))
    let old_csr: Vec<(String, String)> = if !old_conj_ids.is_empty() {
        sqlx::query_as(
            "SELECT text, source_text FROM conj_source_reading WHERE conj_id = $1",
        )
        .bind(conj_id)
        .fetch_all(&ctx.pool)
        .await?
    } else {
        Vec::new()
    };

    // dict-load.lisp:437 — (remove-duplicates (set-difference source-readings old-csr :test 'equal) :test 'equal)
    // set-difference preserves order of the first arg; remove-duplicates default keeps the
    // last occurrence.
    let source_readings: Vec<(String, String)> = {
        let old_set: HashSet<(String, String)> = old_csr.into_iter().collect();
        let diff: Vec<(String, String)> = source_readings
            .into_iter()
            .filter(|sr| !old_set.contains(sr))
            .collect();
        dedupe_keep_last(diff)
    };

    // dict-load.lisp:438-443 — per source-reading, INSERT if not already present.
    for (text, source_text) in &source_readings {
        let existing: Option<i32> = sqlx::query_scalar(
            "SELECT id FROM conj_source_reading \
             WHERE conj_id = $1 AND text = $2 AND source_text = $3 \
             LIMIT 1",
        )
        .bind(conj_id)
        .bind(text)
        .bind(source_text)
        .fetch_optional(&ctx.pool)
        .await?;
        if existing.is_none() {
            sqlx::query(
                "INSERT INTO conj_source_reading (conj_id, text, source_text) \
                 VALUES ($1, $2, $3)",
            )
            .bind(conj_id)
            .bind(text)
            .bind(source_text)
            .execute(&ctx.pool)
            .await?;
        }
    }

    // dict-load.lisp:445 — (return (not seq-candidates))
    Ok(seq_candidates.is_empty())
}

/// Mirror CL's `(remove-duplicates seq :test 'equal)` default behaviour:
/// preserves the **last** occurrence of each value (the earlier
/// duplicates are dropped).
fn dedupe_keep_last<T: std::hash::Hash + Eq + Clone>(v: Vec<T>) -> Vec<T> {
    let mut seen: HashSet<T> = HashSet::new();
    let mut result: Vec<T> = Vec::with_capacity(v.len());
    for item in v.into_iter().rev() {
        if seen.insert(item.clone()) {
            result.push(item);
        }
    }
    result.reverse();
    result
}
