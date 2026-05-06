//! Transliteration of `ichiran/dict:find-word-seq` (`dict-grammar.lisp:75`).
//!
//! Returns the rows of `kana_text` or `kanji_text` where `text = word`
//! and `seq ∈ seqs`. The table is chosen by
//! [`crate::characters::test_word`] against [`CharClass::Kana`] — kana
//! inputs hit `kana_text`, anything else hits `kanji_text`. The
//! polymorphic return is wrapped in a 2-variant enum per CONVENTIONS
//! §4.3 (closed `(:tag . value)` shape: the Lisp dispatch tag is the
//! table-name symbol; the Rust dispatch tag is the [`WordSeqRows`]
//! variant).
//!
//! Diverges from the upstream lambda list `(word &rest seqs)` by:
//! - taking `&KaniranContext` for the DB handle, replacing Lisp's
//!   `*connection*` per the [`crate::conn::kani_context`] module doc;
//! - taking `seqs` as `&[i32]` instead of `&rest` packed positional —
//!   Rust has no `&rest` keyword.
//!
//! Empty `seqs` returns the matching enum variant with an empty `Vec`.
//! Postgres' `seq = ANY($2)` against an empty array yields no rows,
//! mirroring the Lisp behavior where `(:in 'seq (:set))` filters
//! everything out.

use crate::characters::char_class_type::CharClass;
use crate::characters::test_word::test_word;
use crate::conn::kani_context::KaniranContext;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kanji_text_dao::KanjiText;

#[derive(Debug, Clone)]
pub enum WordSeqRows {
    Kana(Vec<KanaText>),
    Kanji(Vec<KanjiText>),
}

pub async fn find_word_seq(
    ctx: &KaniranContext,
    word: &str,
    seqs: &[i32],
) -> Result<WordSeqRows, sqlx::Error> {
    if test_word(word, CharClass::Kana) {
        let rows = sqlx::query_as::<_, KanaText>(
            "SELECT * FROM kana_text WHERE text = $1 AND seq = ANY($2)",
        )
        .bind(word)
        .bind(seqs)
        .fetch_all(&ctx.pool)
        .await?;
        Ok(WordSeqRows::Kana(rows))
    } else {
        let rows = sqlx::query_as::<_, KanjiText>(
            "SELECT * FROM kanji_text WHERE text = $1 AND seq = ANY($2)",
        )
        .bind(word)
        .bind(seqs)
        .fetch_all(&ctx.pool)
        .await?;
        Ok(WordSeqRows::Kanji(rows))
    }
}
