//! Port of `ichiran/dict:get-counter-stags` (`dict-counters.lisp:291`).
//!
//! For a set of JMdict sequence numbers, returns two maps —
//! `(stagks, stagrs)` — listing the kanji-restriction and
//! kana-restriction texts attached to any sense whose `pos` is
//! `ctr`. Empty entries are absent from each map (matching the
//! upstream `gethash ... nil` default), so a caller treats "no key"
//! and "empty list" identically — there are no restrictions for that
//! seq.
//!
//! Diverges from the upstream lambda list `(seqs)` by taking
//! `&KaniranContext` as a leading parameter for the database handle
//! per the [`crate::conn::kani_context`] module doc, and by returning
//! a `(HashMap, HashMap)` tuple in place of the Lisp `(cons stagks
//! stagrs)`.

use crate::conn::kani_context::KaniranContext;
use sqlx::Row;
use std::collections::HashMap;

pub type CounterStags = (HashMap<i32, Vec<String>>, HashMap<i32, Vec<String>>);

pub async fn get_counter_stags(
    ctx: &KaniranContext,
    seqs: &[i32],
) -> Result<CounterStags, sqlx::Error> {
    let mut stagks: HashMap<i32, Vec<String>> = HashMap::new();
    let mut stagrs: HashMap<i32, Vec<String>> = HashMap::new();

    let sql = "SELECT sp.seq, sp.text \
               FROM sense_prop sp, sense_prop sp1 \
               WHERE sp.seq = sp1.seq \
                 AND sp.sense_id = sp1.sense_id \
                 AND sp.tag = $1 \
                 AND sp1.tag = 'pos' \
                 AND sp1.text = 'ctr' \
                 AND sp.seq = ANY($2)";

    for row in sqlx::query(sql)
        .bind("stagk")
        .bind(seqs)
        .fetch_all(&ctx.pool)
        .await?
    {
        let seq: i32 = row.get("seq");
        let text: String = row.get("text");
        stagks.entry(seq).or_default().push(text);
    }

    for row in sqlx::query(sql)
        .bind("stagr")
        .bind(seqs)
        .fetch_all(&ctx.pool)
        .await?
    {
        let seq: i32 = row.get("seq");
        let text: String = row.get("text");
        stagrs.entry(seq).or_default().push(text);
    }

    Ok((stagks, stagrs))
}
