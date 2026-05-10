//! Transliteration of `ichiran/dict:find-word-conj-of` (`dict-grammar.lisp:79`).
//!
//! Returns kana_text or kanji_text rows for `word` matching either:
//! (a) any of `seqs` directly (= [`crate::dict::find_word_seq::find_word_seq`]),
//!     or
//! (b) any seq joined to `seqs` via the `conjugation.from` column.
//!
//! The two row sets are unioned and deduplicated by `id` per SBCL's
//! `(union list1 list2 :key #'id)` semantics: SBCL's union picks the
//! longer of the two lists (list1 on length-tie), starts the result
//! as the shorter list, then iterates the longer list and prepends
//! each non-duplicate (cons-prepend, so the longer list ends up
//! reversed at the head). Net shape:
//!
//! `(reverse <longer-list's uniques>) ++ <shorter-list>`
//!
//! When list1 is empty (the typical kana-text-not-in-seqs case), the
//! JOIN result is the longer list and gets reversed wholesale —
//! matching the captured fixture order. SBCL's `union` does NOT
//! deduplicate within either list, and the JOIN can emit the same
//! `kt.*` row multiple times when a `kt.seq` carries several
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
/// set union. SBCL picks the longer list (list1 wins length-tie),
/// starts a fresh result, copies the shorter list in, then walks the
/// longer list left-to-right `cons`-pushing each non-duplicate. The
/// final shape is `reverse(<longer's uniques>) ++ <shorter>`.
///
/// Empirically verified on SBCL 2.2.9:
/// - `(union '() '(1 2 3))`            → `(3 2 1)`
/// - `(union '(1 2 3) '())`            → `(3 2 1)`
/// - `(union '(1 2 3) '(4 5 6))`       → `(3 2 1 4 5 6)` (list1 wins tie)
/// - `(union '(4) '(1 2 3))`           → `(3 2 1 4)` (list2 longer)
/// - `(union '(1 2 3) '(2))`           → `(3 1 2)` (skip dup 2)
fn union_by_id<T>(list1: Vec<T>, list2: Vec<T>, id: impl Fn(&T) -> i32) -> Vec<T> {
    let (shorter, longer) = if list1.len() >= list2.len() {
        (list2, list1)
    } else {
        (list1, list2)
    };
    let shorter_keys: HashSet<i32> = shorter.iter().map(&id).collect();
    let mut uniques: Vec<T> = Vec::new();
    for elt in longer {
        if !shorter_keys.contains(&id(&elt)) {
            uniques.push(elt);
        }
    }
    uniques.reverse();
    uniques.extend(shorter);
    uniques
}
