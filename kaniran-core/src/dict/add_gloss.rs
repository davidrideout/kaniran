//! Port of `ichiran/dict:add-gloss` (`dict-errata.lisp:158`).
//!
//! Appends `texts` as new gloss rows on the sense at `(seq, ord)`.
//! Each new gloss receives the next `ord` after the current max;
//! duplicates against the existing `gloss.text` set are skipped.

use super::gloss_dao::Gloss;
use crate::conn::kani_context::KaniranContext;

pub async fn add_gloss(
    ctx: &KaniranContext,
    seq: i32,
    ord: i32,
    texts: &[&str],
) -> Result<(), sqlx::Error> {
    // dict-errata.lisp:159 (query (:select 'id :from 'sense …) :single)
    let sense_id: i32 = sqlx::query_scalar(
        "SELECT id FROM sense WHERE seq = $1 AND ord = $2",
    )
    .bind(seq)
    .bind(ord)
    .fetch_one(&ctx.pool)
    .await?;
    // dict-errata.lisp:160 (select-dao 'gloss (:= 'sense-id sense-id) (:desc :ord))
    let glosses: Vec<Gloss> = sqlx::query_as(
        "SELECT * FROM gloss WHERE sense_id = $1 ORDER BY ord DESC",
    )
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
