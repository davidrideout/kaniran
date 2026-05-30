//! Port of `ichiran/dict:true-kanji` (`dict.lisp:564-566`).
//!
//! Returns the kanji writing for a reading, descending through any
//! `proxy-text` wrappers to the underlying source. The Lisp gf has
//! two methods:
//!
//! ```lisp
//! (defgeneric true-kanji (obj)
//!   (:method (obj) (get-kanji obj))
//!   (:method ((obj proxy-text)) (true-kanji (source obj))))
//! ```
//!
//! The Rust port dispatches on [`KaniWordDispatchEnum`]. The
//! `proxy-text` branch unwraps through [`ProxyText::source`]
//! iteratively (the source is itself a [`KaniSimpleTextDispatchEnum`]
//! that may carry another `Proxy`) and then re-enters the
//! [`super::get_kanji::get_kanji`] dispatcher for the terminal
//! kanji-text / kana-text. Every non-proxy branch delegates directly
//! to [`get_kanji`].
//!
//! Diverges from the upstream lambda list `(obj)` only by taking
//! `&KaniranContext` for the database handle, replacing the upstream
//! dynamic `*connection*` per [`crate::conn::kani_context`]. Async
//! because [`get_kanji`]'s kana-text branch reaches the database.

use crate::conn::kani_context::KaniranContext;
use crate::dict::get_kanji::get_kanji;
use crate::dict::kani::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use crate::dict::proxy_text_class::ProxyText;

pub async fn true_kanji(
    ctx: &KaniranContext,
    obj: &KaniWordDispatchEnum,
) -> Result<Option<String>, sqlx::Error> {
    match obj {
        // dict.lisp:566 (:method ((obj proxy-text)) (true-kanji (source obj)))
        KaniWordDispatchEnum::Proxy(p) => {
            let lifted = match unwrap_proxy_chain(p) {
                KaniSimpleTextDispatchEnum::Kanji(k) => KaniWordDispatchEnum::Kanji(k.clone()),
                KaniSimpleTextDispatchEnum::Kana(k) => KaniWordDispatchEnum::Kana(k.clone()),
                KaniSimpleTextDispatchEnum::Proxy(_) => unreachable!(
                    "unwrap_proxy_chain terminates at Kanji or Kana"
                ),
            };
            get_kanji(ctx, &lifted).await
        }
        // dict.lisp:565 (:method (obj) (get-kanji obj))
        other => get_kanji(ctx, other).await,
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
