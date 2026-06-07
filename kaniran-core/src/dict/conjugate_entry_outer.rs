//! Port of `ichiran/dict:conjugate-entry-outer` (`dict-load.lisp:344`).
//!
//! Drives [`insert_conjugation`] over every cell of the conjugation
//! matrix built by [`conjugate_entry_inner`], writing the new
//! conjugated-reading rows for an entry.

use crate::conn::kani_context::KaniranContext;

use super::conjugate_entry_inner::conjugate_entry_inner;
use super::errata::get_all_readings;
use super::get_pos::get_pos;
use super::insert_conjugation::insert_conjugation;
use super::next_seq::next_seq;

pub async fn conjugate_entry_outer(
    ctx: &KaniranContext,
    seq_star: i32,
    via: Option<i32>,
    conj_types: Option<&[i32]>,
    as_posi: Option<&[String]>,
) -> Result<(), sqlx::Error> {
    // dict-load.lisp:345 — (or via seq*)
    let seq = via.unwrap_or(seq_star);
    // dict-load.lisp:346 — (conjugate-entry-inner seq :conj-types conj-types :as-posi as-posi)
    let conj_matrix = conjugate_entry_inner(ctx, seq, conj_types, as_posi).await?;
    // dict-load.lisp:347 — (get-all-readings seq)
    let original_readings = get_all_readings(ctx, seq).await?;
    // dict-load.lisp:348 — (next-seq)
    let mut next_seq_val = next_seq(ctx).await?;

    // dict-load.lisp:349 — iterate hash-key (pos-id conj-id) / hash-value matrix.
    // Sort by (pos_id, conj_id) so iteration is deterministic. Order matters
    // downstream
    let mut entries: Vec<_> = conj_matrix.iter().collect();
    entries.sort_by_key(|((pos_id, conj_id), _)| (*pos_id, *conj_id));
    for ((pos_id, conj_id), matrix) in entries {
        // dict-load.lisp:350-351 — ignore-neg / ignore-fml flags
        let ignore_neg = matrix[1][0].is_empty() && matrix[1][1].is_empty();
        let ignore_fml = matrix[0][1].is_empty() && matrix[1][1].is_empty();
        // dict-load.lisp:352 — (get-pos pos-id)
        let pos = get_pos(*pos_id)
            .expect("conjugate-entry-outer: pos-id in conj-matrix not in *pos-by-index*");
        // dict-load.lisp:353-365 — loop for ii from 0 below 4
        for ii in 0..4 {
            let neg_flag = ii >= 2;
            let fml_flag = ii % 2 != 0;
            // dict-load.lisp:357-358 — (row-major-aref matrix ii)
            let cell = &matrix[ii / 2][ii % 2];
            // dict-load.lisp:356-358 — (remove-if (lambda (item) (member (car item) original-readings :test 'equal)))
            let readings: Vec<_> = cell
                .iter()
                .filter(|item| !original_readings.contains(&item.0))
                .cloned()
                .collect();
            if readings.is_empty() {
                continue;
            }
            // dict-load.lisp:360-364 — insert-conjugation … :neg (if ignore-neg :null neg) :fml (if ignore-fml :null fml)
            let inserted = insert_conjugation(
                ctx,
                &readings,
                next_seq_val,
                seq_star,
                pos,
                *conj_id,
                if ignore_neg { None } else { Some(neg_flag) },
                if ignore_fml { None } else { Some(fml_flag) },
                via,
            )
            .await?;
            if inserted {
                next_seq_val += 1;
            }
        }
    }
    Ok(())
}
