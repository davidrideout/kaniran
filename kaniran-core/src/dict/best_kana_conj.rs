//! Port of `ichiran/dict:best-kana-conj` (`dict.lisp:430`).
//!
//! For a [`KanjiText`] reading row, return the kana surface form to
//! display, walking the conjugation chain back through parent readings
//! when the pre-baked `best_kana` slot doesn't apply.

use crate::characters::kanji::kanji_cross_match;
use crate::characters::kanji::kanji_regex;
use crate::conn::kani_context::KaniranContext;
use crate::dict::kanji_text_dao::KanjiText;
use crate::dict::query_parents_kanji::query_parents_kanji;
use crate::dict::simple_text_class::WordConjugations;

pub async fn best_kana_conj(
    ctx: &KaniranContext,
    obj: &KanjiText,
) -> Result<Option<String>, sqlx::Error> {
    let wc = &obj.state.conjugations;
    // dict.lisp:431-433 ((and (or (not wc) (eql wc :root)) (not (eql (best-kana obj) :null))))
    let root_or_unset = matches!(wc, None | Some(WordConjugations::Root));
    if root_or_unset && obj.best_kana.is_some() {
        return Ok(obj.best_kana.clone());
    }

    let parents = query_parents_kanji(ctx, obj.seq, &obj.text).await?;
    for (pid, cid) in parents {
        // dict.lisp:436 (for parent-kt = (get-dao 'kanji-text pid))
        // fetch_one mirrors upstream: a missing pid would surface as nil
        // from get-dao and crash on the next slot access; propagating
        // the sqlx error preserves that fail-loud stance.
        let parent_kt: KanjiText =
            sqlx::query_as("SELECT * FROM kanji_text WHERE id = $1")
                .bind(pid)
                .fetch_one(&ctx.pool)
                .await?;
        // dict.lisp:437 (for parent-bk = (best-kana-conj parent-kt))
        let parent_bk = Box::pin(best_kana_conj(ctx, &parent_kt)).await?;
        // dict.lisp:438 (unless (or (eql parent-bk :null)
        //                           (and wc (or (eql wc :root) (not (find cid wc))))))
        let skip = parent_bk.is_none()
            || match wc {
                None => false,
                Some(WordConjugations::Root) => true,
                Some(WordConjugations::Ids(ids)) => !ids.contains(&cid),
            };
        if skip {
            continue;
        }
        let parent_bk = parent_bk.expect("checked Some via `skip` above");

        // dict.lisp:439-442 (query (:select 'text :from 'conj-source-reading
        //   :where (:and (:= 'conj-id cid) (:= 'source-text parent-bk))) :column)
        let readings: Vec<String> = sqlx::query_scalar(
            "SELECT text FROM conj_source_reading \
             WHERE conj_id = $1 AND source_text = $2",
        )
        .bind(cid)
        .bind(&parent_bk)
        .fetch_all(&ctx.pool)
        .await?;
        if readings.is_empty() {
            continue;
        }
        if readings.len() == 1 {
            return Ok(Some(readings.into_iter().next().unwrap()));
        }
        // dict.lisp:447 (km = (kanji-cross-match (text parent-kt) parent-bk (text obj)))
        let km = kanji_cross_match(&parent_kt.text, &parent_bk, &obj.text);
        if let Some(km_text) = &km {
            // dict.lisp:448 (find km readings :test 'equal)
            if let Some(hit) = readings.iter().find(|r| *r == km_text) {
                return Ok(Some(hit.clone()));
            }
        }
        // dict.lisp:449-454 (stable-sort by |len(r) - len-km| then first regex match,
        // falling back to (car readings) — SBCL's destructive stable-sort
        // leaves the `readings` variable pointing at the original head
        // cell, whose car is unchanged, so the fallback returns the
        // pre-sort first reading. Capture it before sorting to preserve
        // that.
        let first_reading = readings[0].clone();
        let regex = kanji_regex(&obj.text);
        let len_km = km.as_ref().map(|s| s.chars().count() as i64).unwrap_or(0);
        let mut sorted = readings;
        sorted.sort_by_key(|r| (r.chars().count() as i64 - len_km).abs());
        for rd in &sorted {
            if regex.is_match(rd).unwrap_or(false) {
                return Ok(Some(rd.clone()));
            }
        }
        return Ok(Some(first_reading));
    }
    Ok(None)
}
