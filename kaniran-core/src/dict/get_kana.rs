//! Port of `ichiran/dict:get-kana` (gf — `dict.lisp:12-13`).
//!
//! Generic function returning the most popular kana representation
//! for a word, dispatching over the word variant.

use crate::conn::kani_context::KaniranContext;
use crate::dict::kani_word::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};

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

