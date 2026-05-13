//! Port of `ichiran/dict:find-word-as-hiragana` (`dict.lisp:592`).
//!
//! When `str` is not strictly hiragana, looks up its hiragana
//! equivalent through [`find_word`] root-only (or through the
//! caller-provided `finder` override) and wraps each resulting
//! reading in a [`ProxyText`] that carries the original surface
//! form as both `text` and `kana` while delegating identity
//! through to the underlying simple-text row.
//!
//! Diverges from the upstream lambda list
//! `(str &key exclude finder)` only by:
//!
//! - taking `&KaniranContext` for the database handle, replacing
//!   the upstream dynamic `*connection*` per
//!   [`crate::conn::kani_context`];
//! - taking `exclude` as `&[i32]` (positional) rather than a `&key`
//!   keyword. Both upstream callers
//!   (`find-word-full` `dict.lisp:1057`, `or-as-hiragana`
//!   `dict-grammar.lisp:97`) pass it with a computed list value;
//!   there is no "absent vs explicit nil" distinction to preserve;
//! - taking `finder` as
//!   `Option<Box<dyn FnOnce(String) -> Pin<Box<dyn Future<...>>>>>`
//!   ([`HiraganaFinder`]). Rust async closures + generic `Future`
//!   bounds don't infer `None`'s `F` without a turbofish at the call
//!   site; a boxed-trait object sidesteps that and matches the
//!   upstream "callable or absent" shape exactly. `None` ↔
//!   `:finder nil` (or absent — both fall to
//!   `find_word(ctx, hira, true)`); `Some(boxed)` ↔ caller-provided
//!   closure invoked with the hiragana form.
//!
//! Returns `Vec<ProxyText>` directly rather than the upstream
//! list-of-`make-instance`-results: every callsite immediately
//! consumes the rows as a homogeneous proxy-text sequence (`nconc`
//! into the `find-word-full` simple-words pile, or wrapped through
//! `or-as-hiragana`'s `or`).

use std::future::Future;
use std::pin::Pin;

use crate::characters::as_hiragana::as_hiragana;
use crate::conn::kani_context::KaniranContext;
use crate::dict::find_word::{find_word, FindWordRows};
use crate::dict::kani_word::KaniSimpleTextDispatchEnum;
use crate::dict::proxy_text_class::ProxyText;
use crate::dict::simple_text_class::SimpleText;

/// Boxed async finder closure for [`find_word_as_hiragana`]. Mirrors
/// `or-as-hiragana`'s `(lambda (w) (apply fn w args))` callback shape
/// (`dict-grammar.lisp:97-100`): a one-shot unary call that takes
/// the hiragana surface form and returns either a [`FindWordRows`]
/// list or a `sqlx::Error`. `Send` so the result composes with
/// `tokio::task::spawn` paths (audit harness, segmenter pipeline).
pub type HiraganaFinder<'a> = Box<
    dyn FnOnce(String) -> Pin<Box<dyn Future<Output = Result<FindWordRows, sqlx::Error>> + Send + 'a>>
        + Send
        + 'a,
>;

pub async fn find_word_as_hiragana(
    ctx: &KaniranContext,
    str_: &str,
    exclude: &[i32],
    finder: Option<HiraganaFinder<'_>>,
) -> Result<Vec<ProxyText>, sqlx::Error> {
    let as_hira = as_hiragana(str_);
    if str_ == as_hira {
        return Ok(Vec::new());
    }
    let words = match finder {
        Some(f) => f(as_hira).await?,
        // root_only=true, so the substring-hash short-circuit doesn't
        // apply (find_word skips the cache check for root_only). Pass
        // None unconditionally — there's no path where this caller
        // holds a cache to forward.
        None => find_word(ctx, &as_hira, true, None).await?,
    };
    let proxies = match words {
        FindWordRows::Kana(rows) => rows
            .into_iter()
            .filter(|w| !exclude.contains(&w.seq))
            .map(|w| ProxyText {
                text: str_.to_string(),
                kana: str_.to_string(),
                source: Box::new(KaniSimpleTextDispatchEnum::Kana(w)),
                state: SimpleText::default(),
            })
            .collect(),
        FindWordRows::Kanji(rows) => rows
            .into_iter()
            .filter(|w| !exclude.contains(&w.seq))
            .map(|w| ProxyText {
                text: str_.to_string(),
                kana: str_.to_string(),
                source: Box::new(KaniSimpleTextDispatchEnum::Kanji(w)),
                state: SimpleText::default(),
            })
            .collect(),
    };
    Ok(proxies)
}
