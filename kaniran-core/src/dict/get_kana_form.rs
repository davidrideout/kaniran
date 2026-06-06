//! Transliteration of `ichiran/dict:get-kana-form` (`dict-grammar.lisp:38`).
//!
//! Looks up the first kana_text row by `(text, seq)` pair. If a `conj`
//! value is supplied, mutates the loaded row's runtime-only
//! conjugations slot before returning it.

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
