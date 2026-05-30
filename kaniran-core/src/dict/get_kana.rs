//! Port of `ichiran/dict:get-kana` (gf — `dict.lisp:12-13`).
//!
//! Generic function returning "most popular kana representation"
//! for a word. Multi-method dispatch upstream — fourteen method
//! bodies across the simple-text family, the counter-text family
//! and proxy/compound:
//!
//! - **`((obj entry))`** at `dict.lisp:44-45` — ported on
//!   [`Entry::get_kana`] (in [`super::entry_dao`]). Reached only
//!   from locally-Entry-typed callsites (entry-digest at
//!   `dict.lisp:67`); no upstream caller passes an entry through
//!   polymorphic dispatch, so the inherent method is not wired
//!   through this file's dispatcher.
//! - **`((obj simple-text)) :around`** at `dict.lisp:80-84` —
//!   the hint dispatcher. If `*disable-hints*` is unset and the
//!   reading isn't already hinted, rebind `*disable-hints*` to T
//!   and try [`super::split::hint::get_hint`]; a non-nil hint result
//!   shortcuts the return, otherwise fall through to the primary
//!   method (call-next-method).
//! - **`((obj kanji-text))`** at `dict.lisp:111-115` — try
//!   [`best_kana_conj`]; if `:null`, fall through to
//!   [`get_kanji_kana_old`].
//! - **`((obj kana-text))`** at `dict.lisp:150-151` — `(text obj)`.
//! - **`((obj proxy-text))`** — auto-generated slot reader from
//!   `(kana :reader get-kana :initarg :kana)` at `dict.lisp:552`.
//! - **`((obj compound-text))`** — auto-generated slot reader from
//!   `(kana :reader get-kana :initarg :kana)` at `dict.lisp:610`.
//! - **`((obj counter-text)) :around`** at `dict-counters.lisp:69-71` —
//!   append `(counter-suffix obj)` to the primary result, when
//!   non-nil.
//! - **`((obj counter-text))`** at `dict-counters.lisp:64-67` —
//!   `(counter-join obj n (number-to-kana n :separator *kana-hint-space*)
//!   (copy-seq (counter-kana obj)))`.
//! - **`((obj number-text))`** at `dict-counters.lisp:208-209` —
//!   `(number-to-kana (number-value obj) :separator *kana-hint-space*)`.
//! - **`((obj counter-tsu))`** at `dict-counters.lisp:502-513` —
//!   table over 1..=9; otherwise `(call-next-method)`.
//! - **`((obj counter-hifumi))`** at `dict-counters.lisp:521-538` —
//!   prefix from a closed 1..=10 kun-yomi table + `(counter-kana
//!   obj)` when `value` is in `digit-set`; otherwise
//!   `(call-next-method)`.
//! - **`((obj counter-days-kun))`** at `dict-counters.lisp:689-704` —
//!   case-table over the `allowed` slot's exact values (no
//!   fallback). The `verify` method restricts inputs to those
//!   values, so an unmapped number is impossible at runtime; the
//!   Rust port returns an empty string for that case (matching
//!   Lisp's `case`-without-`t` returning `nil`, which the
//!   `:around` wrapper concatenates with the suffix).
//! - **`((obj counter-people))`** at `dict-counters.lisp:737-741` —
//!   1 → ひとり, 2 → ふたり, else `(call-next-method)`.
//! - **`((obj counter-age))`** at `dict-counters.lisp:759-762` —
//!   20 → はたち, else `(call-next-method)`.
//!
//! ## Divergences
//!
//! Diverges from the upstream lambda list `(obj)` by:
//!
//! - taking `&KaniranContext` for the database handle, replacing
//!   the upstream dynamic `*connection*` per
//!   [`crate::conn::kani_context`]. The same `&KaniranContext` also
//!   carries the upstream `*disable-hints*` binding as a
//!   [`KaniranContext::disable_hints`] field: the `:around` rebind
//!   at `dict.lisp:82` is mirrored by
//!   [`KaniranContext::with_disable_hints`] returning a sibling
//!   ctx, which propagates through the borrow-checker rather than
//!   through a `task_local!`/`thread_local!` side channel. Survives
//!   rayon / `tokio::spawn` boundaries without a per-boundary
//!   snapshot/restore obligation.
//! - returning [`Result<Option<String>, sqlx::Error>`] — `None`
//!   covers the cases where upstream would signal
//!   `no-applicable-method` via `(text nil)`: the kanji-text path's
//!   `best-kana-conj :null` falling through to
//!   `(get-kanji-kana-old obj)` returning nil (no sibling kana_text
//!   row), and the entry path returning nil (no ord=0 kana_text
//!   row for the entry's seq). Upstream raises a CL condition
//!   recoverable via `handler-case`; the Rust port surfaces the
//!   same case as `Ok(None)` so callers can branch on it without
//!   `catch_unwind`. Database errors propagate as the
//!   `sqlx::Error` arm.

use crate::conn::kani_context::KaniranContext;
use crate::dict::kani::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};

pub async fn get_kana(
    ctx: &KaniranContext,
    obj: &KaniWordDispatchEnum,
) -> Result<Option<String>, sqlx::Error> {
    match obj {
        // simple-text family handles its own `:around` internally
        // (dict.lisp:80-84) — see [`KaniSimpleTextDispatchEnum::get_kana`].
        // The clone wraps a borrowed simple-text variant into the
        // family enum; the family method then implements both the
        // `:around` and the primary `call-next-method`.
        KaniWordDispatchEnum::Kanji(k) => {
            KaniSimpleTextDispatchEnum::Kanji(k.clone())
                .get_kana(ctx).await
        }
        KaniWordDispatchEnum::Kana(k) => {
            KaniSimpleTextDispatchEnum::Kana(k.clone())
                .get_kana(ctx).await
        }
        KaniWordDispatchEnum::Proxy(p) => {
            KaniSimpleTextDispatchEnum::Proxy(p.clone())
                .get_kana(ctx).await
        }
        // counter-text family handles its own `:around` (suffix
        // append) and per-subclass overrides internally — see
        // [`Counter::get_kana`].
        KaniWordDispatchEnum::Counter(c) => Ok(Some(c.get_kana())),
        // dict.lisp:610 (kana :reader get-kana :initarg :kana) on compound-text
        KaniWordDispatchEnum::Compound(c) => Ok(Some(c.kana.clone())),
    }
}

