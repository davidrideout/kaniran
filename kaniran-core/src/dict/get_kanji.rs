//! Port of `ichiran/dict:get-kanji` (gf — `dict.lisp:15-16`).
//!
//! Generic function returning "most popular kanji representation"
//! for a word. Four method bodies upstream:
//!
//! - **`((obj entry))`** at `dict.lisp:51-53` — ported on
//!   [`Entry::get_kanji`] (in [`super::entry_dao`]); when `n_kanji > 0`,
//!   selects the `kanji_text` row at `ord = 0` and returns its `text`.
//! - **`((obj kanji-text))`** at `dict.lisp:108-109` — slot read,
//!   `(text obj)`. The kanji-text holds the kanji surface itself.
//! - **`((obj kana-text))`** at `dict.lisp:153-155` — calls
//!   [`best_kanji_conj`] and unwraps the `:null` sentinel to `None`.
//! - **`((obj counter-text))`** at `dict-counters.lisp:61-62` —
//!   concatenates `(number-to-kanji (number-value obj))` with the
//!   counter morpheme `(counter-text obj)` (the `text` slot on
//!   counter-text).
//!
//! `proxy-text` and `compound-text` have no get-kanji method
//! upstream; CLOS raises `no-applicable-method` when reached. The
//! Rust dispatcher matches with `unreachable!` so the same logic
//! invariant fails loud.
//!
//! Two distinct callable surfaces in Rust, mirroring the receiver
//! polymorphism that the upstream gf collapses:
//!
//! - [`get_kanji`] free fn — dispatcher over [`KaniWordDispatchEnum`];
//!   covers the kanji-text, kana-text, counter-text methods. Async
//!   because the kana-text branch reaches the database through
//!   [`best_kanji_conj`].
//! - [`Entry::get_kanji`] inherent method — for the entry receiver.

use crate::conn::kani_context::KaniranContext;
use crate::dict::best_kanji_conj::best_kanji_conj;
use crate::dict::entry_dao::Entry;
use crate::dict::kani_word::KaniWordDispatchEnum;
use crate::numbers::constants::{DIGIT_KANJI_DEFAULT, POWER_KANJI};
use crate::numbers::number_to_kanji::number_to_kanji;

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
