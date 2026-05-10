//! Port of `ichiran/kanji:get-readings` (`kanji.lisp:213`).
//!
//! Looks up the kanjidic2 readings of `char`, defaulting to
//! everything except `ja_na` (named-reading) entries. With `names`
//! set, the typeset filter is empty — upstream's `(if names nil
//! '("ja_na"))` — and the underlying SQL `NOT IN ()` reduces to
//! UNKNOWN for every row, so the call returns an empty `Vec`. This
//! matches the upstream REPL capture `(get-readings #\日 :names t)`
//! ⇒ `NIL`.
//!
//! Diverges from the upstream lambda list `(char &key names)` by:
//!
//! - taking `&KaniranContext` for the database handle, replacing the
//!   upstream dynamic `*connection*` per [`crate::conn::kani_context`];
//! - taking `names` as a plain `bool` per CONVENTIONS §4.4. The
//!   keyword is binary (absent ↔ off, `t` ↔ on);
//! - taking `char: char`. Upstream accepts either a `character` or
//!   already-stringified input; the only in-scope callsites pass a
//!   single character.

use super::get_readings_cache::get_readings_cache;
use crate::conn::kani_context::KaniranContext;

pub async fn get_readings(
    ctx: &KaniranContext,
    char: char,
    names: bool,
) -> Result<Vec<(String, String)>, sqlx::Error> {
    let str: String = char.into();
    let typeset: Vec<String> = if names {
        Vec::new()
    } else {
        vec!["ja_na".to_string()]
    };
    get_readings_cache(ctx, &str, &typeset).await
}
