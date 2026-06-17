//! Plain-text romanization — the thinnest flavor: just the romaji string,
//! optionally followed by a dictionary-info trailer (legacy `-i`).

use std::error::Error;

use crate::conn::kani_context::KaniranContext;
use crate::core::kani_romanize_method::KaniRomanizeMethod;
use crate::core::romanize::romanize;

/// Romanize `input` to a single string.
pub(super) fn render(
    ctx: &KaniranContext,
    input: &str,
    method: KaniRomanizeMethod<'_>,
) -> Result<String, Box<dyn Error>> {
    Ok(romanize(ctx, input, method, false)?.0)
}

/// Romanize `input`, then append one `* word  gloss` line per definition.
pub(super) fn render_with_info(
    ctx: &KaniranContext,
    input: &str,
    method: KaniRomanizeMethod<'_>,
) -> Result<String, Box<dyn Error>> {
    let (romaji, info) = romanize(ctx, input, method, true)?;
    let mut out = romaji;
    for (word, gloss) in &info {
        // cli.lisp:44 (print-romanize-info)
        out.push_str(&format!("\n\n* {word}  {gloss}"));
    }
    Ok(out)
}
