//! Port of `ichiran/kanji:get-readings` (`kanji.lisp:213`).
//!
//! Looks up the kanjidic2 readings of `char`, defaulting to everything
//! except `ja_na` (named-reading) entries. With `names` set the typeset
//! filter is empty and the call returns an empty `Vec`.

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
