use super::add_new_sense::add_new_sense;
use super::apply_patch::apply_patch;
use super::conj_data_struct::ConjData;
use super::conj_prop_dao::ConjProp;
use super::conj_source_reading_dao::ConjSourceReading;
use super::conjugate_entry_outer::conjugate_entry_outer;
use super::conjugation_dao::Conjugation;
use super::conjugation_rule_struct::ConjugationRule;
use super::entry_dao::Entry;
use super::get_pos::get_pos;
use super::get_pos_index::get_pos_index;
use super::gloss_dao::Gloss;
use super::kana_text_dao::KanaText;
use super::kani_conj_form::{ConjForm, FormToken};
use super::kani_reading_table::KaniReadingTable;
use super::kanji_text_dao::KanjiText;
use super::next_seq::next_seq;
use super::sense_prop_dao::SenseProp;
use super::set_reading::{set_reading, SetReadingObj};
use crate::characters::char_class::{test_word, CharClass};
use crate::conn::kani_context::KaniranContext;
use crate::custom::load::{load_custom_data, CustomDataKey, LoadCustomDataError};
use sqlx::Row;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Port of `ichiran/dict:find-conj` (`dict-errata.lisp:3`).
///
/// Returns conjugation ids from `seq_from` whose `(conj-type, pos, neg,
/// fml)` quadruple matches `options`.
pub async fn find_conj(
    ctx: &KaniranContext,
    seq_from: i32,
    options: (i32, &str, Option<bool>, Option<bool>),
) -> Result<Vec<i32>, sqlx::Error> {
    let (conj_type, pos, neg, fml) = options;
    let rows: Vec<(i32,)> = sqlx::query_as(
        r#"SELECT conj.id
           FROM conjugation AS conj
           INNER JOIN conj_prop AS prop ON prop.conj_id = conj.id
           WHERE conj."from" = $1
             AND prop.conj_type = $2
             AND prop.pos = $3
             AND prop.neg IS NOT DISTINCT FROM $4
             AND prop.fml IS NOT DISTINCT FROM $5"#,
    )
    .bind(seq_from)
    .bind(conj_type)
    .bind(pos)
    .bind(neg)
    .bind(fml)
    .fetch_all(&ctx.pool)
    .await?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

/// Port of `ichiran/dict:add-conj` (`dict-errata.lisp:17`).
///
/// If no conjugation matching `options` already links `seq-from`,
/// mints a fresh entry plus its kana/kanji readings, conjugation row,
/// `conj-prop` row, and `conj-source-reading` rows from `reading-map`.
/// `options` is the 4-tuple `(conj-type, pos, neg, fml)`; `reading-map`
/// is a slice of `(src-reading, reading)` pairs.
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

/// Port of `ichiran/dict:add-reading` (`dict-errata.lisp:35`).
///
/// Inserts `reading` into the entry's kana or kanji table at the next
/// `ord` and bumps the entry's `n_kana` / `n_kanji`; returns the
/// (possibly updated) entry.
pub async fn add_reading(
    ctx: &KaniranContext,
    seq: i32,
    reading: &str,
    common: Option<i32>,
    conjugate_p: bool,
    table: Option<KaniReadingTable>,
) -> Result<Entry, sqlx::Error> {
    let is_kana = test_word(reading, CharClass::Kana);
    let table = table.unwrap_or(if is_kana {
        KaniReadingTable::Kana
    } else {
        KaniReadingTable::Kanji
    });
    let mut entry: Entry = sqlx::query_as("SELECT * FROM entry WHERE seq = $1")
        .bind(seq)
        .fetch_one(&ctx.pool)
        .await?;
    let tname = table.table_name();
    let existing: Option<i32> = sqlx::query_scalar(&format!(
        "SELECT id FROM {} WHERE seq = $1 AND text = $2 LIMIT 1",
        tname
    ))
    .bind(seq)
    .bind(reading)
    .fetch_optional(&ctx.pool)
    .await?;
    if existing.is_none() {
        let maxord: Option<i32> =
            sqlx::query_scalar(&format!("SELECT MAX(ord) FROM {} WHERE seq = $1", tname))
                .bind(seq)
                .fetch_one(&ctx.pool)
                .await?;
        let ord = match maxord {
            None => 0,
            Some(m) => m + 1,
        };
        match table {
            KaniReadingTable::Kana => {
                // kana-text initforms (dict.lisp:128): common-tags "",
                // nokanji nil, best-kanji :null.
                sqlx::query(
                    "INSERT INTO kana_text \
                     (seq, text, ord, common, common_tags, conjugate_p, nokanji, best_kanji) \
                     VALUES ($1, $2, $3, $4, '', $5, FALSE, NULL)",
                )
                .bind(seq)
                .bind(reading)
                .bind(ord)
                .bind(common)
                .bind(conjugate_p)
                .execute(&ctx.pool)
                .await?;
                entry.n_kana += 1;
            }
            KaniReadingTable::Kanji => {
                // kanji-text initforms (dict.lisp:86): common-tags "",
                // nokanji nil, best-kana :null.
                sqlx::query(
                    "INSERT INTO kanji_text \
                     (seq, text, ord, common, common_tags, conjugate_p, nokanji, best_kana) \
                     VALUES ($1, $2, $3, $4, '', $5, FALSE, NULL)",
                )
                .bind(seq)
                .bind(reading)
                .bind(ord)
                .bind(common)
                .bind(conjugate_p)
                .execute(&ctx.pool)
                .await?;
                entry.n_kanji += 1;
            }
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
    Ok(entry)
}

/// Port of `ichiran/dict:replace-reading` (`dict-errata.lisp:49`).
///
/// Renames every row of the entry's kana or kanji table from
/// `reading_from` to `reading_to`, then calls [`reset_readings`] iff
/// at least one row was updated.
pub async fn replace_reading(
    ctx: &KaniranContext,
    seq: i32,
    reading_from: &str,
    reading_to: &str,
) -> Result<(), sqlx::Error> {
    let is_kana = test_word(reading_from, CharClass::Kana);
    let tname = if is_kana { "kana_text" } else { "kanji_text" };
    let updated = sqlx::query(&format!(
        "UPDATE {} SET text = $1 WHERE seq = $2 AND text = $3",
        tname
    ))
    .bind(reading_to)
    .bind(seq)
    .bind(reading_from)
    .execute(&ctx.pool)
    .await?
    .rows_affected();
    if updated > 0 {
        reset_readings(ctx, &[seq]).await?;
    }
    Ok(())
}

/// Port of `ichiran/dict:replace-reading-conj` (`dict-errata.lisp:60`).
///
/// For `seq` and every entry conjugated from it, rewrites rows of
/// `table` whose `text` starts with `prefix_from` to start with
/// `prefix_to` instead, then [`reset_readings`] across the touched
/// seqs.
pub async fn replace_reading_conj(
    ctx: &KaniranContext,
    seq: i32,
    table: KaniReadingTable,
    prefix_from: &str,
    prefix_to: &str,
) -> Result<(), sqlx::Error> {
    let mut seqs: Vec<i32> = vec![seq];
    let conj_seqs: Vec<i32> =
        sqlx::query_scalar("SELECT DISTINCT seq FROM conjugation WHERE \"from\" = $1")
            .bind(seq)
            .fetch_all(&ctx.pool)
            .await?;
    seqs.extend(conj_seqs);
    let tname = table.table_name();
    let like_pat = format!("{}%", prefix_from);
    let rows: Vec<(i32, i32, String)> = sqlx::query_as(&format!(
        "SELECT id, seq, text FROM {} WHERE seq = ANY($1) AND text LIKE $2 ORDER BY seq",
        tname
    ))
    .bind(&seqs)
    .bind(&like_pat)
    .fetch_all(&ctx.pool)
    .await?;
    let prefix_from_chars = prefix_from.chars().count();
    let mut to_update: Vec<i32> = Vec::new();
    for (id, row_seq, text) in &rows {
        let tail: String = text.chars().skip(prefix_from_chars).collect();
        let new_text = format!("{}{}", prefix_to, tail);
        sqlx::query(&format!("UPDATE {} SET text = $1 WHERE id = $2", tname))
            .bind(&new_text)
            .bind(id)
            .execute(&ctx.pool)
            .await?;
        to_update.push(*row_seq);
    }
    if !to_update.is_empty() {
        reset_readings(ctx, &to_update).await?;
    }
    Ok(())
}

/// Port of `ichiran/dict:reset-readings` (`dict-errata.lisp:70`).
///
/// Re-runs [`set_reading`] over every `kana_text` then `kanji_text`
/// row belonging to any of `seqs`.
pub async fn reset_readings(ctx: &KaniranContext, seqs: &[i32]) -> Result<(), sqlx::Error> {
    let mut kana: Vec<KanaText> = sqlx::query_as("SELECT * FROM kana_text WHERE seq = ANY($1)")
        .bind(seqs)
        .fetch_all(&ctx.pool)
        .await?;
    let mut kanji: Vec<KanjiText> = sqlx::query_as("SELECT * FROM kanji_text WHERE seq = ANY($1)")
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

/// Port of `ichiran/dict:delete-reading` (`dict-errata.lisp:76`).
///
/// Deletes every row of the entry's kana or kanji table whose `text`
/// equals `reading`, decrements the entry's matching counter,
/// renumbers survivors so `ord` is 0-based and contiguous, then calls
/// [`reset_readings`].
pub async fn delete_reading(
    ctx: &KaniranContext,
    seq: i32,
    reading: &str,
    table: Option<KaniReadingTable>,
) -> Result<(), sqlx::Error> {
    let is_kana = test_word(reading, CharClass::Kana);
    let table = table.unwrap_or(if is_kana {
        KaniReadingTable::Kana
    } else {
        KaniReadingTable::Kanji
    });
    let mut entry: Entry = sqlx::query_as("SELECT * FROM entry WHERE seq = $1")
        .bind(seq)
        .fetch_one(&ctx.pool)
        .await?;
    let tname = table.table_name();
    let to_delete: Vec<i32> = sqlx::query(&format!(
        "SELECT id FROM {} WHERE seq = $1 AND text = $2",
        tname
    ))
    .bind(seq)
    .bind(reading)
    .fetch_all(&ctx.pool)
    .await?
    .into_iter()
    .map(|row| row.get::<i32, _>("id"))
    .collect();
    if !to_delete.is_empty() {
        let mut deleted: i32 = 0;
        for id in &to_delete {
            sqlx::query(&format!("DELETE FROM {} WHERE id = $1", tname))
                .bind(id)
                .execute(&ctx.pool)
                .await?;
            deleted += 1;
        }
        match table {
            KaniReadingTable::Kana => entry.n_kana -= deleted,
            KaniReadingTable::Kanji => entry.n_kanji -= deleted,
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
        let survivor_ids: Vec<i32> = sqlx::query(&format!(
            "SELECT id FROM {} WHERE seq = $1 ORDER BY ord",
            tname
        ))
        .bind(seq)
        .fetch_all(&ctx.pool)
        .await?
        .into_iter()
        .map(|row| row.get::<i32, _>("id"))
        .collect();
        for (new_ord, id) in survivor_ids.iter().enumerate() {
            sqlx::query(&format!("UPDATE {} SET ord = $1 WHERE id = $2", tname))
                .bind(new_ord as i32)
                .bind(id)
                .execute(&ctx.pool)
                .await?;
        }
        reset_readings(ctx, &[seq]).await?;
    }
    Ok(())
}

/// Port of `ichiran/dict:root-diff` (`dict-errata.lisp:95`).
///
/// Counts how many leading characters of `base_text` and `reading`
/// sit outside their shared right-aligned tail.
pub fn root_diff(base_text: &str, reading: &str) -> (usize, usize) {
    let base_chars: Vec<char> = base_text.chars().collect();
    let reading_chars: Vec<char> = reading.chars().collect();
    let lb = base_chars.len();
    let lr = reading_chars.len();
    let mut ib = lb;
    let mut ir = lr;
    while ib > 0 && ir > 0 {
        ib -= 1;
        ir -= 1;
        if base_chars[ib] != reading_chars[ir] {
            return (ib + 1, ir + 1);
        }
    }
    if lr >= lb {
        (0, lr - lb)
    } else {
        (lb - lr, 0)
    }
}

/// Port of `ichiran/dict:root-diff-fn` (`dict-errata.lisp:104`).
///
/// Returns a closure that rewrites the leading `b` characters of its
/// input with the leading `r` characters of `reading`, where `(b, r)
/// = root_diff(base_text, reading)`.
pub fn root_diff_fn(base_text: &str, reading: &str) -> impl Fn(&str) -> String {
    let (b, r) = root_diff(base_text, reading);
    let prefix: String = reading.chars().take(r).collect();
    move |text| {
        let mut out = prefix.clone();
        out.extend(text.chars().skip(b));
        out
    }
}

/// Port of `ichiran/dict:add-conj-reading` (`dict-errata.lisp:109`).
///
/// Builds a [`root_diff_fn`] from `seq`'s headword to `reading`, then
/// for every entry conjugated from `seq` inserts a parallel reading
/// row + matching `conj_source_reading` row and bumps the conjugated
/// entry's `n_kana` / `n_kanji`.
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
    let conjs: Vec<Conjugation> = sqlx::query_as("SELECT * FROM conjugation WHERE \"from\" = $1")
        .bind(seq)
        .fetch_all(&ctx.pool)
        .await?;
    for conj in &conjs {
        let mut entry: Entry = sqlx::query_as("SELECT * FROM entry WHERE seq = $1")
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
        let maxord: Option<i32> =
            sqlx::query_scalar(&format!("SELECT MAX(ord) FROM {} WHERE seq = $1", tname))
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

/// Port of `ichiran/dict:delete-senses` (`dict-errata.lisp:131`).
///
/// Drops every sense (and its glosses and remaining props) whose
/// sense-props on `seq` include any that satisfy `prop_test`. Deletes
/// all rows linked to the matched `sense-id`s, not just the matched
/// props themselves.
pub async fn delete_senses(
    ctx: &KaniranContext,
    seq: i32,
    prop_test: impl Fn(&SenseProp) -> bool,
) -> Result<(), sqlx::Error> {
    let all_props: Vec<SenseProp> = sqlx::query_as("SELECT * FROM sense_prop WHERE seq = $1")
        .bind(seq)
        .fetch_all(&ctx.pool)
        .await?;
    let sense_props: Vec<&SenseProp> = all_props.iter().filter(|p| prop_test(p)).collect();
    let mut sense_ids: Vec<i32> = Vec::new();
    for prop in &sense_props {
        if !sense_ids.contains(&prop.sense_id) {
            sense_ids.push(prop.sense_id);
        }
    }
    sqlx::query("DELETE FROM sense_prop WHERE sense_id = ANY($1)")
        .bind(&sense_ids)
        .execute(&ctx.pool)
        .await?;
    sqlx::query("DELETE FROM gloss WHERE sense_id = ANY($1)")
        .bind(&sense_ids)
        .execute(&ctx.pool)
        .await?;
    sqlx::query("DELETE FROM sense WHERE id = ANY($1)")
        .bind(&sense_ids)
        .execute(&ctx.pool)
        .await?;
    Ok(())
}

/// Port of `ichiran/dict:delete-sense-prop` (`dict-errata.lisp:138`).
///
/// Removes every `sense-prop` row matching `(seq, tag, text)`.
pub async fn delete_sense_prop(
    ctx: &KaniranContext,
    seq: i32,
    tag: &str,
    text: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM sense_prop WHERE seq = $1 AND tag = $2 AND text = $3")
        .bind(seq)
        .bind(tag)
        .bind(text)
        .execute(&ctx.pool)
        .await?;
    Ok(())
}

/// Port of `ichiran/dict:add-sense-prop` (`dict-errata.lisp:142`).
///
/// Looks up the sense at `(seq, sense-ord)` and, when present, inserts
/// one `sense-prop` row `(tag, text)` unless the same `(sense-id, tag,
/// text)` triple already exists.
pub async fn add_sense_prop(
    ctx: &KaniranContext,
    seq: i32,
    sense_ord: i32,
    tag: &str,
    text: &str,
) -> Result<(), sqlx::Error> {
    // dict-errata.lisp:143 (car (select-dao 'sense (:and (:= 'seq seq) (:= 'ord sense-ord))))
    let sense_id: Option<i32> =
        sqlx::query_scalar("SELECT id FROM sense WHERE seq = $1 AND ord = $2 LIMIT 1")
            .bind(seq)
            .bind(sense_ord)
            .fetch_optional(&ctx.pool)
            .await?;
    let Some(sense_id) = sense_id else {
        return Ok(());
    };
    // dict-errata.lisp:145 (unless (select-dao 'sense-prop …))
    let existing: Option<i32> = sqlx::query_scalar(
        "SELECT id FROM sense_prop WHERE sense_id = $1 AND tag = $2 AND text = $3 LIMIT 1",
    )
    .bind(sense_id)
    .bind(tag)
    .bind(text)
    .fetch_optional(&ctx.pool)
    .await?;
    if existing.is_some() {
        return Ok(());
    }
    // dict-errata.lisp:146 (make-dao 'sense-prop :sense-id … :tag tag :text text :ord 0 :seq seq)
    sqlx::query(
        "INSERT INTO sense_prop (sense_id, tag, text, ord, seq) \
         VALUES ($1, $2, $3, 0, $4)",
    )
    .bind(sense_id)
    .bind(tag)
    .bind(text)
    .bind(seq)
    .execute(&ctx.pool)
    .await?;
    Ok(())
}

/// Port of `ichiran/dict:add-sense` (`dict-errata.lisp:148`).
///
/// Inserts a new sense at `(seq, ord)` plus its glosses, unless a
/// sense at `(seq, ord)` already exists.
pub async fn add_sense(
    ctx: &KaniranContext,
    seq: i32,
    ord: i32,
    glosses: &[&str],
) -> Result<(), sqlx::Error> {
    // dict-errata.lisp:149 (unless (select-dao 'sense (:and (:= 'seq seq) (:= 'ord ord))))
    let existing: Option<i32> =
        sqlx::query_scalar("SELECT id FROM sense WHERE seq = $1 AND ord = $2 LIMIT 1")
            .bind(seq)
            .bind(ord)
            .fetch_optional(&ctx.pool)
            .await?;
    if existing.is_some() {
        return Ok(());
    }
    // dict-errata.lisp:150 (id (make-dao 'sense :seq seq :ord ord))
    let sense_id: i32 =
        sqlx::query_scalar("INSERT INTO sense (seq, ord) VALUES ($1, $2) RETURNING id")
            .bind(seq)
            .bind(ord)
            .fetch_one(&ctx.pool)
            .await?;
    // dict-errata.lisp:151-153 (loop for gord from 0 for gloss in glosses do (make-dao 'gloss …))
    for (gord, gloss) in glosses.iter().enumerate() {
        sqlx::query("INSERT INTO gloss (sense_id, text, ord) VALUES ($1, $2, $3)")
            .bind(sense_id)
            .bind(gloss)
            .bind(gord as i32)
            .execute(&ctx.pool)
            .await?;
    }
    Ok(())
}

/// Port of `ichiran/dict:add-errata` (`dict-errata.lisp:287`).
///
/// Top-level errata pipeline applied after the JMdict load completes.
/// Calls every errata helper in turn, then dispatches the monthly
/// `add-errata-<tag>` batches.
pub async fn add_errata(ctx: &KaniranContext) -> Result<(), LoadCustomDataError> {
    conjugate_da(ctx, None).await?;
    add_deha_ja_readings(ctx).await?;
    remove_hiragana_nokanji(ctx).await?;
    add_gozaimasu_conjs(ctx, None).await?;

    set_primary_nokanji(ctx, 1538900, false).await?;
    set_primary_nokanji(ctx, 1580640, false).await?;
    set_primary_nokanji(ctx, 1289030, false).await?;

    add_primary_nokanji(ctx, 1415510, "タカ").await?;

    delete_reading(ctx, 1247250, "キミ", None).await?;
    add_reading(ctx, 2015370, "ワシ", None, true, None).await?;
    add_reading(ctx, 1202410, "カニ", None, true, None).await?;
    delete_reading(ctx, 1521960, "ボツ", None).await?;
    add_reading(ctx, 2145800, "イラ", None, true, None).await?;
    add_reading(ctx, 1517840, "ハチ", None, true, None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1517840, "ハチ", Some(34)).await?;

    add_reading(ctx, 2029080, "ねぇ", None, true, None).await?;
    // dict-errata.lisp:309 (add-reading 2089020 "じゃ" :common 0 :conjugate-p nil)
    add_reading(ctx, 2089020, "じゃ", Some(0), false, None).await?;

    delete_reading(ctx, 2145800, "いら", None).await?;

    delete_reading(ctx, 2067160, "たも", None).await?;

    delete_reading(ctx, 2423450, "サシ", None).await?;
    delete_reading(ctx, 2574600, "どうなん", None).await?;

    delete_sense_prop(ctx, 1611000, "misc", "uk").await?;
    delete_sense_prop(ctx, 1305070, "misc", "uk").await?;
    delete_sense_prop(ctx, 1583470, "misc", "uk").await?;
    delete_sense_prop(ctx, 1446760, "misc", "uk").await?;
    delete_sense_prop(ctx, 1302910, "misc", "uk").await?;
    delete_sense_prop(ctx, 2802220, "misc", "uk").await?;
    delete_sense_prop(ctx, 1535790, "misc", "uk").await?;
    delete_sense_prop(ctx, 2119750, "misc", "uk").await?;
    delete_sense_prop(ctx, 2220330, "misc", "uk").await?;
    delete_sense_prop(ctx, 1207600, "misc", "uk").await?;
    delete_sense_prop(ctx, 1399970, "misc", "uk").await?;
    delete_sense_prop(ctx, 2094480, "misc", "uk").await?;
    delete_sense_prop(ctx, 2729170, "misc", "uk").await?;
    delete_sense_prop(ctx, 1580640, "misc", "uk").await?;
    delete_sense_prop(ctx, 1569440, "misc", "uk").await?;
    delete_sense_prop(ctx, 2423450, "misc", "uk").await?;
    delete_sense_prop(ctx, 1578850, "misc", "uk").await?;
    delete_sense_prop(ctx, 1609500, "misc", "uk").await?;
    delete_sense_prop(ctx, 1444150, "misc", "uk").await?;
    delete_sense_prop(ctx, 1546640, "misc", "uk").await?;
    delete_sense_prop(ctx, 1314490, "misc", "uk").await?;
    delete_sense_prop(ctx, 2643710, "misc", "uk").await?;
    delete_sense_prop(ctx, 1611260, "misc", "uk").await?;
    delete_sense_prop(ctx, 2208960, "misc", "uk").await?;
    delete_sense_prop(ctx, 1155020, "misc", "uk").await?;
    delete_sense_prop(ctx, 1208240, "misc", "uk").await?;
    delete_sense_prop(ctx, 1207590, "misc", "uk").await?;
    delete_sense_prop(ctx, 1279680, "misc", "uk").await?;
    delete_sense_prop(ctx, 1469810, "misc", "uk").await?;
    delete_sense_prop(ctx, 1474370, "misc", "uk").await?;
    delete_sense_prop(ctx, 1609300, "misc", "uk").await?;
    delete_sense_prop(ctx, 1612920, "misc", "uk").await?;
    delete_sense_prop(ctx, 2827450, "misc", "uk").await?;
    delete_sense_prop(ctx, 1333570, "misc", "uk").await?;
    delete_sense_prop(ctx, 1610400, "misc", "uk").await?;
    delete_sense_prop(ctx, 2097190, "misc", "uk").await?;

    add_sense_prop(ctx, 1394680, 0, "misc", "uk").await?;
    add_sense_prop(ctx, 2272830, 0, "misc", "uk").await?;
    add_sense_prop(ctx, 1270680, 0, "misc", "uk").await?;
    add_sense_prop(ctx, 1541560, 0, "misc", "uk").await?;
    add_sense_prop(ctx, 1739410, 1, "misc", "uk").await?;
    add_sense_prop(ctx, 1207610, 0, "misc", "uk").await?;
    add_sense_prop(ctx, 2424410, 0, "misc", "uk").await?;
    add_sense_prop(ctx, 1387080, 0, "misc", "uk").await?;
    add_sense_prop(ctx, 1509350, 0, "misc", "uk").await?;
    add_sense_prop(ctx, 1637460, 0, "misc", "uk").await?;

    add_sense_prop(ctx, 2425930, 0, "pos", "prt").await?;
    add_sense_prop(ctx, 2457930, 0, "pos", "prt").await?;
    delete_sense_prop(ctx, 2629920, "pos", "adv-to").await?;

    set_common(ctx, KaniReadingTable::Kana, 1310920, "したい", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1159430, "いたい", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1523060, "ほんと", Some(2)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1577100, "なん", Some(2)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1012440, "めく", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1005600, "しまった", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2139720, "ん", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1309910, "してい", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1311320, "してい", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1423310, "なか", Some(1)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1245280, "空", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1308640, "しない", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1579130, "ことし", Some(0)).await?;
    set_common(
        ctx,
        KaniReadingTable::Kana,
        2084660,
        "いなくなった",
        Some(0),
    )
    .await?;
    set_common(ctx, KaniReadingTable::Kana, 1570850, "すね", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1470740, "のうち", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1156100, "いいん", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1472520, "はいいん", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1445000, "としん", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1408100, "たよう", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2409180, "ような", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1524550, "まいそう", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1925750, "そうする", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1587780, "いる", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1322180, "いる", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1391500, "いる", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1606560, "分かる", Some(11)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1606560, "わかる", Some(11)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1547720, "来る", Some(11)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1547720, "くる", Some(11)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2134680, "それは", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2134680, "そりゃ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1409140, "からだ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1552120, "ながす", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1516930, "ほう", Some(1)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1518220, "ほうが", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1603340, "ほうが", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1158400, "いどう", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1157970, "いどう", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1599900, "になう", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1465590, "はいる", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1535930, "とい", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1472480, "はいらん", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 2019640, "杯", Some(20)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1416220, "たち", Some(10)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1402900, "そうなん", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1446980, "いたむ", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1432710, "いたむ", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1632670, "かむ", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1224090, "きが", Some(40)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1534470, "もうこ", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1739410, "わけない", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1416860, "誰も", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2093030, "そっか", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1001840, "お兄ちゃん", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1341350, "旬", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1188790, "いつか", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1582900, "もす", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1577270, "セリフ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1375650, "せいか", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1363540, "真逆", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1632200, "どうか", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1920245, "何の", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2733410, "だよね", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1234260, "ともに", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 2242840, "未", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1246890, "リス", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1257270, "やらしい", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1343100, "とこ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1529930, "むこう", Some(14)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1317910, "自重", Some(30)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1586420, "あったかい", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1214190, "かんない", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1614320, "かんない", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1517220, "ほうがい", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1380990, "せいなん", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1280630, "こうなん", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1289620, "こんなん", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1204090, "がいまい", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1459170, "ないほう", None).await?;

    set_common(ctx, KaniReadingTable::Kana, 2457920, "ですか", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1228390, "すいもの", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1423240, "きもの", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1212110, "かんじ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1516160, "たから", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1575510, "コマ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1603990, "街", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1548520, "からむ", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2174250, "もしや", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1595080, "のく", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1309950, "しどう", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1524860, "まくら", Some(9)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1451770, "同じよう", Some(30)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1244210, "くない", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1898260, "どうし", Some(11)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1407980, "多分", Some(1)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1579630, "なのか", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1371880, "すいてき", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1008420, "でしょ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1928670, "だろ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1000580, "彼", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1546380, "ようと", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2246510, "なさそう", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 2246510, "無さそう", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1579110, "きょう", Some(2)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1235870, "きょう", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1587200, "いこう", Some(11)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1158240, "いこう", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1534440, "もうまく", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1459400, "ないよう", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1590480, "カッコ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1208240, "カッコ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1495770, "つける", Some(11)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1610400, "つける", Some(12)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1495740, "つく", Some(11)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1495740, "付く", Some(11)).await?;

    // dict-errata.lisp:542 (delete-senses 2611370 (constantly t))
    delete_senses(ctx, 2611370, |_prop| true).await?;
    // dict-errata.lisp:543-545 (let ((entry (get-dao 'entry 2611370)))
    //   (setf (slot-value entry 'root-p) nil) (update-dao entry))
    let mut entry: super::entry_dao::Entry = sqlx::query_as("SELECT * FROM entry WHERE seq = $1")
        .bind(2611370)
        .fetch_one(&ctx.pool)
        .await?;
    entry.root_p = false;
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
    delete_reading(ctx, 2611370, "為り", None).await?;

    rearrange_readings_conj(ctx, 1584060, KaniReadingTable::Kana, "つつ").await?;
    set_common(ctx, KaniReadingTable::Kana, 1584060, "つつむ", Some(6)).await?;

    rearrange_readings_conj(ctx, 1602880, KaniReadingTable::Kanji, "増や").await?;

    // dict-errata.lisp:555 (delete-senses 1008490 (lambda (prop) (and (equal (text prop) "n") (equal (tag prop) "pos"))))
    delete_senses(ctx, 1008490, |prop| prop.text == "n" && prop.tag == "pos").await?;

    // dict-errata.lisp:558 (delete-senses 2017560 (lambda (prop) (and (equal (text prop) "prt") (equal (tag prop) "pos"))))
    delete_senses(ctx, 2017560, |prop| prop.text == "prt" && prop.tag == "pos").await?;

    delete_conjugation(ctx, 2029110, 2257550, None).await?;
    delete_conjugation(ctx, 2086640, 2684620, None).await?;

    add_errata_feb17(ctx).await?;
    add_errata_jan18(ctx).await?;
    add_errata_mar18(ctx).await?;
    add_errata_aug18(ctx).await?;
    add_errata_jan19(ctx).await?;
    add_errata_apr19(ctx).await?;
    add_errata_jan20(ctx).await?;
    add_errata_apr20(ctx).await?;
    add_errata_jul20(ctx).await?;
    add_errata_jan21(ctx).await?;
    add_errata_may21(ctx).await?;
    add_errata_jan22(ctx).await?;
    add_errata_dec23(ctx).await?;
    add_errata_jan25(ctx).await?;
    add_errata_jan26(ctx).await?;
    add_errata_counters(ctx).await?;

    // dict-errata.lisp:581 (ichiran/custom:load-custom-data '(:extra) t)
    load_custom_data(ctx, &[CustomDataKey::Extra], true).await?;
    Ok(())
}

/// Port of `ichiran/dict:add-new-sense*` (`dict-errata.lisp:155`).
///
/// Convenience wrapper around [`super::add_new_sense::add_new_sense`]:
/// wraps the single `pos` in a 1-element positions list and forwards
/// `glosses`.
pub async fn add_new_sense_star_(
    ctx: &KaniranContext,
    seq: i32,
    pos: &str,
    glosses: &[String],
) -> Result<Option<(i32, i32)>, sqlx::Error> {
    // dict-errata.lisp:156 (add-new-sense seq (list pos) glosses)
    let positions = [pos.to_string()];
    add_new_sense(ctx, seq, &positions, glosses).await
}

/// Port of `ichiran/dict:add-gloss` (`dict-errata.lisp:158`).
///
/// Appends `texts` as new gloss rows on the sense at `(seq, ord)`.
/// Each new gloss receives the next `ord` after the current max;
/// duplicates against the existing `gloss.text` set are skipped.
pub async fn add_gloss(
    ctx: &KaniranContext,
    seq: i32,
    ord: i32,
    texts: &[&str],
) -> Result<(), sqlx::Error> {
    // dict-errata.lisp:159 (query (:select 'id :from 'sense …) :single)
    let sense_id: i32 = sqlx::query_scalar("SELECT id FROM sense WHERE seq = $1 AND ord = $2")
        .bind(seq)
        .bind(ord)
        .fetch_one(&ctx.pool)
        .await?;
    // dict-errata.lisp:160 (select-dao 'gloss (:= 'sense-id sense-id) (:desc :ord))
    let glosses: Vec<Gloss> =
        sqlx::query_as("SELECT * FROM gloss WHERE sense_id = $1 ORDER BY ord DESC")
            .bind(sense_id)
            .fetch_all(&ctx.pool)
            .await?;
    // dict-errata.lisp:161 (glosses-text (mapcar 'text glosses))
    let glosses_text: Vec<&str> = glosses.iter().map(|g| g.text.as_str()).collect();
    // dict-errata.lisp:162 (max-ord (if glosses (1+ (ord (car glosses))) 0))
    let mut max_ord = match glosses.first() {
        Some(g) => g.ord + 1,
        None => 0,
    };
    // dict-errata.lisp:163-166 (loop for new-text in texts unless (find …) do (make-dao 'gloss …) (incf max-ord))
    for new_text in texts {
        if glosses_text.iter().any(|g| *g == *new_text) {
            continue;
        }
        sqlx::query("INSERT INTO gloss (sense_id, text, ord) VALUES ($1, $2, $3)")
            .bind(sense_id)
            .bind(new_text)
            .bind(max_ord)
            .execute(&ctx.pool)
            .await?;
        max_ord += 1;
    }
    Ok(())
}

/// Port of `ichiran/dict:set-common` (`dict-errata.lisp:168`).
///
/// Updates the `common` field on every row in `table` matching
/// `(seq, text)` to `common` (`None` writes SQL NULL).
pub async fn set_common(
    ctx: &KaniranContext,
    table: KaniReadingTable,
    seq: i32,
    text: &str,
    common: Option<i32>,
) -> Result<(), sqlx::Error> {
    let sql = format!(
        "UPDATE {} SET common = $1 WHERE seq = $2 AND text = $3",
        table.table_name()
    );
    sqlx::query(&sql)
        .bind(common)
        .bind(seq)
        .bind(text)
        .execute(&ctx.pool)
        .await?;
    Ok(())
}

/// Port of `ichiran/dict:add-deha-ja-readings` (`dict-errata.lisp:173`).
///
/// For every conjugation derived from seq 2089020 (the copula `で[は]`)
/// whose kana reading starts with `では`, mints a sibling reading
/// where `では` is rewritten to `じゃ`. Same rewrite is applied to the
/// matching `conj_source_reading` rows; `source_text` itself is
/// rewritten only when it starts with `では`.
/// `(concatenate 'string "じゃ" (subseq deha 2))`.
fn rewrite_deha_to_ja(s: &str) -> String {
    let split = s.char_indices().nth(2).map(|(b, _)| b).unwrap_or(s.len());
    format!("じゃ{}", &s[split..])
}

pub async fn add_deha_ja_readings(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    // dict-errata.lisp:174-178 (query (:select 'conj.seq 'kt.text :distinct …))
    let deha_list: Vec<(i32, String)> = sqlx::query_as(
        r#"SELECT DISTINCT conj.seq, kt.text
           FROM conjugation AS conj, kana_text AS kt
           WHERE conj."from" = 2089020
             AND kt.seq = conj.seq
             AND kt.text LIKE 'では%'"#,
    )
    .fetch_all(&ctx.pool)
    .await?;
    // dict-errata.lisp:179-181 (loop for (seq deha) … do (add-reading seq ja))
    for (seq, deha) in &deha_list {
        let ja = rewrite_deha_to_ja(deha);
        add_reading(ctx, *seq, &ja, None, true, None).await?;
    }

    // dict-errata.lisp:183-187 (query (:select 'csr.conj-id 'csr.text 'csr.source-text :distinct? — NO …))
    let deha_src_reading: Vec<(i32, String, String)> = sqlx::query_as(
        r#"SELECT csr.conj_id, csr.text, csr.source_text
           FROM conjugation AS conj, conj_source_reading AS csr
           WHERE conj."from" = 2089020
             AND csr.conj_id = conj.id
             AND csr.text LIKE 'では%'"#,
    )
    .fetch_all(&ctx.pool)
    .await?;
    // dict-errata.lisp:188-196 (loop for (conj-id text source-text) … unless jsr do (make-dao …))
    for (conj_id, text, source_text) in &deha_src_reading {
        let ja = rewrite_deha_to_ja(text);
        // dict-errata.lisp:190 (select-dao 'conj-source-reading (:and (:= 'conj-id conj-id) (:= 'text ja) (:= 'source-text source-text)))
        let jsr: Vec<ConjSourceReading> = sqlx::query_as(
            "SELECT * FROM conj_source_reading \
             WHERE conj_id = $1 AND text = $2 AND source_text = $3",
        )
        .bind(conj_id)
        .bind(&ja)
        .bind(source_text)
        .fetch_all(&ctx.pool)
        .await?;
        if !jsr.is_empty() {
            continue;
        }
        // dict-errata.lisp:194-196 (:source-text (if (alexandria:starts-with-subseq "では" source-text) (concatenate … "じゃ" (subseq … 2)) source-text))
        let new_source_text = if source_text.starts_with("では") {
            rewrite_deha_to_ja(source_text)
        } else {
            source_text.clone()
        };
        // dict-errata.lisp:192-196 (make-dao 'conj-source-reading :conj-id conj-id :text ja :source-text …)
        sqlx::query(
            "INSERT INTO conj_source_reading (conj_id, text, source_text) \
             VALUES ($1, $2, $3)",
        )
        .bind(conj_id)
        .bind(&ja)
        .bind(&new_source_text)
        .execute(&ctx.pool)
        .await?;
    }
    Ok(())
}

/// Port of `ichiran/dict:delete-conjugation` (`dict-errata.lisp:198`).
///
/// Drops every `conjugation` row from `from` to `seq` via `via`, plus
/// its `conj-prop` and `conj-source-reading` children, then drops the
/// target `entry` itself unless it's a root entry or still has other
/// conjugations.
pub async fn delete_conjugation(
    ctx: &KaniranContext,
    seq: i32,
    from: i32,
    via: Option<i32>,
) -> Result<(), sqlx::Error> {
    // dict-errata.lisp:199-203 (query-dao 'conjugation (:select '* :from 'conjugation :where …))
    // `:===` is null-safe equality; map to `IS NOT DISTINCT FROM`.
    let conj_ids: Vec<i32> = sqlx::query_scalar(
        r#"SELECT id FROM conjugation
           WHERE seq = $1 AND "from" = $2 AND via IS NOT DISTINCT FROM $3"#,
    )
    .bind(seq)
    .bind(from)
    .bind(via)
    .fetch_all(&ctx.pool)
    .await?;
    // dict-errata.lisp:204 (entry (get-dao 'entry seq))
    let entry: Entry = sqlx::query_as("SELECT * FROM entry WHERE seq = $1")
        .bind(seq)
        .fetch_one(&ctx.pool)
        .await?;
    // dict-errata.lisp:205 (when conj …) — bail when no matching rows.
    if conj_ids.is_empty() {
        return Ok(());
    }
    // dict-errata.lisp:207-209 (delete-entry (not (or (root-p entry) (select-dao 'conjugation …))))
    let other_conj: Option<i32> = sqlx::query_scalar(
        "SELECT id FROM conjugation WHERE seq = $1 AND NOT (id = ANY($2)) LIMIT 1",
    )
    .bind(seq)
    .bind(&conj_ids)
    .fetch_optional(&ctx.pool)
    .await?;
    let delete_entry = !(entry.root_p || other_conj.is_some());
    // dict-errata.lisp:211-213 (query (:delete-from …))
    sqlx::query("DELETE FROM conj_prop WHERE conj_id = ANY($1)")
        .bind(&conj_ids)
        .execute(&ctx.pool)
        .await?;
    sqlx::query("DELETE FROM conj_source_reading WHERE conj_id = ANY($1)")
        .bind(&conj_ids)
        .execute(&ctx.pool)
        .await?;
    sqlx::query("DELETE FROM conjugation WHERE id = ANY($1)")
        .bind(&conj_ids)
        .execute(&ctx.pool)
        .await?;
    // dict-errata.lisp:214-215 (when delete-entry (delete-dao entry))
    if delete_entry {
        sqlx::query("DELETE FROM entry WHERE seq = $1")
            .bind(seq)
            .execute(&ctx.pool)
            .await?;
    }
    Ok(())
}

/// Port of `ichiran/dict:remove-hiragana-nokanji` (`dict-errata.lisp:217`).
///
/// Finds every `kana_text` row carrying `nokanji = TRUE` whose `text`
/// is pure hiragana, then clears the `primary_nokanji` flag on every
/// entry that owns one of those rows and still has the flag set.
pub async fn remove_hiragana_nokanji(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    // dict-errata.lisp:218-219 (remove-if-not … (select-dao 'kana-text 'nokanji))
    let all_nokanji_kts: Vec<KanaText> = sqlx::query_as("SELECT * FROM kana_text WHERE nokanji")
        .fetch_all(&ctx.pool)
        .await?;
    let kts: Vec<KanaText> = all_nokanji_kts
        .into_iter()
        .filter(|kt| test_word(&kt.text, CharClass::Hiragana))
        .collect();
    // dict-errata.lisp:220 (select-dao 'entry (:and (:in 'seq (:set (mapcar #'seq kts))) 'primary-nokanji))
    if kts.is_empty() {
        return Ok(());
    }
    let seqs: Vec<i32> = kts.iter().map(|kt| kt.seq).collect();
    let entries: Vec<Entry> =
        sqlx::query_as("SELECT * FROM entry WHERE seq = ANY($1) AND primary_nokanji")
            .bind(&seqs)
            .fetch_all(&ctx.pool)
            .await?;
    // dict-errata.lisp:221-222 (setf (slot-value entry 'primary-nokanji) nil) (update-dao entry)
    for mut entry in entries {
        entry.primary_nokanji = false;
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

/// Port of `ichiran/dict:set-primary-nokanji` (`dict-errata.lisp:224`).
///
/// Looks up the entry by `seq` and writes `value` into its
/// `primary_nokanji` column.
pub async fn set_primary_nokanji(
    ctx: &KaniranContext,
    seq: i32,
    value: bool,
) -> Result<(), sqlx::Error> {
    // dict-errata.lisp:225 (get-dao 'entry seq)
    let mut entry: Entry = sqlx::query_as("SELECT * FROM entry WHERE seq = $1")
        .bind(seq)
        .fetch_one(&ctx.pool)
        .await?;
    // dict-errata.lisp:226 (setf (slot-value entry 'primary-nokanji) value)
    entry.primary_nokanji = value;
    // dict-errata.lisp:227 (update-dao entry)
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
    Ok(())
}

/// Port of `ichiran/dict:rearrange-readings` (`dict-errata.lisp:229`).
///
/// Reassigns `ord` so every row whose `text` starts with `prefix`
/// lands first (0..offset) and the rest follow (offset..n), preserving
/// the original ascending-`ord` order inside each group.
pub async fn rearrange_readings(
    ctx: &KaniranContext,
    seq: i32,
    table: KaniReadingTable,
    prefix: &str,
) -> Result<(), sqlx::Error> {
    let tname = table.table_name();
    // dict-errata.lisp:232-234 (query (:select (:count 'id) … (:like 'text (:|| prefix "%"))) :single)
    let pattern = format!("{prefix}%");
    let offset: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(id) FROM {tname} WHERE seq = $1 AND text LIKE $2",
    ))
    .bind(seq)
    .bind(&pattern)
    .fetch_one(&ctx.pool)
    .await?;
    // dict-errata.lisp:235 (with cnt1 = -1 and cnt2 = (1- offset))
    let mut cnt1: i32 = -1;
    let mut cnt2: i32 = (offset as i32) - 1;
    // dict-errata.lisp:236 (select-dao table (:= 'seq seq) 'ord) — sorted by ord asc
    let rows = sqlx::query(&format!(
        "SELECT id, text FROM {tname} WHERE seq = $1 ORDER BY ord",
    ))
    .bind(seq)
    .fetch_all(&ctx.pool)
    .await?;
    for row in rows {
        let id: i32 = row.try_get("id")?;
        let text: String = row.try_get("text")?;
        // dict-errata.lisp:237-238 (if (alexandria:starts-with-subseq prefix (text kt)) (incf cnt1) (incf cnt2))
        let new_ord = if text.starts_with(prefix) {
            cnt1 += 1;
            cnt1
        } else {
            cnt2 += 1;
            cnt2
        };
        // dict-errata.lisp:239 (setf (slot-value kt 'ord) new-ord) (update-dao kt)
        sqlx::query(&format!("UPDATE {tname} SET ord = $1 WHERE id = $2"))
            .bind(new_ord)
            .bind(id)
            .execute(&ctx.pool)
            .await?;
    }
    Ok(())
}

/// Port of `ichiran/dict:rearrange-readings-conj` (`dict-errata.lisp:241`).
///
/// Runs [`rearrange_readings`] for `seq`, then runs it again for every
/// distinct `conjugation.seq` whose `from = seq`.
pub async fn rearrange_readings_conj(
    ctx: &KaniranContext,
    seq: i32,
    table: KaniReadingTable,
    prefix: &str,
) -> Result<(), sqlx::Error> {
    // dict-errata.lisp:242 (rearrange-readings seq table prefix)
    rearrange_readings(ctx, seq, table, prefix).await?;
    // dict-errata.lisp:243 (dolist (seq (query (:select 'seq :distinct :from 'conjugation :where (:= 'from seq)) :column)) …)
    let conj_seqs: Vec<i32> =
        sqlx::query_scalar(r#"SELECT DISTINCT seq FROM conjugation WHERE "from" = $1"#)
            .bind(seq)
            .fetch_all(&ctx.pool)
            .await?;
    for conj_seq in conj_seqs {
        // dict-errata.lisp:244 (rearrange-readings seq table prefix)
        rearrange_readings(ctx, conj_seq, table, prefix).await?;
    }
    Ok(())
}

/// Port of `ichiran/dict:add-primary-nokanji` (`dict-errata.lisp:251`).
///
/// Sets the entry's `primary_nokanji` flag and marks every matching
/// `kana_text` row (same `seq`, exact `text` = `reading`) as
/// `nokanji = TRUE`.
pub async fn add_primary_nokanji(
    ctx: &KaniranContext,
    seq: i32,
    reading: &str,
) -> Result<(), sqlx::Error> {
    // dict-errata.lisp:252 (set-primary-nokanji seq t)
    set_primary_nokanji(ctx, seq, true).await?;
    // dict-errata.lisp:253-255 (do-readings 'kana-text seq reading (kt) (setf (slot-value kt 'nokanji) t) (update-dao kt))
    let kts: Vec<KanaText> = sqlx::query_as("SELECT * FROM kana_text WHERE seq = $1 AND text = $2")
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

/// Port of `ichiran/dict:get-all-readings` (`dict-errata.lisp:257`).
///
/// Returns the union of `kanji_text.text` and `kana_text.text` for a
/// single entry (by `seq`) as a list of strings.
pub async fn get_all_readings(ctx: &KaniranContext, seq: i32) -> Result<Vec<String>, sqlx::Error> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT text FROM kanji_text WHERE seq = $1 \
         UNION \
         SELECT text FROM kana_text WHERE seq = $1",
    )
    .bind(seq)
    .fetch_all(&ctx.pool)
    .await?;
    Ok(rows.into_iter().map(|(t,)| t).collect())
}

/// Port of `ichiran/dict:add-gozaimasu-conjs` (`dict-errata.lisp:263`).
///
/// For seqs 1612690 (ございます) and 2253080 (ござる), mints six
/// conjugations (せん, した, して, しょう, したら, したり) by
/// rewriting the trailing `す` of each reading via [`apply_patch`].
/// When `reset` is `Some(true)`, every existing conjugation `from`
/// these seqs is dropped first via [`delete_conjugation`].
///
/// [`apply_patch`]: super::apply_patch::apply_patch
/// [`delete_conjugation`]: super::errata::delete_conjugation
pub async fn add_gozaimasu_conjs(
    ctx: &KaniranContext,
    reset: Option<bool>,
) -> Result<(), sqlx::Error> {
    // dict-errata.lisp:263 (&aux (seqs '(1612690 2253080)))
    let seqs: [i32; 2] = [1612690, 2253080];
    // dict-errata.lisp:264-266 (when reset (loop for conj in (select-dao 'conjugation (:in 'from (:set seqs))) do (delete-conjugation …)))
    if matches!(reset, Some(true)) {
        let rows: Vec<(i32, i32)> =
            sqlx::query_as(r#"SELECT seq, "from" FROM conjugation WHERE "from" = ANY($1)"#)
                .bind(seqs.as_slice())
                .fetch_all(&ctx.pool)
                .await?;
        for (conj_seq, conj_from) in rows {
            delete_conjugation(ctx, conj_seq, conj_from, None).await?;
        }
    }
    // dict-errata.lisp:267-278 (loop for seq in seqs … do (loop for (conj suf) in '(…) do (add-conj …)))
    let forms: [((i32, &str, Option<bool>, Option<bool>), &str); 6] = [
        ((1, "exp", Some(true), None), "せん"),
        ((2, "exp", None, None), "した"),
        ((3, "exp", None, None), "して"),
        ((9, "exp", None, None), "しょう"),
        ((11, "exp", None, None), "したら"),
        ((12, "exp", None, None), "したり"),
    ];
    for seq in &seqs {
        // dict-errata.lisp:268 (readings = (get-all-readings seq))
        let readings = get_all_readings(ctx, *seq).await?;
        for (conj_opts, suf) in &forms {
            // dict-errata.lisp:276-278 (loop for reading in readings collect (list reading (apply-patch reading (cons suf "す"))))
            let reading_map: Vec<(String, String)> = readings
                .iter()
                .map(|reading| (reading.clone(), apply_patch(reading, (suf, "す"))))
                .collect();
            // dict-errata.lisp:276 (add-conj seq conj reading-map)
            add_conj(ctx, *seq, *conj_opts, &reading_map).await?;
        }
    }
    Ok(())
}

/// Port of `ichiran/dict:conjugate-da` (`dict-errata.lisp:280`).
///
/// Ensures the entry at `seq` carries a `pos = "cop-da"` sense-prop —
/// adding it and running [`conjugate_entry_outer`] when it's missing.
/// `seq` defaults to 2089020 (the copula `だ`) when `None`.
///
/// [`conjugate_entry_outer`]: super::conjugate_entry_outer::conjugate_entry_outer
pub async fn conjugate_da(ctx: &KaniranContext, seq: Option<i32>) -> Result<(), sqlx::Error> {
    let seq = seq.unwrap_or(2089020);
    // dict-errata.lisp:283 (unless (select-dao 'sense-prop (:and (:= 'seq seq) (:= 'tag "pos") (:= 'text "cop-da"))) …)
    let existing: Option<i32> = sqlx::query_scalar(
        "SELECT id FROM sense_prop \
         WHERE seq = $1 AND tag = 'pos' AND text = 'cop-da' LIMIT 1",
    )
    .bind(seq)
    .fetch_optional(&ctx.pool)
    .await?;
    if existing.is_some() {
        return Ok(());
    }
    // dict-errata.lisp:284 (add-sense-prop seq 0 "pos" "cop-da")
    add_sense_prop(ctx, seq, 0, "pos", "cop-da").await?;
    // dict-errata.lisp:285 (conjugate-entry-outer seq)
    conjugate_entry_outer(ctx, seq, None, None, None).await?;
    Ok(())
}

/// Port of `ichiran/dict:add-errata-feb17` (`dict-errata.lisp:584`).
///
/// Applies the February-2017 batch of JMdict overrides: `common`
/// adjustments on `kana-text` / `kanji-text` rows, sense-prop
/// tweaks, `primary-nokanji` flips, and two new readings.
pub async fn add_errata_feb17(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    set_common(ctx, KaniReadingTable::Kana, 2136890, "とする", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2100900, "となる", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1006200, "すべき", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2683060, "なのです", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2683060, "なんです", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1001200, "おい", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1001200, "おおい", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1441840, "伝い", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1409140, "身体", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 2830705, "身体", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1009040, "どきっと", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2261300, "するべき", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2215430, "には", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2210140, "まい", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2192950, "なさい", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2143350, "かも", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2106890, "そのよう", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2084040, "すれば", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2036080, "うつ", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1922760, "という", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1632520, "ふん", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1631750, "がる", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1394680, "そういう", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1208840, "かつ", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1011430, "べき", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1008340, "である", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1007960, "ちんちん", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1301230, "さんなん", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1311010, "氏", Some(20)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1311010, "うじ", Some(20)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 2101130, "氏", Some(21)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1155180, "いない", Some(10)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1609450, "思いきって", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1309320, "思いきる", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1312880, "メス", Some(15)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1312880, "めす", None).await?;
    set_common(
        ctx,
        KaniReadingTable::Kana,
        2061540,
        "ぶっちゃける",
        Some(0),
    )
    .await?;
    set_common(ctx, KaniReadingTable::Kana, 2034520, "ですら", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1566210, "いずれ", Some(9)).await?;

    delete_sense_prop(ctx, 2021030, "misc", "uk").await?;
    delete_sense_prop(ctx, 1586730, "misc", "uk").await?;
    delete_sense_prop(ctx, 1441400, "misc", "uk").await?;

    add_sense_prop(ctx, 1569590, 0, "misc", "uk").await?;
    add_sense_prop(ctx, 1590540, 0, "misc", "uk").await?;
    add_sense_prop(ctx, 1430200, 0, "misc", "uk").await?;

    set_primary_nokanji(ctx, 1374550, false).await?;
    set_primary_nokanji(ctx, 1591900, false).await?;
    set_primary_nokanji(ctx, 1000230, false).await?;
    set_primary_nokanji(ctx, 1517810, false).await?;
    set_primary_nokanji(ctx, 1585410, false).await?;

    add_reading(ctx, 1029150, "えっち", None, true, None).await?;
    add_reading(ctx, 1363740, "マネ", None, true, None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1363740, "マネ", Some(9)).await?;

    set_common(ctx, KaniReadingTable::Kanji, 1000420, "彼の", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 2219590, "元", Some(10)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2219590, "もと", Some(10)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1394760, "さほど", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1529560, "なし", Some(10)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1436830, "ていない", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1057580, "さぼる", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1402420, "走り", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1402420, "はしり", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1209540, "かる", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1244840, "かる", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1280640, "こうは", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1158960, "いほう", Some(0)).await?;

    delete_sense_prop(ctx, 2122310, "pos", "prt").await?;
    Ok(())
}

/// Port of `ichiran/dict:add-errata-jan18` (`dict-errata.lisp:660`).
///
/// Applies the January-2018 batch of JMdict overrides: `common`
/// adjustments on `kana-text` / `kanji-text` rows, sense-prop
/// tweaks, two `primary-nokanji` flips, and one new reading.
pub async fn add_errata_jan18(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    set_common(ctx, KaniReadingTable::Kanji, 2067770, "等", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2067770, "ら", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1242230, "近よる", Some(38)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1315120, "字", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1315120, "あざ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1315130, "字", Some(5)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1315130, "じ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1005530, "しっくり", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1554850, "りきむ", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2812650, "ゲー", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2083340, "やろう", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2083340, "やろ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1008730, "とろ", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1457840, "ないかい", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2829697, "いかん", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2157330, "おじゃま", Some(9)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1199800, "かいらん", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2719580, "いらん", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1808040, "めちゃ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1277450, "すき", Some(9)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1006460, "ズレる", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1522290, "本会議", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1522290, "ほんかいぎ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1220570, "きたい", Some(10)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1221020, "きたい", Some(11)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2083990, "ならん", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 2518850, "切れ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1221900, "基地外", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1379380, "せいと", Some(10)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1203280, "外に", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1383690, "後継ぎ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2083600, "すまん", Some(0)).await?;

    add_reading(ctx, 1384840, "キレ", Some(0), true, None).await?;

    delete_sense_prop(ctx, 1303400, "misc", "uk").await?;
    delete_sense_prop(ctx, 1434020, "misc", "uk").await?;
    delete_sense_prop(ctx, 1196520, "misc", "uk").await?;
    delete_sense_prop(ctx, 1414190, "misc", "uk").await?;

    add_sense_prop(ctx, 1188380, 0, "misc", "uk").await?;
    add_sense_prop(ctx, 1258330, 0, "misc", "uk").await?;
    add_sense_prop(ctx, 2217330, 0, "misc", "uk").await?;

    set_primary_nokanji(ctx, 1258330, false).await?;
    set_primary_nokanji(ctx, 1588930, false).await?;

    add_sense_prop(ctx, 1445160, 0, "pos", "ctr").await?;
    Ok(())
}

/// Port of `ichiran/dict:add-errata-mar18` (`dict-errata.lisp:711`).
///
/// Applies the March-2018 batch of JMdict overrides: `common`
/// adjustments on `kana-text` / `kanji-text` rows, sense-prop tweaks,
/// one `primary-nokanji` flip, and a new sense for な.
pub async fn add_errata_mar18(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    set_primary_nokanji(ctx, 1565440, false).await?;

    set_common(ctx, KaniReadingTable::Kana, 1207610, "かける", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1236100, "強いる", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1236100, "しいる", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1451750, "おんなじ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 2068330, "事故る", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1579260, "きのう", Some(2)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 2644980, "柔らかさ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2644980, "やわらかさ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2083610, "ベタ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2083610, "べた", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1119610, "ベタ", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1004840, "コロコロ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1257040, "ケンカ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1633840, "ごとき", Some(0)).await?;

    add_sense_prop(ctx, 1238460, 0, "misc", "uk").await?;

    delete_sense_prop(ctx, 1896380, "misc", "uk").await?;
    delete_sense_prop(ctx, 1157000, "misc", "uk").await?;
    delete_sense_prop(ctx, 1576360, "misc", "uk").await?;

    add_sense_prop(ctx, 1468900, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1241380, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1241380, 1, "pos", "ctr").await?;

    add_new_sense_star_(ctx, 2029110, "prt", &["indicates な-adjective".to_string()]).await?;
    Ok(())
}

/// Port of `ichiran/dict:add-errata-aug18` (`dict-errata.lisp:743`).
///
/// Applies the August-2018 batch of JMdict overrides: `common`
/// adjustments on `kana-text` / `kanji-text` rows, sense-prop tweaks,
/// a new reading for オケ together with its `primary-nokanji` flag,
/// and a `misc` "uk" delete.
pub async fn add_errata_aug18(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    set_common(ctx, KaniReadingTable::Kana, 1593870, "さらう", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2141690, "ふざけんな", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1214770, "かん", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1214770, "観", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 2082780, "意味深", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2209180, "とて", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1574640, "ロバ", Some(0)).await?;

    add_reading(ctx, 2722640, "オケ", None, true, None).await?;
    add_primary_nokanji(ctx, 2722640, "オケ").await?;
    set_common(ctx, KaniReadingTable::Kana, 2722640, "オケ", Some(0)).await?;
    add_sense_prop(ctx, 2722640, 0, "misc", "uk").await?;
    add_sense_prop(ctx, 1527140, 0, "misc", "uk").await?;

    add_sense_prop(ctx, 1208870, 0, "misc", "uk").await?;

    delete_sense_prop(ctx, 1598660, "misc", "uk").await?;
    Ok(())
}

/// Port of `ichiran/dict:add-errata-jan19` (`dict-errata.lisp:763`).
///
/// Applies the January-2019 batch of JMdict overrides: `common`
/// adjustments, a `misc` "uk" add and delete, three new readings
/// (two with companion `add-conj-reading` calls), five proverb
/// readings dropped, one `primary-nokanji` flip, and one `arch` drop.
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
    delete_reading(
        ctx,
        2833961,
        "梅は食うとも核食うな、中に天神寝てござる",
        None,
    )
    .await?;
    delete_reading(ctx, 2834318, "二人は伴侶、三人は仲間割れ", None).await?;
    delete_reading(ctx, 2834363, "墨は餓鬼に磨らせ、筆は鬼に持たせよ", None).await?;

    set_primary_nokanji(ctx, 1631830, false).await?;

    delete_sense_prop(ctx, 1270350, "misc", "arch").await?;
    Ok(())
}

/// Port of `ichiran/dict:add-errata-apr19` (`dict-errata.lisp:788`).
///
/// Applies the April-2019 batch of JMdict corrections (common-flag
/// adjustments, sense-prop tweaks, reading deletes).
pub async fn add_errata_apr19(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    set_common(ctx, KaniReadingTable::Kanji, 1538750, "癒やす", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1538750, "癒す", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1538750, "いやす", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2147610, "いなくなる", Some(0)).await?;

    set_common(ctx, KaniReadingTable::Kana, 1346290, "マス", Some(37)).await?;
    add_sense_prop(ctx, 1346290, 3, "misc", "uk").await?;
    set_primary_nokanji(ctx, 1346290, true).await?;

    set_primary_nokanji(ctx, 1409110, false).await?;

    delete_reading(ctx, 2081610, "スレ違", None).await?;
    set_primary_nokanji(ctx, 2081610, false).await?;

    add_sense_prop(ctx, 1615340, 0, "misc", "uk").await?;
    add_sense_prop(ctx, 1658480, 0, "pos", "ctr").await?;
    Ok(())
}

/// Port of `ichiran/dict:add-errata-jan20` (`dict-errata.lisp:807`).
///
/// Applies the January-2020 batch of JMdict corrections (common-flag
/// adjustments, sense-prop tweaks, reading add/delete, conj readings).
pub async fn add_errata_jan20(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    add_reading(ctx, 2839843, "うえをしたへ", None, true, None).await?;
    delete_reading(ctx, 2839843, "うえをしたえ", None).await?;
    add_reading(ctx, 1593170, "コケる", None, true, None).await?;
    add_conj_reading(ctx, 1593170, "コケる").await?;

    add_sense_prop(ctx, 1565100, 0, "misc", "uk").await?;
    delete_sense_prop(ctx, 1632980, "misc", "uk").await?;
    delete_sense_prop(ctx, 1715710, "misc", "uk").await?;
    set_common(ctx, KaniReadingTable::Kana, 1715710, "みたところ", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2841254, "からって", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2028950, "とは", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1292400, "再開", Some(13)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1292400, "さいかい", Some(13)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1306200, "しよう", Some(10)).await?;
    set_common(
        ctx,
        KaniReadingTable::Kana,
        2056930,
        "つまらなさそう",
        Some(0),
    )
    .await?;
    set_common(ctx, KaniReadingTable::Kanji, 1164710, "一段落", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1570220, "すくむ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1352130, "うえ", Some(1)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1502390, "もん", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2780660, "もん", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2653620, "がち", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2653620, "ガチ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1135480, "モノ", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1003000, "カラカラ", Some(0)).await?;

    set_primary_nokanji(ctx, 1495000, false).await?;

    add_sense_prop(ctx, 2510160, 0, "misc", "obsc").await?;

    add_sense_prop(ctx, 1468900, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1469050, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1469050, 1, "pos", "ctr").await?;
    add_sense_prop(ctx, 1469050, 2, "pos", "ctr").await?;
    add_sense_prop(ctx, 1284270, 0, "pos", "ctr").await?;

    delete_sense_prop(ctx, 1245280, "pos", "adj-no").await?;
    delete_sense_prop(ctx, 1392570, "pos", "adj-no").await?;

    add_sense_prop(ctx, 1429740, 0, "pos", "suf").await?;
    add_sense_prop(ctx, 1429740, 1, "pos", "n").await?;
    delete_sense_prop(ctx, 2647210, "pos", "suf").await?;
    Ok(())
}

/// Port of `ichiran/dict:add-errata-apr20` (`dict-errata.lisp:852`).
///
/// Applies the April-2020 batch of JMdict corrections (common-flag
/// adjustments, sense-prop tweaks, a new sense).
pub async fn add_errata_apr20(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    set_common(ctx, KaniReadingTable::Kana, 1225940, "アリ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1568080, "ふくろう", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1025450, "ウイルス", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1025450, "ウィルス", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1004320, "こうゆう", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1580290, "おとめ", Some(0)).await?;

    add_sense_prop(ctx, 1219510, 0, "misc", "uk").await?;
    add_sense_prop(ctx, 1616370, 0, "misc", "uk").await?;

    add_new_sense_star_(ctx, 1315920, "ctr", &["hours (period of)".to_string()]).await?;

    add_sense_prop(ctx, 1220540, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1220540, 3, "pos", "ctr").await?;
    add_sense_prop(ctx, 1220540, 4, "pos", "ctr").await?;
    add_sense_prop(ctx, 1220540, 5, "pos", "ctr").await?;
    add_sense_prop(ctx, 1220540, 6, "pos", "ctr").await?;

    add_sense_prop(ctx, 2842087, 0, "pos", "ctr").await?;
    set_common(ctx, KaniReadingTable::Kana, 2842087, "パー", Some(0)).await?;

    add_sense_prop(ctx, 1956530, 1, "pos", "n").await?;
    Ok(())
}

/// Port of `ichiran/dict:add-errata-jul20` (`dict-errata.lisp:878`).
///
/// Applies the July-2020 batch of JMdict corrections (common-flag
/// adjustments, sense-prop tweaks, reading rearrangement).
pub async fn add_errata_jul20(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    set_common(ctx, KaniReadingTable::Kana, 2101130, "し", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1982860, "代", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1367020, "ひとけ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1002190, "おしり", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2085020, "もどき", Some(0)).await?;

    set_primary_nokanji(ctx, 1756600, false).await?;

    add_reading(ctx, 2217330, "ワイ", None, true, None).await?;
    add_primary_nokanji(ctx, 2217330, "ワイ").await?;
    add_sense_prop(ctx, 2217330, 0, "misc", "uk").await?;
    delete_sense_prop(ctx, 2217330, "misc", "arch").await?;

    add_reading(ctx, 1103270, "ぱんつ", None, true, None).await?;

    add_sense_prop(ctx, 1586290, 0, "misc", "uk").await?;

    add_sense_prop(ctx, 1257260, 0, "misc", "uk").await?;

    rearrange_readings_conj(ctx, 1980880, KaniReadingTable::Kanji, "かけ直").await?;
    Ok(())
}

/// Port of `ichiran/dict:add-errata-jan21` (`dict-errata.lisp:901`).
///
/// Applies the January-2021 batch of JMdict corrections (common-flag
/// adjustments, sense-prop tweaks, reading replacements).
pub async fn add_errata_jan21(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    set_common(ctx, KaniReadingTable::Kana, 2124820, "コロナウイルス", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2846738, "なん", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 2083720, "っぽい", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1012980, "遣る", None).await?;

    add_sense_prop(ctx, 1411570, 0, "pos", "vs").await?;
    add_sense_prop(ctx, 1613860, 0, "pos", "ctr").await?;
    add_sense_prop(ctx, 1613860, 1, "pos", "ctr").await?;

    add_sense_prop(ctx, 2679820, 0, "misc", "uk").await?;
    delete_sense_prop(ctx, 1426680, "misc", "uk").await?;
    add_sense_prop(ctx, 1590390, 0, "misc", "uk").await?;

    delete_sense_prop(ctx, 1215240, "pos", "ctr").await?;
    add_sense_prop(ctx, 2145410, 0, "pos", "ctr").await?;

    replace_reading(
        ctx,
        2847494,
        "いきはよいといかえりはこわい",
        "いきはよいよいかえりはこわい",
    )
    .await?;
    Ok(())
}

/// Port of `ichiran/dict:add-errata-may21` (`dict-errata.lisp:921`).
///
/// Applies the May-2021 batch of JMdict corrections (common-flag
/// adjustments, sense-prop deletes, new readings).
pub async fn add_errata_may21(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    add_reading(ctx, 1089590, "どんまい", None, true, None).await?;

    set_common(ctx, KaniReadingTable::Kana, 2848303, "てか", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1979920, "貴方", None).await?;

    delete_sense_prop(ctx, 1547720, "misc", "uk").await?;
    delete_sense_prop(ctx, 1495770, "misc", "uk").await?;
    delete_sense_prop(ctx, 2611890, "misc", "uk").await?;
    Ok(())
}

/// Port of `ichiran/dict:add-errata-jan22` (`dict-errata.lisp:932`).
///
/// Applies the January-2022 batch of JMdict corrections (common-flag
/// adjustments, sense-prop tweaks, new readings, conj readings).
pub async fn add_errata_jan22(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    add_reading(ctx, 1566420, "ハメる", None, true, None).await?;
    add_conj_reading(ctx, 1566420, "ハメる").await?;

    add_reading(ctx, 1161240, "いっかねん", None, true, None).await?;

    set_common(ctx, KaniReadingTable::Kana, 2008650, "そうした", None).await?;
    add_sense_prop(ctx, 1188270, 0, "pos", "n").await?;
    delete_sense_prop(ctx, 1188270, "pos", "pn").await?;

    delete_sense_prop(ctx, 1240530, "pos", "ctr").await?;

    add_sense_prop(ctx, 1247260, 0, "pos", "n-suf").await?;

    set_common(
        ctx,
        KaniReadingTable::Kana,
        1001840,
        "おにいちゃん",
        Some(0),
    )
    .await?;
    set_common(ctx, KaniReadingTable::Kana, 1806840, "がいそう", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1639750, "こだから", None).await?;
    Ok(())
}

/// Port of `ichiran/dict:add-errata-dec23` (`dict-errata.lisp:954`).
///
/// Applies the December-2023 batch of JMdict corrections (common-flag
/// adjustments, sense-prop add/delete pairs).
pub async fn add_errata_dec23(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    add_sense_prop(ctx, 1180540, 0, "misc", "uk").await?;
    delete_sense_prop(ctx, 2854117, "misc", "uk").await?;
    delete_sense_prop(ctx, 2859257, "misc", "uk").await?;
    delete_sense_prop(ctx, 1198890, "misc", "uk").await?;

    add_sense_prop(ctx, 2826371, 0, "misc", "uk").await?;
    delete_sense_prop(ctx, 2826371, "misc", "rare").await?;

    set_common(ctx, KaniReadingTable::Kana, 1625620, "はいかん", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1625610, "はいかん", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1681460, "はいかん", None).await?;

    set_common(ctx, KaniReadingTable::Kanji, 2855480, "乙女", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 2855480, "おとめ", Some(0)).await?;

    set_common(ctx, KaniReadingTable::Kana, 1930050, "バラす", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1582460, "ないかい", None).await?;
    set_common(ctx, KaniReadingTable::Kana, 1202300, "かいが", Some(0)).await?;

    set_common(ctx, KaniReadingTable::Kanji, 1328740, "狩る", Some(0)).await?;

    set_common(ctx, KaniReadingTable::Kana, 1009610, "にも", Some(0)).await?;
    Ok(())
}

/// Port of `ichiran/dict:add-errata-jan25` (`dict-errata.lisp:986`).
///
/// Applies the January-2025 batch of JMdict corrections (common-flag
/// adjustments, sense-prop tweaks, reading replacements/deletes).
pub async fn add_errata_jan25(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    delete_reading(ctx, 2028930, "ヶ", Some(KaniReadingTable::Kana)).await?;
    delete_reading(ctx, 2028930, "ケ", Some(KaniReadingTable::Kana)).await?;

    delete_sense_prop(ctx, 1138570, "pos", "ctr").await?;
    add_sense_prop(ctx, 1138570, 1, "pos", "ctr").await?;
    add_sense_prop(ctx, 1138570, 2, "pos", "ctr").await?;
    add_sense_prop(ctx, 1138570, 3, "pos", "ctr").await?;

    set_common(ctx, KaniReadingTable::Kana, 1001120, "うんち", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kana, 1511600, "かたかな", Some(0)).await?;
    set_common(
        ctx,
        KaniReadingTable::Kana,
        1056400,
        "サウンドトラック",
        Some(0),
    )
    .await?;
    set_common(ctx, KaniReadingTable::Kana, 1510640, "へん", Some(5)).await?;

    replace_reading(
        ctx,
        2860664,
        "こどもはおやのせなかをみてそだう",
        "こどもはおやのせなかをみてそだつ",
    )
    .await?;
    replace_reading_conj(
        ctx,
        2863544,
        KaniReadingTable::Kana,
        "みぎにでるのは",
        "みぎにでるものは",
    )
    .await?;
    Ok(())
}

/// Port of `ichiran/dict:add-errata-jan26` (`dict-errata.lisp:1005`).
///
/// Applies the January-2026 batch of JMdict corrections (sense-prop
/// deletes, common-flag adjustments, `primary-nokanji` flips).
pub async fn add_errata_jan26(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    delete_sense_prop(ctx, 1236660, "misc", "uk").await?;
    delete_sense_prop(ctx, 2859279, "misc", "uk").await?;
    delete_sense_prop(ctx, 1591420, "misc", "uk").await?;

    set_primary_nokanji(ctx, 1502390, false).await?;
    set_common(ctx, KaniReadingTable::Kana, 1502390, "モノ", Some(0)).await?;

    set_common(ctx, KaniReadingTable::Kana, 1392580, "まえ", Some(5)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1502920, "分かつ", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1169130, "引分ける", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1326660, "取り計らう", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1340420, "出来", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1340430, "出来", Some(9)).await?;

    set_common(ctx, KaniReadingTable::Kanji, 1589320, "思い", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1281000, "考え", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 2862681, "閉まり", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1989500, "開き", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1985020, "気づき", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1180130, "押し", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1216850, "含み", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1231760, "居座り", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1236660, "恐れ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1238660, "驚き", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1259890, "見直し", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1297250, "作り", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1304480, "残り", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1327090, "守り", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1327100, "守り", Some(9)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1396550, "狙い", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1403130, "増やし", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1535930, "問い", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1548390, "頼り", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1609560, "勝ち", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1954660, "聞こえ", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1497960, "負け", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1502940, "分かり", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1917220, "分かれ", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1221250, "帰り", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1351280, "笑い", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1352300, "上げ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1354720, "乗り", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1502990, "分け", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1630270, "脅かし", None).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1456130, "読み", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1403020, "騒ぎ", Some(0)).await?;
    set_common(ctx, KaniReadingTable::Kanji, 1659120, "受け", Some(0)).await?;
    Ok(())
}

/// Port of `ichiran/dict:*skip-words*` (`dict-errata.lisp:1155`).
///
/// Seqs of words that aren't really words (suffixes, etc.); a candidate
/// whose seq-set intersects this list scores 0 and is dropped.
pub static SKIP_WORDS: &[i32] = &[
    2822120, // ても良い
    2013800, // ちゃう
    2108590, // とく
    2029040, // ば
    2428180, // い
    2654250, // た
    2561100, // うまいな
    2210270, // ませんか
    2210710, // ましょうか
    2257550, // ない
    2210320, // ません
    2017560, // たい
    2394890, // とる
    2194000, // であ
    2568000, // れる/られる
    2537250, // しようとする
    2760890, // 三箱
    2831062, // てる
    2831063, // てく
    2029030, // ものの
    2568020, // せる
    900000,  // たそう
    2827357, // まう
];

/// Port of `ichiran/dict:add-errata-counters` (`dict-errata.lisp:1068`).
///
/// Applies counter-word corrections to JMdict: reading edits, new
/// senses/glosses, and `pos`=`ctr` sense-prop tagging.
pub async fn add_errata_counters(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    delete_reading(ctx, 1299960, "さんかい", None).await?;
    // dict-errata.lisp:1070 (mapc 'set-reading (select-dao 'kanji-text (:= 'seq 1299960)))
    let mut kanji_rows: Vec<KanjiText> = sqlx::query_as("SELECT * FROM kanji_text WHERE seq = $1")
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

/// Port of `ichiran/dict:*final-prt*` (`dict-errata.lisp:1182`).
///
/// Seqs of words that only have meaning when they're the final
/// segment of a path.
pub static FINAL_PRT: &[i32] = &[
    2017770, // かい
    // 1008450 // では (commented out upstream)
    2425930, // なの
    // 2780660 // もの (commented out upstream)
    2130430, // け / っけ
    2029130, // ぞ
    2834812, // ぜ
    2718360, // がな
    2201380, // わい
    2722170, // のう
    2751630, // かいな
];

/// Port of `ichiran/dict:*semi-final-prt*` (`dict-errata.lisp:1196`).
///
/// Particles that are final but also have other uses; the final-prt
/// list plus さ/し/な/ね/わ.
pub fn semi_final_prt() -> &'static [i32] {
    static CACHE: OnceLock<Vec<i32>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            let mut out: Vec<i32> = FINAL_PRT.to_vec();
            out.extend_from_slice(&[
                2029120, // さ
                2086640, // し
                2029110, // な
                2029080, // ね
                2029100, // わ
            ]);
            out
        })
        .as_slice()
}

/// Port of `ichiran/dict:*copulae*` (`dict-errata.lisp:1205`).
///
/// JMdict seqs treated as copulae (e.g. だ) during scoring.
pub static COPULAE: &[i32] = &[
    2089020, // だ
            // 2755350 // じゃない (commented out upstream)
];

/// Port of `ichiran/dict:*non-final-prt*` (`dict-errata.lisp:1209`).
///
/// Particles that don't get the final-position score bonus; the only
/// entry is `ん` (2139720).
pub static NON_FINAL_PRT: &[i32] = &[
    2139720, // ん
];

/// Port of `ichiran/dict:*no-kanji-break-penalty*` (`dict-errata.lisp:1214`).
///
/// Seqs of words that are exempt from the kanji-break penalty.
pub static NO_KANJI_BREAK_PENALTY: &[i32] = &[
    1169870, // 飲む
    1198360, // 会議
    1277450, // 好き
    2028980, // で
    1423000, // 着る
    1164690, // 一段
    1587040, // 言う
    2827864, // なので
];

/// Port of `ichiran/dict:*force-kanji-break*` (`dict-errata.lisp:1226`).
///
/// Literal substrings that force the segmenter to break at a kanji
/// boundary.
pub static FORCE_KANJI_BREAK: &[&str] = &["です"];

/// Port of `ichiran/dict:*no-kanji-break*` (`dict-errata.lisp:1229`).
///
/// Literal substrings that do not cause a kanji break in the segmenter.
pub static NO_KANJI_BREAK: &[&str] = &["日置"];

/// Port of `ichiran/dict:*skip-conj-forms*` (`dict-errata.lisp:1310`).
///
/// Conjugation forms whose hits the segmenter drops.
pub static SKIP_CONJ_FORMS: &[ConjForm] = &[
    ConjForm::Triple(FormToken::Int(10), FormToken::Bool(true), FormToken::Any),
    ConjForm::Triple(
        FormToken::Int(3),
        FormToken::Bool(true),
        FormToken::Bool(true),
    ),
    ConjForm::Quadruple(
        FormToken::Str("vs-s"),
        FormToken::Int(5),
        FormToken::Any,
        FormToken::Any,
    ),
];

/// Port of `ichiran/dict:*weak-conj-forms*` (`dict-errata.lisp:1316`).
///
/// Conjugation forms whose hits the segmenter scores down rather than
/// drops outright (the "weak" tier).
pub static WEAK_CONJ_FORMS: &[ConjForm] = &[
    ConjForm::Triple(FormToken::Int(51), FormToken::Any, FormToken::Any), // +conj-adjective-stem+
    ConjForm::Triple(FormToken::Int(52), FormToken::Any, FormToken::Any), // +conj-negative-stem+
    ConjForm::Triple(FormToken::Int(53), FormToken::Any, FormToken::Any), // +conj-causative-su+
    ConjForm::Triple(FormToken::Int(54), FormToken::Any, FormToken::Any), // +conj-adjective-literary+
    ConjForm::Triple(FormToken::Int(9), FormToken::Bool(true), FormToken::Any),
];

/// Port of `ichiran/dict:errata-conj-description-hook` (`dict-errata.lisp:1242`).
///
/// Adds the five ichiran-internal conjugation types
/// (`+conj-adverbial+`=50 … `+conj-adjective-literary+`=54) to the
/// conj-id → description map after it is loaded from conj.csv.
pub fn errata_conj_description_hook(hash: &mut HashMap<i32, String>) {
    hash.insert(50, "Adverbial".to_string()); // +conj-adverbial+
    hash.insert(51, "Adjective Stem".to_string()); // +conj-adjective-stem+
    hash.insert(52, "Negative Stem".to_string()); // +conj-negative-stem+
    hash.insert(53, "Causative (~su)".to_string()); // +conj-causative-su+
    hash.insert(54, "Old/literary form".to_string()); // +conj-adjective-literary+
}

/// Port of `ichiran/dict:errata-conj-rules-hook` (`dict-errata.lisp:1250`).
///
/// Post-load fixups on the conjugation-rules hash (pos-id → list of
/// `conjugation-rule`): adds adverbial / stem / literary rules for
/// `adj-i` and `adj-ix`, a `v5aru` irregular, patches negative-formal
/// okurigana for `v1`/`v1-s` and the negative-conditional for `v5u`,
/// drops `vs-s` potential forms, and (over every entry) rewrites godan
/// causative-su and adds a negative-stem rule for `v5*`.
pub fn errata_conj_rules_hook(hash: &mut HashMap<i32, Vec<ConjugationRule>>) {
    // dict-errata.lisp:1251 — adj-i: adverbial / adjective-stem / literary
    let pos = get_pos_index("adj-i").expect("adj-i in *pos-index*");
    let rules = [
        ConjugationRule {
            pos,
            conj: 50,
            neg: false,
            fml: false,
            onum: 1,
            stem: 1,
            okuri: "く".to_string(),
            euphr: String::new(),
            euphk: String::new(),
        },
        ConjugationRule {
            pos,
            conj: 51,
            neg: false,
            fml: false,
            onum: 1,
            stem: 1,
            okuri: String::new(),
            euphr: String::new(),
            euphk: String::new(),
        },
        ConjugationRule {
            pos,
            conj: 54,
            neg: false,
            fml: false,
            onum: 1,
            stem: 1,
            okuri: "き".to_string(),
            euphr: String::new(),
            euphk: String::new(),
        },
    ];
    for rule in rules {
        hash.entry(pos).or_default().insert(0, rule);
    }

    // dict-errata.lisp:1261 — adj-ix: same as adj-i with euphr "よ"
    let pos = get_pos_index("adj-ix").expect("adj-ix in *pos-index*");
    let rules = [
        ConjugationRule {
            pos,
            conj: 50,
            neg: false,
            fml: false,
            onum: 1,
            stem: 1,
            okuri: "く".to_string(),
            euphr: "よ".to_string(),
            euphk: String::new(),
        },
        ConjugationRule {
            pos,
            conj: 51,
            neg: false,
            fml: false,
            onum: 1,
            stem: 1,
            okuri: String::new(),
            euphr: "よ".to_string(),
            euphk: String::new(),
        },
        ConjugationRule {
            pos,
            conj: 54,
            neg: false,
            fml: false,
            onum: 1,
            stem: 1,
            okuri: "き".to_string(),
            euphr: "よ".to_string(),
            euphk: String::new(),
        },
    ];
    for rule in rules {
        hash.entry(pos).or_default().insert(0, rule);
    }

    // dict-errata.lisp:1271 — v5aru irregular
    let pos = get_pos_index("v5aru").expect("v5aru in *pos-index*");
    hash.entry(pos).or_default().insert(
        0,
        ConjugationRule {
            pos,
            conj: 3,
            neg: false,
            fml: false,
            onum: 2,
            stem: 1,
            okuri: "り".to_string(),
            euphr: String::new(),
            euphk: String::new(),
        },
    );

    // dict-errata.lisp:1276 — fix non-past negative formal for v1 v1-s
    let posi = ["v1", "v1-s"].map(|tag| get_pos_index(tag).expect("v1/v1-s in *pos-index*"));
    for pos in posi {
        if let Some(rules) = hash.get_mut(&pos) {
            for rule in rules.iter_mut() {
                if rule.conj == 1 && rule.fml && rule.neg {
                    rule.okuri = "ません".to_string();
                }
            }
        }
    }

    // dict-errata.lisp:1282 — fix incorrect negative conditional of v5u
    let pos = get_pos_index("v5u").expect("v5u in *pos-index*");
    if let Some(rules) = hash.get_mut(&pos) {
        for rule in rules.iter_mut() {
            if rule.conj == 11 && !rule.fml && rule.neg {
                rule.okuri = "わなかったら".to_string();
            }
        }
    }

    // dict-errata.lisp:1287 — remove potential forms of vs-s
    let pos = get_pos_index("vs-s").expect("vs-s in *pos-index*");
    hash.entry(pos).or_default().retain(|r| r.conj != 5);

    // dict-errata.lisp:1290 (maphash) — add conj-negative-stem for godan verbs
    for (key, val) in hash.iter_mut() {
        let pos = get_pos(*key);
        // dict-errata.lisp:1294 — conj 7 / onum 2 → causative-su, onum 1
        for r in val.iter_mut() {
            if r.conj == 7 && r.onum == 2 {
                r.conj = 53;
                r.onum = 1;
            }
        }
        // dict-errata.lisp:1298 (alexandria:starts-with-subseq "v5" pos)
        if pos.is_some_and(|p| p.starts_with("v5")) {
            // dict-errata.lisp:1299 — first non-formal negative (conj 1) rule
            if let Some(mut new_rule) = val.iter().find(|r| r.conj == 1 && r.neg && !r.fml).cloned()
            {
                let len = new_rule.okuri.chars().count();
                if len > 2 {
                    new_rule.conj = 52;
                    new_rule.okuri = new_rule.okuri.chars().take(len - 2).collect();
                    val.insert(0, new_rule);
                }
            }
        }
    }
}

/// Port of `ichiran/dict:skip-by-conj-data` (`dict-errata.lisp:1336`).
///
/// True iff `conj_data` is non-empty and every prop matches
/// [`SKIP_CONJ_FORMS`] (empty list → false).
pub fn skip_by_conj_data(conj_data: &[ConjData]) -> bool {
    !conj_data.is_empty() && conj_data.iter().all(matches)
}

fn matches(cd: &ConjData) -> bool {
    cd.prop
        .as_ref()
        .map(|prop| test_conj_prop(prop, SKIP_CONJ_FORMS))
        .unwrap_or(false)
}

/// Port of `ichiran/dict:test-conj-prop` (`dict-errata.lisp:1336`).
///
/// Predicate: does [`ConjProp`] match any element of `forms`? A
/// 3-element form matches `(conj-type neg fml)`, a 4-element form adds
/// `pos`; a `:any` cell always matches.
pub fn test_conj_prop(prop: &ConjProp, forms: &[ConjForm]) -> bool {
    forms.iter().any(|form| match form {
        ConjForm::Triple(ct, neg, fml) => {
            match_conj_type(*ct, prop.conj_type)
                && match_bool(*neg, prop.neg)
                && match_bool(*fml, prop.fml)
        }
        ConjForm::Quadruple(pos, ct, neg, fml) => {
            match_pos(*pos, &prop.pos)
                && match_conj_type(*ct, prop.conj_type)
                && match_bool(*neg, prop.neg)
                && match_bool(*fml, prop.fml)
        }
    })
}

fn match_conj_type(token: FormToken, value: i32) -> bool {
    match token {
        FormToken::Any => true,
        FormToken::Int(n) => n == value,
        _ => false,
    }
}

fn match_pos(token: FormToken, value: &str) -> bool {
    match token {
        FormToken::Any => true,
        FormToken::Str(s) => s == value,
        _ => false,
    }
}

fn match_bool(token: FormToken, value: Option<bool>) -> bool {
    match token {
        FormToken::Any => true,
        FormToken::Bool(b) => value == Some(b),
        FormToken::DbNull => value.is_none(),
        _ => false,
    }
}

#[cfg(test)]
mod tests;
