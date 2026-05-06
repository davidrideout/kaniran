//! Transliteration of `ichiran/dict:get-kana-form` (`dict-grammar.lisp:38`).
//!
//! Looks up a single kana_text row by `(text, seq)` pair. If a `conj`
//! value is supplied, mutates the loaded row's runtime-only
//! [`crate::dict::simple_text_class::SimpleText::conjugations`] slot
//! before returning it.
//!
//! The Lisp uses `(car (select-dao 'kana-text ...))` to take the first
//! match of the `(text, seq)` filter — primary key index makes this
//! deterministic and at-most-one in practice. We mirror that with
//! `fetch_all().into_iter().next()`; an extra `LIMIT 1` would be a
//! micro-optimization the upstream doesn't carry.
//!
//! Diverges from the upstream lambda list `(seq text &key conj)` by:
//! - taking `&KaniranContext` for the DB handle, replacing Lisp's
//!   `*connection*` per the [`crate::conn::kani_context`] module doc;
//! - taking `conj` as `Option<WordConjugations>` instead of `&key conj`
//!   — Rust has no keyword arguments, and the slot's value space is
//!   exactly what [`WordConjugations`] encodes (`:root` or a list of
//!   conjugation ids; `nil` corresponds to `None`).

use crate::conn::kani_context::KaniranContext;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::simple_text_class::WordConjugations;

pub async fn get_kana_form(
    ctx: &KaniranContext,
    seq: i32,
    text: &str,
    conj: Option<WordConjugations>,
) -> Result<Option<KanaText>, sqlx::Error> {
    let row = sqlx::query_as::<_, KanaText>(
        "SELECT * FROM kana_text WHERE text = $1 AND seq = $2",
    )
    .bind(text)
    .bind(seq)
    .fetch_all(&ctx.pool)
    .await?
    .into_iter()
    .next();
    Ok(row.map(|mut r| {
        if let Some(c) = conj {
            r.state.conjugations = Some(c);
        }
        r
    }))
}
