//! Port of `ichiran/dict:best-kanji-conj` (`dict.lisp:457`).
//!
//! Mirror of [`super::best_kana_conj::best_kana_conj`] with the
//! direction reversed: given a [`KanaText`] reading row, return the
//! kanji surface form to display. The fast path returns the pre-baked
//! `best_kanji` slot when the row carries no conjugations or is itself
//! a root. A second short-circuit returns `None` when the kana row is
//! marked `nokanji` or its parent entry has zero kanji writings.
//! Otherwise the function walks the conjugation chain: it asks
//! [`query_parents_kana`] for each `(parent-kt.id, conj.id)` pair
//! that produced this reading, recurses into the parent (also a
//! kana-text) to obtain its best kanji, and looks up the readings on
//! `conj_source_reading` that derived from that parent kanji. The
//! first reading that satisfies [`kanji_match`] for the input text is
//! returned; if none match, the walk continues into the next parent.
//!
//! Diverges from the upstream lambda list `(obj)` only by taking
//! `&KaniranContext` for the database handle, replacing the upstream
//! dynamic `*connection*` per [`crate::conn::kani_context`]. The Lisp
//! `:null` return sentinel maps to `None`; a successful kanji lookup
//! returns `Some(String)`.

use crate::characters::kanji::kanji_match;
use crate::conn::kani_context::KaniranContext;
use crate::dict::entry_dao::Entry;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::query_parents_kana::query_parents_kana;
use crate::dict::simple_text_class::WordConjugations;

pub async fn best_kanji_conj(
    ctx: &KaniranContext,
    obj: &KanaText,
) -> Result<Option<String>, sqlx::Error> {
    let wc = &obj.state.conjugations;
    // dict.lisp:458-460 ((and (or (not wc) (eql wc :root)) (not (eql (best-kanji obj) :null))))
    let root_or_unset = matches!(wc, None | Some(WordConjugations::Root));
    if root_or_unset && obj.best_kanji.is_some() {
        return Ok(obj.best_kanji.clone());
    }

    // dict.lisp:461-462 ((or (nokanji obj) (= (n-kanji (get-dao 'entry (seq obj))) 0)) :null)
    if obj.nokanji {
        return Ok(None);
    }
    let entry: Entry = sqlx::query_as("SELECT * FROM entry WHERE seq = $1")
        .bind(obj.seq)
        .fetch_one(&ctx.pool)
        .await?;
    if entry.n_kanji == 0 {
        return Ok(None);
    }

    let parents = query_parents_kana(ctx, obj.seq, &obj.text).await?;
    for (pid, cid) in parents {
        // dict.lisp:465 (for parent-bk = (best-kanji-conj (get-dao 'kana-text pid)))
        let parent_kt: KanaText =
            sqlx::query_as("SELECT * FROM kana_text WHERE id = $1")
                .bind(pid)
                .fetch_one(&ctx.pool)
                .await?;
        let parent_bk = Box::pin(best_kanji_conj(ctx, &parent_kt)).await?;
        // dict.lisp:466 (unless (or (eql parent-bk :null)
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

        // dict.lisp:467-470 (query (:select 'text :from 'conj-source-reading
        //   :where (:and (:= 'conj-id cid) (:= 'source-text parent-bk))) :column)
        let readings: Vec<String> = sqlx::query_scalar(
            "SELECT text FROM conj_source_reading \
             WHERE conj_id = $1 AND source_text = $2",
        )
        .bind(cid)
        .bind(&parent_bk)
        .fetch_all(&ctx.pool)
        .await?;
        // dict.lisp:471-473 (some (lambda (reading) (and (kanji-match reading (text obj)) reading)) readings)
        if let Some(hit) = readings.into_iter().find(|r| kanji_match(r, &obj.text)) {
            return Ok(Some(hit));
        }
    }
    Ok(None)
}
