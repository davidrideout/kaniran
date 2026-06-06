//! Port of `ichiran/dict:true-kanji` (`dict.lisp:564-566`).
//!
//! Returns the kanji writing for a reading via [`get_kanji`],
//! descending through any `proxy-text` wrappers to the underlying
//! source first.

use crate::conn::kani_context::KaniranContext;
use crate::dict::get_kanji::get_kanji;
use crate::dict::kani_word::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
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
