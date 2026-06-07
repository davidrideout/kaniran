//! Port of `ichiran/dict:get-kanji` (gf — `dict.lisp:15-16`).
//!
//! Generic function returning the most popular kanji representation
//! for a word, dispatching over the word variant.

use crate::conn::kani_context::KaniranContext;
use crate::dict::best_kanji_conj::best_kanji_conj;
use crate::dict::entry_dao::Entry;
use crate::dict::kani_word::KaniWordDispatchEnum;
use crate::numbers::constants::DIGIT_KANJI_DEFAULT;
use crate::numbers::constants::POWER_KANJI;
use crate::numbers::kanji_form::number_to_kanji;

pub async fn get_kanji(
    ctx: &KaniranContext,
    obj: &KaniWordDispatchEnum,
) -> Result<Option<String>, sqlx::Error> {
    match obj {
        // dict.lisp:108-109 (defmethod get-kanji ((obj kanji-text))) — (text obj)
        KaniWordDispatchEnum::Kanji(k) => Ok(Some(k.text.clone())),
        // dict.lisp:153-155 (defmethod get-kanji ((obj kana-text)))
        // (let ((bk (best-kanji-conj obj))) (unless (eql bk :null) bk))
        KaniWordDispatchEnum::Kana(k) => best_kanji_conj(ctx, k).await,
        // dict-counters.lisp:61-62 (defmethod get-kanji ((obj counter-text)))
        // (concatenate 'string (number-to-kanji (number-value obj)) (counter-text obj))
        KaniWordDispatchEnum::Counter(c) => {
            let base = c.base();
            let prefix = number_to_kanji(base.number, DIGIT_KANJI_DEFAULT, POWER_KANJI, false);
            Ok(Some(format!("{}{}", prefix, base.text)))
        }
        KaniWordDispatchEnum::Proxy(_) | KaniWordDispatchEnum::Compound(_) => {
            unreachable!(
                "get-kanji has no method on proxy-text / compound-text (dict.lisp:15)"
            )
        }
    }
}

impl Entry {
    /// `get-kanji` method body — `dict.lisp:51-53`:
    ///
    /// ```lisp
    /// (defmethod get-kanji ((obj entry))
    ///   (when (> (n-kanji obj) 0)
    ///     (text (car (select-dao 'kanji-text (:and (:= 'seq (seq obj)) (:= 'ord 0)))))))
    /// ```
    ///
    /// Returns the `text` of the entry's headword kanji row at
    /// `ord = 0` when the entry has any kanji writings; `None`
    /// otherwise.
    ///
    /// Diverges from the upstream lambda list `(obj)` only by taking
    /// `&KaniranContext` for the database handle, replacing the
    /// upstream dynamic `*connection*` per
    /// [`crate::conn::kani_context`]. `None` mirrors upstream falling
    /// off the `when` when `n-kanji = 0`; a missing `ord = 0` row
    /// propagates as [`sqlx::Error::RowNotFound`], matching upstream
    /// erroring on `(text nil)`.
    pub async fn get_kanji(
        &self,
        ctx: &KaniranContext,
    ) -> Result<Option<String>, sqlx::Error> {
        if self.n_kanji <= 0 {
            return Ok(None);
        }
        let (text,): (String,) = sqlx::query_as(
            "SELECT text FROM kanji_text WHERE seq = $1 AND ord = 0",
        )
        .bind(self.seq)
        .fetch_one(&ctx.pool)
        .await?;
        Ok(Some(text))
    }
}
