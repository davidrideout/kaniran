//! Port of `ichiran/dict:add-gozaimasu-conjs` (`dict-errata.lisp:263`).
//!
//! For seqs 1612690 (ございます) and 2253080 (ござる), mints six
//! conjugations (せん, した, して, しょう, したら, したり) by
//! rewriting the trailing `す` of each reading via [`apply_patch`].
//! When `reset` is `Some(true)`, every existing conjugation `from`
//! these seqs is dropped first via [`delete_conjugation`].
//!
//! [`apply_patch`]: super::apply_patch::apply_patch
//! [`delete_conjugation`]: super::delete_conjugation::delete_conjugation

use super::add_conj::add_conj;
use super::apply_patch::apply_patch;
use super::delete_conjugation::delete_conjugation;
use super::get_all_readings::get_all_readings;
use crate::conn::kani_context::KaniranContext;

pub async fn add_gozaimasu_conjs(
    ctx: &KaniranContext,
    reset: Option<bool>,
) -> Result<(), sqlx::Error> {
    // dict-errata.lisp:263 (&aux (seqs '(1612690 2253080)))
    let seqs: [i32; 2] = [1612690, 2253080];
    // dict-errata.lisp:264-266 (when reset (loop for conj in (select-dao 'conjugation (:in 'from (:set seqs))) do (delete-conjugation …)))
    if matches!(reset, Some(true)) {
        let rows: Vec<(i32, i32)> = sqlx::query_as(
            r#"SELECT seq, "from" FROM conjugation WHERE "from" = ANY($1)"#,
        )
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
