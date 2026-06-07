//! Port of `ichiran/dict:get-kanji-kana-old` (`dict.lisp:117-124`).
//!
//! Fallback `get-kana` body for the kanji-text method. Builds a regex
//! from the kanji-text's surface form, walks the entry's `kana_text`
//! rows in `ord` order and returns the first kana whose text matches;
//! if none match, returns the first kana row's text.

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
