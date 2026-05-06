//! Transliteration of `ichiran/dict:find-word-conj-of` (`dict-grammar.lisp:79`).
//!
//! Returns kana_text or kanji_text rows for `word` matching either:
//! (a) any of `seqs` directly (= [`crate::dict::find_word_seq::find_word_seq`]),
//!     or
//! (b) any seq joined to `seqs` via the `conjugation.from` column.
//!
//! The two row sets are unioned and deduplicated by `id` per SBCL's
//! `(union list1 list2 :key #'id)` semantics: `result` starts as
//! `list2` (the JOIN-query rows in DB order); `list1` (find-word-seq
//! rows) is then walked left-to-right and each non-duplicate is
//! prepended. Net effect: list1's non-duplicates appear at the head in
//! reversed order, followed by list2 in its original order. SBCL's
//! `union` does NOT deduplicate within `list2`, and the JOIN can emit
//! the same `kt.*` row multiple times when a `kt.seq` carries several
//! `conjugation.from` matches; we mirror that — no implicit DISTINCT.
//!
//! Diverges from the upstream lambda list `(word &rest seqs)` by
//! taking `&KaniranContext` and `seqs: &[i32]`, identical to
//! [`crate::dict::find_word_seq::find_word_seq`].

use crate::characters::char_class_type::CharClass;
use crate::characters::test_word::test_word;
use crate::conn::kani_context::KaniranContext;
use crate::dict::find_word_seq::{find_word_seq, WordSeqRows};
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kanji_text_dao::KanjiText;
use std::collections::HashSet;

pub async fn find_word_conj_of(
    ctx: &KaniranContext,
    word: &str,
    seqs: &[i32],
) -> Result<WordSeqRows, sqlx::Error> {
    let primary = find_word_seq(ctx, word, seqs).await?;
    if test_word(word, CharClass::Kana) {
        let conj_rows: Vec<KanaText> = sqlx::query_as::<_, KanaText>(
            "SELECT kt.* FROM kana_text kt, conjugation conj \
             WHERE kt.seq = conj.seq AND conj.\"from\" = ANY($1) AND kt.text = $2",
        )
        .bind(seqs)
        .bind(word)
        .fetch_all(&ctx.pool)
        .await?;
        let primary_rows = match primary {
            WordSeqRows::Kana(v) => v,
            WordSeqRows::Kanji(_) => unreachable!(
                "test_word dispatch must agree between find-word-seq and find-word-conj-of"
            ),
        };
        Ok(WordSeqRows::Kana(union_by_id(primary_rows, conj_rows, |r| r.id)))
    } else {
        let conj_rows: Vec<KanjiText> = sqlx::query_as::<_, KanjiText>(
            "SELECT kt.* FROM kanji_text kt, conjugation conj \
             WHERE kt.seq = conj.seq AND conj.\"from\" = ANY($1) AND kt.text = $2",
        )
        .bind(seqs)
        .bind(word)
        .fetch_all(&ctx.pool)
        .await?;
        let primary_rows = match primary {
            WordSeqRows::Kanji(v) => v,
            WordSeqRows::Kana(_) => unreachable!(
                "test_word dispatch must agree between find-word-seq and find-word-conj-of"
            ),
        };
        Ok(WordSeqRows::Kanji(union_by_id(primary_rows, conj_rows, |r| r.id)))
    }
}

/// `(union list1 list2 :key id)` for SBCL semantics — NOT a generic
/// set union. `list2` is preserved verbatim (including any internal
/// duplicates); `list1` is walked left-to-right and each id not yet
/// present is prepended to the result. Empirically verified via
/// `(union '(1 2 3) '(4 5 6))` → `(3 2 1 4 5 6)` on SBCL 2.2.9.
fn union_by_id<T>(list1: Vec<T>, list2: Vec<T>, id: impl Fn(&T) -> i32) -> Vec<T> {
    let mut keys: HashSet<i32> = list2.iter().map(&id).collect();
    let mut prefix: Vec<T> = Vec::new();
    for x in list1 {
        if keys.insert(id(&x)) {
            prefix.push(x);
        }
    }
    prefix.reverse();
    prefix.extend(list2);
    prefix
}
