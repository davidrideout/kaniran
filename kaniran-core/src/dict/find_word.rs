//! Port of `ichiran/dict:find-word` (`dict.lisp:489`).
//!
//! Looks up rows of `kana_text` / `kanji_text` whose `text` column
//! equals `word`. Picks the table by [`test_word`] against
//! [`CharClass::Kana`] — kana inputs hit `kana_text`, anything else
//! hits `kanji_text`. With `root_only=true`, an inner-join against
//! `entry` restricts results to entries flagged `root_p`
//! (derivation-pruning survivors).
//!
//! Diverges from the upstream lambda list `(word &key root-only)`
//! by:
//!
//! - taking `&KaniranContext` for the database handle, replacing
//!   the upstream dynamic `*connection*` per
//!   [`crate::conn::kani_context`];
//! - taking `root_only` as a plain `bool` rather than the upstream
//!   `&key` keyword. Per CONVENTIONS §4.4 a 2-state keyword whose
//!   default is the falsy value is interchangeable with `bool` —
//!   the parameter name reads clearly at the call site
//!   (`find_word(ctx, w, true)` ↔ `:root-only t`; `false` ↔
//!   `:root-only nil` or absent, both resolving to the same upstream
//!   behavior because the keyword defaults to `nil`).
//!
//! The polymorphic Lisp return — a list of `kana-text` xor
//! `kanji-text` rows depending on the table chosen — is wrapped in a
//! 2-variant enum per CONVENTIONS §4.3. Same shape as
//! [`super::grammar::find_word::WordSeqRows`] but kept as its own type
//! because the two functions have distinct semantics
//! (`find-word-seq` filters by `seq IN (...)`; `find-word` does not),
//! and each type stays local to the function that owns it per §1.
//!
//! ## *substring-hash* short-circuit
//!
//! Upstream consults the dynamically-bound
//! [`crate::dict::_star_substring_hash_star_`] cache before issuing
//! the SQL query. The cache rides on the ctx as
//! [`KaniranContext::substring_hash`]; populated by
//! `find-substring-words` (and rebound via
//! [`KaniranContext::with_substring_hash`] for the nested-find loop
//! inside `find-word-full`), it's read back here for any `word`
//! present as a key — root-only is excluded from the short-circuit
//! because the upstream `if` routes root-only directly to the
//! JOIN-against-`entry` branch. Today no producer wires up the
//! cache, so the slot is `None` and the path is dead at runtime;
//! the wire-up lands when wave 326 (`find-word-full`) ports.

use crate::characters::char_classes::CharClass;
use crate::characters::char_classes::test_word;
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
