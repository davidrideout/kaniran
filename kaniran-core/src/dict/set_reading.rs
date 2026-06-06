//! Port of `ichiran/dict:set-reading` (gf — `dict-load.lisp:487`)
//! and the two methods on `kanji-text` (`dict-load.lisp:490`) and
//! `kana-text` (`dict-load.lisp:507`).
//!
//! Picks the best cross-reading for a kanji/kana row and writes it
//! back to the row's `best_kana` / `best_kanji` column. Restricted
//! readings (`re_restr` JMdict tags) gate the candidate list.

use crate::conn::kani_context::KaniranContext;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kanji_text_dao::KanjiText;

pub enum SetReadingObj<'a> {
    Kanji(&'a mut KanjiText),
    Kana(&'a mut KanaText),
}

pub async fn set_reading(
    ctx: &KaniranContext,
    obj: SetReadingObj<'_>,
) -> Result<(), sqlx::Error> {
    match obj {
        SetReadingObj::Kanji(obj) => set_reading_kanji(ctx, obj).await,
        SetReadingObj::Kana(obj) => set_reading_kana(ctx, obj).await,
    }
}

async fn set_reading_kanji(
    ctx: &KaniranContext,
    obj: &mut KanjiText,
) -> Result<(), sqlx::Error> {
    let seq = obj.seq;
    let cur_best = obj.best_kana.clone();
    // dict-load.lisp:493 (query (:select 'reading 'text :from 'restricted-readings :where (:= 'seq seq)))
    let restricted: Vec<(String, String)> = sqlx::query_as(
        "SELECT reading, text FROM restricted_readings WHERE seq = $1",
    )
    .bind(seq)
    .fetch_all(&ctx.pool)
    .await?;
    // dict-load.lisp:494 (loop for reading in (select-dao 'kana-text (:= 'seq seq) 'ord))
    let readings: Vec<KanaText> =
        sqlx::query_as("SELECT * FROM kana_text WHERE seq = $1 ORDER BY ord")
            .bind(seq)
            .fetch_all(&ctx.pool)
            .await?;
    for reading in &readings {
        let rtext = &reading.text;
        // dict-load.lisp:496 (for restr = (loop for (rt kt) in restricted when (equal rtext rt) collect kt))
        let restr: Vec<&String> = restricted
            .iter()
            .filter_map(|(rt, kt)| if rt == rtext { Some(kt) } else { None })
            .collect();
        // dict-load.lisp:497-499 (when (and (not (nokanji reading)) (or (not restr) (member (text obj) restr :test 'equal))))
        if !reading.nokanji && (restr.is_empty() || restr.iter().any(|kt| *kt == &obj.text)) {
            // dict-load.lisp:500-501 (unless (equal cur-best (text reading)) (setf (best-kana obj) (text reading)) (update-dao obj))
            if cur_best.as_deref() != Some(rtext.as_str()) {
                sqlx::query("UPDATE kanji_text SET best_kana = $1 WHERE id = $2")
                    .bind(rtext)
                    .bind(obj.id)
                    .execute(&ctx.pool)
                    .await?;
                obj.best_kana = Some(rtext.clone());
            }
            // dict-load.lisp:502 (return-from set-reading)
            return Ok(());
        }
    }
    // dict-load.lisp:503-504 (unless (equal cur-best :null) (setf (best-kana obj) :null) (update-dao obj))
    if cur_best.is_some() {
        sqlx::query("UPDATE kanji_text SET best_kana = NULL WHERE id = $1")
            .bind(obj.id)
            .execute(&ctx.pool)
            .await?;
        obj.best_kana = None;
    }
    Ok(())
}

async fn set_reading_kana(
    ctx: &KaniranContext,
    obj: &mut KanaText,
) -> Result<(), sqlx::Error> {
    // dict-load.lisp:508-509 (when (nokanji obj) (return-from set-reading))
    if obj.nokanji {
        return Ok(());
    }
    let seq = obj.seq;
    let cur_best = obj.best_kanji.clone();
    let rtext = obj.text.clone();
    // dict-load.lisp:513-516 (query (:select 'text :from 'restricted-readings
    //   :where (:and (:= 'seq seq) (:= 'reading rtext))) :column)
    let restricted: Vec<String> = sqlx::query_scalar(
        "SELECT text FROM restricted_readings WHERE seq = $1 AND reading = $2",
    )
    .bind(seq)
    .bind(&rtext)
    .fetch_all(&ctx.pool)
    .await?;
    // dict-load.lisp:517-521 (if restricted
    //   (select-dao 'kanji-text (:and (:= 'seq seq) (:in 'text (:set restricted))) 'ord)
    //   (select-dao 'kanji-text (:= 'seq seq) 'ord))
    let kanji_list: Vec<KanjiText> = if !restricted.is_empty() {
        sqlx::query_as(
            "SELECT * FROM kanji_text WHERE seq = $1 AND text = ANY($2) ORDER BY ord",
        )
        .bind(seq)
        .bind(&restricted)
        .fetch_all(&ctx.pool)
        .await?
    } else {
        sqlx::query_as("SELECT * FROM kanji_text WHERE seq = $1 ORDER BY ord")
            .bind(seq)
            .fetch_all(&ctx.pool)
            .await?
    };
    // dict-load.lisp:522-530 (cond (kanji-list ...) (t ...))
    if let Some(first_kt) = kanji_list.first() {
        let ktext = &first_kt.text;
        if cur_best.as_deref() != Some(ktext.as_str()) {
            sqlx::query("UPDATE kana_text SET best_kanji = $1 WHERE id = $2")
                .bind(ktext)
                .bind(obj.id)
                .execute(&ctx.pool)
                .await?;
            obj.best_kanji = Some(ktext.clone());
        }
    } else if cur_best.is_some() {
        sqlx::query("UPDATE kana_text SET best_kanji = NULL WHERE id = $1")
            .bind(obj.id)
            .execute(&ctx.pool)
            .await?;
        obj.best_kanji = None;
    }
    Ok(())
}
