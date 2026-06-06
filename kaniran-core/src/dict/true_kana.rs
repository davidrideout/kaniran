//! Port of `ichiran/dict:true-kana` (`dict.lisp:560-562`).
//!
//! Returns the kana writing for a reading via [`get_kana`], descending
//! through any `proxy-text` wrappers to the underlying source first.

use crate::conn::kani_context::KaniranContext;
use crate::dict::get_kana::get_kana;
use crate::dict::kani_word::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
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
