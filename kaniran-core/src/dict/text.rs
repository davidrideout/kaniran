//! Port of `ichiran/dict:text` — implicit generic function created
//! by `:reader text` slot options on every simple-text family class
//! plus compound-text, with an explicit override on counter-text at
//! `dict-counters.lisp:58-59`:
//!
//! ```lisp
//! (defmethod text ((obj counter-text))
//!   (concatenate 'string (number-text obj) (counter-text obj)))
//! ```
//!
//! For every variant except counter-text the method is the
//! auto-generated slot reader returning the bare `text` slot
//! (e.g. `"だ"`). For counter-text the explicit override returns
//! the digit prefix concatenated with the counter morpheme
//! (e.g. `"1人"` rather than `"人"`). Callers including
//! `calc-score`'s katakana detector (`dict.lisp:796`) and
//! `super::true_text::true_text`'s `T`-method depend on the
//! concatenation.
//!
//! Return type is [`Cow<str>`] because the counter-text branch
//! must allocate the concatenation; every other branch borrows
//! from the input.
//!
//! Data-quality note: the upstream introspector does not emit a
//! md file for this gf — implicit generic functions (created by
//! `:reader X` with no explicit `(defgeneric X ...)`) are not
//! surfaced by `sb-introspect:find-definition-sources-by-name`
//! against `:generic-function` when no top-level defgeneric form
//! exists. Consequence: this symbol does not appear in
//! `symbols.csv` and is invisible to `query.py audit-signatures`,
//! `deps`, and `plan`. Tracked separately as a graph hole.

use std::borrow::Cow;

use super::kani_word::KaniWordDispatchEnum;

pub fn text<'a>(obj: &'a KaniWordDispatchEnum) -> Cow<'a, str> {
    match obj {
        KaniWordDispatchEnum::Kanji(k) => Cow::Borrowed(&k.text),
        KaniWordDispatchEnum::Kana(k) => Cow::Borrowed(&k.text),
        KaniWordDispatchEnum::Proxy(p) => Cow::Borrowed(&p.text),
        KaniWordDispatchEnum::Compound(c) => Cow::Borrowed(&c.text),
        KaniWordDispatchEnum::Counter(c) => {
            let base = c.base();
            Cow::Owned(format!("{}{}", base.number_text, base.text))
        }
    }
}
