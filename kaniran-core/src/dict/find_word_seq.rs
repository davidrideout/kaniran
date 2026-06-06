//! Port of `ichiran/dict:find-word-seq` (`dict-grammar.lisp:75`).
//!
//! Returns the rows of `kana_text` or `kanji_text` where `text = word`
//! and `seq ∈ seqs`. The table is chosen by
//! [`crate::characters::test_word`] against [`CharClass::Kana`] — kana
//! inputs hit `kana_text`, anything else hits `kanji_text`.

use crate::characters::char_class_type::CharClass;
use crate::characters::test_word::test_word;
use crate::conn::kani_context::KaniranContext;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kani_word::KaniWordDispatchEnum;
use crate::dict::kanji_text_dao::KanjiText;

#[derive(Debug, Clone)]
pub enum WordSeqRows {
    Kana(Vec<KanaText>),
    Kanji(Vec<KanjiText>),
}

impl WordSeqRows {
    /// `(car ...)` over the row vector, wrapping the row as a
    /// [`KaniWordDispatchEnum`] for downstream dispatchers. Empty
    /// vector yields `None`. Used by `*split-map*` entries that
    /// resolve a part via `(car (apply find-word-seq ...))`.
    pub(crate) fn first_word(self) -> Option<KaniWordDispatchEnum> {
        match self {
            Self::Kana(v) => v.into_iter().next().map(KaniWordDispatchEnum::Kana),
            Self::Kanji(v) => v.into_iter().next().map(KaniWordDispatchEnum::Kanji),
        }
    }

    /// First row's `seq`, used by the `("text" seq) part-seq` form
    /// of `def-simple-split` to compute the dynamic pseq via
    /// `(seq (car (find-word-conj-of "text" seq)))`. Empty vector
    /// yields `None`.
    pub(crate) fn first_seq(&self) -> Option<i32> {
        match self {
            Self::Kana(v) => v.first().map(|r| r.seq),
            Self::Kanji(v) => v.first().map(|r| r.seq),
        }
    }
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
