//! Port of `ichiran/dict:get-counter-stags` (`dict-counters.lisp:291`).
//!
//! For a set of JMdict sequence numbers, returns two maps —
//! `(stagks, stagrs)` — listing the kanji-restriction and
//! kana-restriction texts attached to any sense whose `pos` is `ctr`.
//! Seqs with no restrictions are absent from the maps.

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
