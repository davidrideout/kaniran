//! Transliteration of `ichiran/dict:get-kana-forms*` (`dict-grammar.lisp:17`).
//!
//! Loads every kana-text tied to `seq` — directly or via
//! `conjugation.from = seq` — and tags each row's runtime
//! `state.conjugations`: original-entry rows get `Root`, derived rows
//! get `Ids(...)` from [`get_kana_forms_conj_data_filter`]. Derived
//! rows with an empty filter are dropped (Lisp `when conj-ids`).
//!
//! The `LEFT JOIN ... WHERE conj.from = $1` is Postgres-reduced to an
//! inner join; reproduced verbatim so the dedupe (UNION) shape stays
//! upstream-identical. Upstream's UNION has no `ORDER BY`, so row
//! order is driver-dependent — a known upstream nondeterminism
//! (docs/known_issues.md); reproduced as-is rather than pinned.
//!
//! [`get_kana_forms_conj_data_filter`]: super::get_kana_forms_conj_data_filter::get_kana_forms_conj_data_filter

use crate::conn::kani_context::KaniranContext;
use super::get_conj_data::{get_conj_data, FromOrConjIds};
use super::get_kana_forms_conj_data_filter::get_kana_forms_conj_data_filter;
use super::kana_text_dao::KanaText;
use super::simple_text_class::WordConjugations;

pub async fn get_kana_forms_star_(
    ctx: &KaniranContext,
    seq: i32,
) -> Result<Vec<KanaText>, sqlx::Error> {
    let kts: Vec<KanaText> = sqlx::query_as(
        "SELECT kt.* FROM kana_text kt WHERE kt.seq = $1 \
         UNION \
         SELECT kt.* FROM kana_text kt \
         LEFT JOIN conjugation conj ON conj.seq = kt.seq \
         WHERE conj.\"from\" = $1",
    )
    .bind(seq)
    .fetch_all(&ctx.pool)
    .await?;

    let mut out: Vec<KanaText> = Vec::with_capacity(kts.len());
    for mut kt in kts {
        if kt.seq == seq {
            kt.state.conjugations = Some(WordConjugations::Root);
            out.push(kt);
        } else {
            let cd = get_conj_data(ctx, kt.seq, FromOrConjIds::From(seq), &[]).await?;
            let conj_ids = get_kana_forms_conj_data_filter(&cd);
            if !conj_ids.is_empty() {
                kt.state.conjugations = Some(WordConjugations::Ids(conj_ids));
                out.push(kt);
            }
        }
    }
    Ok(out)
}
