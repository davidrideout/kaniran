//! Port of `ichiran/dict:get-counter-readings` (`dict-counters.lisp:332`).
//!
//! Builds the kanji + kana reading lists for every counter seq the
//! cache will populate. Source set:
//! [`crate::dict::get_counter_ids::get_counter_ids`] ∪
//! [`crate::dict::_star_extra_counter_ids_star_::EXTRA_COUNTER_IDS`]
//! minus
//! [`crate::dict::_star_skip_counter_ids_star_::SKIP_COUNTER_IDS`].
//! Per seq, kanji-text rows whose `text` is missing from the seq's
//! `stagk` restriction list (when one exists) are dropped, and kana
//! rows are filtered the same way against `stagr`. Surviving rows
//! are sorted by `ord` ascending. Output value per seq is the pair
//! `(kanji-rows, kana-rows)`.
//!
//! Diverges from the upstream lambda list `()` by taking
//! `&KaniranContext` for the database handle (replacing the upstream
//! dynamic `*connection*` per the [`crate::conn::kani_context`]
//! module doc) and by returning a `HashMap<i32, (Vec<KanjiText>,
//! Vec<KanaText>)>` in place of the Lisp `hash-table` of `(cons
//! kanji-list kana-list)`.

use crate::conn::kani_context::KaniranContext;
use crate::dict::_star_extra_counter_ids_star_::EXTRA_COUNTER_IDS;
use crate::dict::_star_skip_counter_ids_star_::SKIP_COUNTER_IDS;
use crate::dict::get_counter_ids::get_counter_ids;
use crate::dict::get_counter_stags::get_counter_stags;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kanji_text_dao::KanjiText;
use std::collections::{HashMap, HashSet};

pub type CounterReadings = HashMap<i32, (Vec<KanjiText>, Vec<KanaText>)>;

pub async fn get_counter_readings(ctx: &KaniranContext) -> Result<CounterReadings, sqlx::Error> {
    let mut counter_ids: Vec<i32> = get_counter_ids(ctx).await?;
    counter_ids.extend(EXTRA_COUNTER_IDS.iter().copied());
    let skip: HashSet<i32> = SKIP_COUNTER_IDS.iter().copied().collect();
    counter_ids.retain(|id| !skip.contains(id));

    let stags = get_counter_stags(ctx, &counter_ids).await?;

    let kanji_readings: Vec<KanjiText> = sqlx::query_as::<_, KanjiText>(
        "SELECT * FROM kanji_text WHERE seq = ANY($1)",
    )
    .bind(&counter_ids)
    .fetch_all(&ctx.pool)
    .await?;

    let kana_readings: Vec<KanaText> = sqlx::query_as::<_, KanaText>(
        "SELECT * FROM kana_text WHERE seq = ANY($1)",
    )
    .bind(&counter_ids)
    .fetch_all(&ctx.pool)
    .await?;

    let mut hash: CounterReadings = HashMap::new();

    for r in kanji_readings {
        let stagks = stags.0.get(&r.seq);
        let admit = match stagks {
            None => true,
            Some(list) => list.iter().any(|t| t == &r.text),
        };
        if admit {
            hash.entry(r.seq).or_insert_with(|| (Vec::new(), Vec::new())).0.push(r);
        }
    }

    for r in kana_readings {
        let stagrs = stags.1.get(&r.seq);
        let admit = match stagrs {
            None => true,
            Some(list) => list.iter().any(|t| t == &r.text),
        };
        if admit {
            hash.entry(r.seq).or_insert_with(|| (Vec::new(), Vec::new())).1.push(r);
        }
    }

    for (_, (kanji, kana)) in hash.iter_mut() {
        kanji.sort_by_key(|r| r.ord);
        kana.sort_by_key(|r| r.ord);
    }

    Ok(hash)
}
