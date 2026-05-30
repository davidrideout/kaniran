//! Port of `ichiran/dict:true-kana` (`dict.lisp:560-562`).
//!
//! Returns the kana writing for a reading, descending through any
//! `proxy-text` wrappers to the underlying source. The Lisp gf has
//! two methods:
//!
//! ```lisp
//! (defgeneric true-kana (obj)
//!   (:method (obj) (get-kana obj))
//!   (:method ((obj proxy-text)) (true-kana (source obj))))
//! ```
//!
//! The Rust port dispatches on [`KaniWordDispatchEnum`]. The
//! `proxy-text` branch unwraps through [`ProxyText::source`]
//! iteratively (the source is itself a [`KaniSimpleTextDispatchEnum`]
//! that may carry another `Proxy`) and then re-enters the
//! [`super::get_kana::get_kana`] dispatcher for the terminal
//! kanji-text / kana-text. Every non-proxy branch delegates directly
//! to [`get_kana`].
//!
//! Diverges from the upstream lambda list `(obj)` by:
//! - taking `&KaniranContext` for the database handle, replacing the
//!   upstream dynamic `*connection*` per
//!   [`crate::conn::kani_context`];
//! - returning [`Result<Option<String>, sqlx::Error>`] — `None`
//!   propagates from the inner [`get_kana`] when upstream would
//!   raise via `(text nil)` (no headword kana row for entry, or no
//!   sibling kana_text row for a kanji-text path). See [`get_kana`]
//!   divergence notes; this fn is a thin proxy and inherits the
//!   signaling shape.
//!
//! Async because [`get_kana`] reaches the database in the kanji-text
//! arm via [`super::best_kana_conj::best_kana_conj`].
//!
//! Hint-state contract: callers of `true-kana` from inside a hint
//! body operate under a ctx rebound via
//! [`KaniranContext::with_disable_hints`]`(true)` (the
//! `simple-text :around` method in [`get_kana`] does this when
//! invoking [`super::split::hint::get_hint`]). The recursive `get_kana`
//! call on the leaf reads `ctx.disable_hints = true` and skips the
//! hint branch, matching upstream's `(let ((*disable-hints* t)) …)`.

use crate::conn::kani_context::KaniranContext;
use crate::dict::get_kana::get_kana;
use crate::dict::kani::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use crate::dict::proxy_text_class::ProxyText;

pub async fn true_kana(
    ctx: &KaniranContext,
    obj: &KaniWordDispatchEnum,
) -> Result<Option<String>, sqlx::Error> {
    match obj {
        // dict.lisp:562 (:method ((obj proxy-text)) (true-kana (source obj)))
        KaniWordDispatchEnum::Proxy(p) => {
            let leaf = unwrap_proxy_chain(p);
            let lifted = match leaf {
                KaniSimpleTextDispatchEnum::Kanji(k) => KaniWordDispatchEnum::Kanji(k.clone()),
                KaniSimpleTextDispatchEnum::Kana(k) => KaniWordDispatchEnum::Kana(k.clone()),
                KaniSimpleTextDispatchEnum::Proxy(_) => unreachable!(
                    "unwrap_proxy_chain terminates at Kanji or Kana"
                ),
            };
            Box::pin(get_kana(ctx, &lifted)).await
        }
        // dict.lisp:561 (:method (obj) (get-kana obj))
        other => Box::pin(get_kana(ctx, other)).await,
    }
}

fn unwrap_proxy_chain(start: &ProxyText) -> &KaniSimpleTextDispatchEnum {
    let mut current: &KaniSimpleTextDispatchEnum = &start.source;
    loop {
        match current {
            KaniSimpleTextDispatchEnum::Proxy(p) => current = &p.source,
            _ => return current,
        }
    }
}
