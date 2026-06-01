//! Port of `ichiran/dict:load-conjugations` (`dict-load.lisp:447`).
//!
//! Walks every entry whose JMdict POS includes a conjugatable tag and
//! drives [`conjugate_entry_outer`] across it. Skip-list:
//! [`DO_NOT_CONJUGATE_SEQ`]. POS gate: [`POS_WITH_CONJ_RULES`].

use crate::conn::kani_context::KaniranContext;
use crate::dict::_star_do_not_conjugate_seq_star_::DO_NOT_CONJUGATE_SEQ;
use crate::dict::_star_pos_with_conj_rules_star_::POS_WITH_CONJ_RULES;
use crate::dict::conjugate_entry_outer::conjugate_entry_outer;

pub async fn load_conjugations(ctx: &KaniranContext) -> Result<(), sqlx::Error> {
    // dict-load.lisp:449-453 (query (:select 'seq :distinct :from 'sense-prop
    //   :where (:and (:not (:in 'seq (:set *do-not-conjugate-seq*)))
    //                (:= 'tag "pos")
    //                (:in 'text (:set *pos-with-conj-rules*)))) :column)
    let seqs: Vec<i32> = sqlx::query_scalar(
        "SELECT DISTINCT seq FROM sense_prop \
         WHERE NOT (seq = ANY($1)) AND tag = 'pos' AND text = ANY($2)",
    )
    .bind(DO_NOT_CONJUGATE_SEQ)
    .bind(POS_WITH_CONJ_RULES)
    .fetch_all(&ctx.pool)
    .await?;
    // dict-load.lisp:454-457 (loop for cnt from 1 for seq in seqs ...)
    for (idx, seq) in seqs.iter().enumerate() {
        conjugate_entry_outer(ctx, *seq, None, None, None).await?;
        let cnt = idx + 1;
        if cnt % 500 == 0 {
            println!("{cnt} entries processed");
        }
    }
    Ok(())
}
