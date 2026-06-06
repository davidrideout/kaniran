//! Port of `ichiran/dict:find-word` (`dict.lisp:489`).
//!
//! Looks up rows of `kana_text` / `kanji_text` whose `text` column
//! equals `word`. Picks the table by [`test_word`] against
//! [`CharClass::Kana`] — kana inputs hit `kana_text`, anything else
//! hits `kanji_text`. With `root_only=true`, an inner-join against
//! `entry` restricts results to entries flagged `root_p`
//! (derivation-pruning survivors).
//!
//! When the ctx's `*substring-hash*` cache holds `word` as a key, the
//! cached rows short-circuit the SQL query (root-only is excluded from
//! the short-circuit).

use crate::characters::char_class_type::CharClass;
use crate::characters::test_word::test_word;
use crate::conn::kani_context::KaniranContext;
use crate::dict::_star_max_word_length_star_::MAX_WORD_LENGTH;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kanji_text_dao::KanjiText;

#[derive(Debug, Clone)]
pub enum FindWordRows {
    Kana(Vec<KanaText>),
    Kanji(Vec<KanjiText>),
}

pub async fn find_word(
    ctx: &KaniranContext,
    word: &str,
    root_only: bool,
) -> Result<FindWordRows, sqlx::Error> {
    // Mirror upstream evaluation order — `(when (<= (length word)
    // *max-word-length*) ...)` short-circuits before `test-word`
    // runs, so the over-length path returns an empty result without
    // touching the kana/kanji predicate. Lisp returns plain nil; the
    // Rust shape (closed 2-variant per CONVENTIONS §4.3) demands a
    // tag, so we hardcode `Kanji(Vec::new())` — every consumer
    // iterates the variant as a list and observes only the (empty)
    // contents, never the tag, so the choice is arbitrary and a
    // fixed value avoids the spurious `test_word` call.
    if word.chars().count() > MAX_WORD_LENGTH {
        return Ok(FindWordRows::Kanji(Vec::new()));
    }
    // dict.lisp:491 — (and *substring-hash* (gethash word *substring-hash*))
    if !root_only {
        if let Some(cache) = ctx.substring_hash.as_deref() {
            if let Some(rows) = cache.get(word) {
                return Ok(rows.clone());
            }
        }
    }
    let kana = test_word(word, CharClass::Kana);
    if kana {
        let rows: Vec<KanaText> = if root_only {
            sqlx::query_as(
                "SELECT wt.* FROM kana_text wt \
                 INNER JOIN entry ON wt.seq = entry.seq \
                 WHERE wt.text = $1 AND entry.root_p",
            )
            .bind(word)
            .fetch_all(&ctx.pool)
            .await?
        } else {
            sqlx::query_as("SELECT * FROM kana_text WHERE text = $1")
                .bind(word)
                .fetch_all(&ctx.pool)
                .await?
        };
        Ok(FindWordRows::Kana(rows))
    } else {
        let rows: Vec<KanjiText> = if root_only {
            sqlx::query_as(
                "SELECT wt.* FROM kanji_text wt \
                 INNER JOIN entry ON wt.seq = entry.seq \
                 WHERE wt.text = $1 AND entry.root_p",
            )
            .bind(word)
            .fetch_all(&ctx.pool)
            .await?
        } else {
            sqlx::query_as("SELECT * FROM kanji_text WHERE text = $1")
                .bind(word)
                .fetch_all(&ctx.pool)
                .await?
        };
        Ok(FindWordRows::Kanji(rows))
    }
}
