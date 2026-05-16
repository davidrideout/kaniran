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
//! upstream-identical.
//!
//! ## Divergence: `ORDER BY u.seq DESC, u.id DESC` appended to the UNION
//!
//! Upstream omits an `ORDER BY` and lets the driver return rows in
//! whatever order Postgres produces. On a stock 17.x server the UNION
//! resolves via a `HashAggregate` whose output is deterministic per
//! plan but **not stable across drivers**:
//!
//! - psql, sqlx's `query_as`, and Postmodern's raw `query` read the
//!   wire order directly.
//! - Postmodern's `query-dao` (upstream's loader) accumulates the
//!   result via a cons-prepend in its DAO walker and therefore hands
//!   the caller the **reverse** of the wire order.
//!
//! Verified empirically on 2026-05-15 against seq `1577980` (`いる`):
//! `psql` and a raw `query` return `538276 → 1717832 → 1717833`;
//! `query-dao` returns `1717833 → 1717832 → 538276`; `sqlx::query_as`
//! returns the psql order.
//!
//! That asymmetry is invisible for most seqs (their UNION emits a
//! single row per `text`), but for `1577980` it produces two
//! `text="いられて"` rows (seqs 10235833 / 11156119) plus analogous
//! fan-outs for `いられなければ`, `いられる`, etc.
//! [`init_suffixes_thread`] writes them into the suffix cache via a
//! last-write-wins overwrite (`b.cache.insert(short, vec![…])`), so
//! whichever instance is iterated **last** wins the cache key. With
//! the wire order, sqlx ends on the largest-id row; with Postmodern's
//! reversed order, upstream ends on the smallest-id row. The result
//! is that Rust and upstream cache **different** `kana_text`
//! instances for the same key (`られて`, `られなければ`, `られる`),
//! which downstream `get_suffixes` audit replay surfaces as
//! 459 / 396_209 (~0.12%) row-level mismatches against captured
//! fixtures (see audit log 2026-05-15, patterns `[N=2]` and `[N=4]`).
//!
//! `ORDER BY u.seq DESC, u.id DESC` aligns Rust's iteration with
//! Postmodern's reversed-loader order. Both `seq` and `id` are needed
//! because the cache-key conflict spans rows with the same `seq`
//! but different `id` (id=1717833 and id=1717832 share seq=11156119);
//! ordering by `seq` alone would leave that pair unsorted and let
//! Postgres pick either internally. With `(seq DESC, id DESC)`, the
//! Rust populator visits rows in the same order Postmodern's DAO
//! walker hands them to the upstream populator, so the
//! last-write-wins overwrite lands on the same row instance. Audit
//! drops to 0 for these patterns.
//!
//! The `ctid` pseudo-column would not work — `HashAggregate` doesn't
//! preserve it — but `(seq, id)` are both `kana_text` columns and
//! stable across DB rebuilds.
//!
//! Upstream is unchanged. This `ORDER BY` is a Rust-only repair that
//! makes the value-copy populator agree with upstream's
//! aliasing-driven populator on which row instance the cache stores.
//! Marking as a documented intentional divergence per CONVENTIONS
//! §4.4.
//!
//! [`init_suffixes_thread`]: super::init_suffixes_thread
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
    // ORDER BY (seq DESC, id DESC) is a documented driver-divergence
    // repair — see the module doc.
    let kts: Vec<KanaText> = sqlx::query_as(
        "SELECT * FROM ( \
           SELECT kt.* FROM kana_text kt WHERE kt.seq = $1 \
           UNION \
           SELECT kt.* FROM kana_text kt \
           LEFT JOIN conjugation conj ON conj.seq = kt.seq \
           WHERE conj.\"from\" = $1 \
         ) u ORDER BY u.seq DESC, u.id DESC",
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
