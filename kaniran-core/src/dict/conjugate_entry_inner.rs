//! Port of `ichiran/dict:conjugate-entry-inner` (`dict-load.lisp:316`).
//!
//! Build the conjugation matrix for one entry. For every pos tag on the
//! entry (or from `as_posi` when supplied) the function looks up the
//! conj rules, fetches the conjugatable kanji/kana readings, applies
//! [`construct_conjugation`], and slots each result into a 2×2 array
//! indexed by `[neg][fml]` under the `(pos-id, conj-id)` key.
//!
//! Diverges from the upstream lambda list `(seq &key conj-types as-posi)`
//! only by taking `&KaniranContext` for the database handle, replacing
//! the upstream dynamic `*connection*` per [`crate::conn::kani_context`],
//! and by representing the keyword arguments as `Option<…>`
//! (`None` = absent → upstream defaults: `nil`).

use std::collections::HashMap;

use crate::conn::kani_context::KaniranContext;

use super::_star_do_not_conjugate_star_::DO_NOT_CONJUGATE;
use super::construct_conjugation::construct_conjugation;
use super::get_conj_rules::get_conj_rules;
use super::get_pos_index::get_pos_index;

/// One row in a `ConjMatrix` cell — the 5-element list
/// `(conj-text kanji-flag reading ord onum)` pushed at
/// `dict-load.lisp:338-341`. Example value:
/// `("食べた".to_string(), 1, "食べる".to_string(), 0, 1)` — past-plain
/// form of 食べる from its `ord=0` kanji reading, rule `onum=1`.
pub type ConjMatrixEntry = (String, i32, String, i32, i32);

/// `(pos-id, conj-id) → 2×2 array` where index `[neg][fml]` holds the
/// list of [`ConjMatrixEntry`] rows produced for that combination.
/// Mirrors the upstream `(make-hash-table :test 'equal)` /
/// `(make-array '(2 2) :initial-element nil)` shape at
/// `dict-load.lisp:319/337`.
pub type ConjMatrix = HashMap<(i32, i32), [[Vec<ConjMatrixEntry>; 2]; 2]>;

pub async fn conjugate_entry_inner(
    ctx: &KaniranContext,
    seq: i32,
    conj_types: Option<&[i32]>,
    as_posi: Option<&[String]>,
) -> Result<ConjMatrix, sqlx::Error> {
    // dict-load.lisp:317-318 — (or as-posi (query (:select 'text :distinct ...)))
    let posi: Vec<String> = match as_posi {
        Some(p) => p.to_vec(),
        None => {
            let rows: Vec<(String,)> = sqlx::query_as(
                "SELECT DISTINCT text FROM sense_prop WHERE tag = 'pos' AND seq = $1",
            )
            .bind(seq)
            .fetch_all(&ctx.pool)
            .await?;
            rows.into_iter().map(|(t,)| t).collect()
        }
    };

    // dict-load.lisp:319 — (make-hash-table :test 'equal)
    let mut conj_matrix: ConjMatrix = HashMap::new();

    for pos in &posi {
        // dict-load.lisp:321 — (get-pos-index pos)
        let pos_id = match get_pos_index(pos) {
            Some(id) => id,
            None => continue,
        };
        // dict-load.lisp:322 — (get-conj-rules pos-id)
        let rules = get_conj_rules(pos_id);
        // dict-load.lisp:323 — (and rules (not (member pos *do-not-conjugate* :test 'equal)))
        if rules.is_empty() || DO_NOT_CONJUGATE.contains(&pos.as_str()) {
            continue;
        }
        // dict-load.lisp:325-328 — (:union (:select 'text 'ord 1 :from 'kanji-text ...) (:select 'text 'ord 0 :from 'kana-text ...))
        let readings: Vec<(String, i32, i32)> = sqlx::query_as(
            "SELECT text, ord, 1 AS kanji_flag FROM kanji_text \
             WHERE seq = $1 AND conjugate_p \
             UNION \
             SELECT text, ord, 0 AS kanji_flag FROM kana_text \
             WHERE seq = $1 AND conjugate_p",
        )
        .bind(seq)
        .fetch_all(&ctx.pool)
        .await?;

        for (reading, ord, kanji_flag) in &readings {
            for rule in &rules {
                let conj_id = rule.conj;
                // dict-load.lisp:331-332 — (or (not conj-types) (member conj-id conj-types))
                if let Some(types) = conj_types {
                    if !types.contains(&conj_id) {
                        continue;
                    }
                }
                let key = (pos_id, conj_id);
                let conj_text = construct_conjugation(reading, rule);
                // dict-load.lisp:335-337 — (unless (gethash key conj-matrix) (setf … (make-array '(2 2) :initial-element nil)))
                let cell = conj_matrix.entry(key).or_default();
                // dict-load.lisp:338-341 — (push (list conj-text kanji-flag reading ord (cr-onum rule)) (aref … (if (cr-neg rule) 1 0) (if (cr-fml rule) 1 0)))
                let neg_idx = if rule.neg { 1 } else { 0 };
                let fml_idx = if rule.fml { 1 } else { 0 };
                cell[neg_idx][fml_idx].insert(
                    0,
                    (conj_text, *kanji_flag, reading.clone(), *ord, rule.onum),
                );
            }
        }
    }
    Ok(conj_matrix)
}
