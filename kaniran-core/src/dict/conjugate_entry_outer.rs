//! Port of `ichiran/dict:conjugate-entry-outer` (`dict-load.lisp:344`).
//!
//! Drive [`insert_conjugation`] over every cell of the conjugation
//! matrix built by [`conjugate_entry_inner`]: for each
//! `(pos-id, conj-id)` key it walks the 2×2 matrix in row-major
//! (`[neg][fml]`) order, drops rows whose conj-text matches one of the
//! entry's original readings, and calls `insert-conjugation` with
//! `neg` / `fml` collapsed to `:null` when the entry has no rows in
//! the corresponding row/column of the matrix.
//!
//! Diverges from the upstream lambda list
//! `(seq* &key via conj-types as-posi)` only by taking
//! `&KaniranContext` for the database handle, replacing the upstream
//! dynamic `*connection*` per [`crate::conn::kani_context`], and by
//! representing the keyword arguments as `Option<…>` (`None` = absent
//! → upstream defaults: `nil`).

use crate::conn::kani_context::KaniranContext;

use super::conjugate_entry_inner::conjugate_entry_inner;
use super::get_all_readings::get_all_readings;
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

    // dict-load.lisp:349 — iterate hash-key (pos-id conj-id) / hash-value matrix
    for ((pos_id, conj_id), matrix) in &conj_matrix {
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
