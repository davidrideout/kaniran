//! Port of `ichiran/dict:load-secondary-conjugations` (`dict-load.lisp:460`).
//!
//! Walks every primary conjugation tagged as a secondary type and drives
//! `conjugate_entry_outer` to build the second-order conjugations (`v5s`
//! posi for the causative-su source form, else `v1`); `from` restricts
//! the candidate set to the given source seqs.

use crate::conn::kani_context::KaniranContext;
use crate::dict::_star_secondary_conjugation_types_from_star_::SECONDARY_CONJUGATION_TYPES_FROM;
use crate::dict::_star_secondary_conjugation_types_star_::SECONDARY_CONJUGATION_TYPES;
use crate::dict::conjugate_entry_outer::conjugate_entry_outer;

// dict-errata.lisp:1239 (defconstant +conj-causative-su+ 53)
const CONJ_CAUSATIVE_SU: i32 = 53;

pub async fn load_secondary_conjugations(
    ctx: &KaniranContext,
    from: Option<&[i32]>,
) -> Result<(), sqlx::Error> {
    // dict-load.lisp:461-473 (sql-compile of the :select conj.from conj.seq conj-prop.conj-type ...)
    let mut sql = String::from(
        "SELECT DISTINCT ON (conj.\"from\", conj.seq) \
         conj.\"from\", conj.seq, conj_prop.conj_type \
         FROM conjugation conj \
         LEFT JOIN conj_prop ON conj.id = conj_prop.conj_id \
         WHERE ",
    );
    let mut conds: Vec<String> = Vec::new();
    if from.is_some() {
        conds.push("conj.\"from\" = ANY($2)".to_string());
    }
    conds.push("conj_prop.conj_type = ANY($1)".to_string());
    conds.push("NOT (conj_prop.pos = ANY(ARRAY['vs-i','vs-s']))".to_string());
    conds.push("conj.via IS NULL".to_string());
    conds.push("(NOT conj_prop.neg OR conj_prop.neg IS NULL)".to_string());
    conds.push("(NOT conj_prop.fml OR conj_prop.fml IS NULL)".to_string());
    sql.push_str(&conds.join(" AND "));

    let to_conj: Vec<(i32, i32, i32)> = {
        let q = sqlx::query_as(&sql).bind(SECONDARY_CONJUGATION_TYPES_FROM);
        let q = if let Some(from_seqs) = from {
            q.bind(from_seqs)
        } else {
            q
        };
        q.fetch_all(&ctx.pool).await?
    };

    // dict-load.lisp:475-480 (loop for cnt from 1 for (seq-from seq conj-type) in to-conj do ...)
    for (idx, (seq_from, seq, conj_type)) in to_conj.iter().enumerate() {
        let as_posi: [String; 1] = [
            if *conj_type == CONJ_CAUSATIVE_SU { "v5s".to_string() } else { "v1".to_string() }
        ];
        conjugate_entry_outer(
            ctx,
            *seq_from,
            Some(*seq),
            Some(SECONDARY_CONJUGATION_TYPES),
            Some(&as_posi),
        )
        .await?;
        let cnt = idx + 1;
        if cnt % 1000 == 0 {
            println!("{cnt} entries processed");
        }
    }
    Ok(())
}
