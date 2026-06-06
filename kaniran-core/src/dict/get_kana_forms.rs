//! Transliteration of `ichiran/dict:get-kana-forms` (`dict-grammar.lisp:32`).
//!
//! Thin wrapper: returns [`get_kana_forms_star_`]; emits a stderr
//! warning when the result is empty.
//!
//! [`get_kana_forms_star_`]: super::get_kana_forms_star_::get_kana_forms_star_

use crate::conn::kani_context::KaniranContext;
use super::get_kana_forms_star_::get_kana_forms_star_;
use super::kana_text_dao::KanaText;

pub async fn get_kana_forms(
    ctx: &KaniranContext,
    seq: i32,
) -> Result<Vec<KanaText>, sqlx::Error> {
    let result = get_kana_forms_star_(ctx, seq).await?;
    if result.is_empty() {
        eprintln!("kaniran: No kana forms found for: {seq}");
    }
    Ok(result)
}
