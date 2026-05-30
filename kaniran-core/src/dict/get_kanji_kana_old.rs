//! Port of `ichiran/dict:get-kanji-kana-old` (`dict.lisp:117-124`).
//!
//! Fallback `get-kana` body used by the kanji-text method
//! (`dict.lisp:111-115`) when `best-kana-conj` returns the `:null`
//! sentinel — the per-entry curated reading is unavailable. Builds a
//! [`kanji_regex`] from the kanji-text's surface form, then walks the
//! entry's `kana_text` rows in `ord` order and returns the first kana
//! whose text matches; if none match, returns the first kana row's
//! text outright.
//!
//! Diverges from the upstream lambda list `(obj)` only by:
//! - taking `&KaniranContext` for the database handle, replacing the
//!   upstream dynamic `*connection*` per [`crate::conn::kani_context`];
//! - typing `obj` as [`KanjiText`] rather than the Lisp `t` — the sole
//!   call site (`dict.lisp:114` inside `get-kana ((obj kanji-text))`)
//!   passes a kanji-text, and the body only consumes `(text obj)` and
//!   `(seq obj)`, both available as direct field reads on [`KanjiText`].
//! - returning [`Option<String>`] rather than a raw string. Upstream
//!   would error on `(text nil)` if `kts` is empty (data integrity
//!   violation: a kanji_text row with no sibling kana_text rows for
//!   its seq); the Rust port surfaces that as [`None`] so callers can
//!   propagate without a panic.

use crate::characters::kanji::kanji_regex;
use crate::conn::kani_context::KaniranContext;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kanji_text_dao::KanjiText;

pub async fn get_kanji_kana_old(
    ctx: &KaniranContext,
    obj: &KanjiText,
) -> Result<Option<String>, sqlx::Error> {
    let regex = kanji_regex(&obj.text);
    let kts = sqlx::query_as::<_, KanaText>(
        "SELECT * FROM kana_text WHERE seq = $1 ORDER BY ord",
    )
    .bind(obj.seq)
    .fetch_all(&ctx.pool)
    .await?;
    for kt in &kts {
        if regex.is_match(&kt.text).unwrap_or(false) {
            return Ok(Some(kt.text.clone()));
        }
    }
    Ok(kts.into_iter().next().map(|kt| kt.text))
}
