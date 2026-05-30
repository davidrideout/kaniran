//! Port of `ichiran/dict:def-simple-suffix` (`dict-grammar.lisp:340-368`).
//!
//! ```lisp
//! (defmacro def-simple-suffix (name keyword
//!                              (&key (stem 0) (score 0) (connector ""))
//!                                 (root-var &optional suf-var patch-var)
//!                              &body get-primary-words)
//!   …
//!   `(defsuffix ,name ,keyword (,root-var ,suf-var ,suf)
//!      (let* ((*suffix-map-temp* ,(if (= stem 0) '*suffix-map-temp* nil))
//!             (,patch-var nil)
//!             (,primary-words (progn ,@get-primary-words)))
//!        (mapcar (lambda (pw &aux score-base)
//!                  (when (listp pw)
//!                    (setf score-base (second pw)
//!                          pw (first pw)))
//!                  (adjoin-word pw ,suf
//!                               :text (concatenate 'string ,root-var ,suf-var)
//!                               :kana (let ((k (get-kana pw)))
//!                                       (concatenate 'string
//!                                                    (if ,patch-var
//!                                                        (concatenate 'string
//!                                                                     (destem k (length (car ,patch-var)))
//!                                                                     (cdr ,patch-var))
//!                                                        (destem k ,stem))
//!                                                    ,connector
//!                                                    ,suf-var))
//!                               :score-mod ,score
//!                               :score-base score-base))
//!                ,primary-words))))
//! ```
//!
//! CONVENTIONS §4.6 case (c): a code-emitting DSL macro. [`PrimaryWord`]
//! mirrors the `(when (listp pw) …)` two-shape input — bare word or
//! `(word score-base)` pair (the latter produced by
//! `pair-words-by-conj` callers like `suffix-rashii`). Each ported
//! `(def-simple-suffix …)` callsite computes its `primary_words` then
//! calls [`def_simple_suffix_body`] with a [`DefSimpleSuffixOpts`]
//! carrying the macro's keyword arguments.
//!
//! ## Divergences from Lisp
//!
//! - **Ctx-injected** per CONVENTIONS §4.8.
//! - **`*suffix-map-temp*` rebinding on `stem != 0` is the caller's
//!   responsibility.** The Lisp `(let* ((*suffix-map-temp* nil)) …)`
//!   wraps the primary-words producer; in Rust, callers with
//!   `stem != 0` construct a rebound ctx (per CONVENTIONS §4.10) and
//!   pass that rebound ctx to their primary-words producer before
//!   handing the result to this helper. The currently-ported callsites
//!   all use `stem = 0` (the no-op rebind branch), so the slot does
//!   not yet exist on `KaniranContext`.

use crate::characters::char_classes::CharClass;
use crate::characters::destem::destem;
use crate::conn::kani_context::KaniranContext;
use crate::dict::adjoin_word::adjoin_word;
use crate::dict::compound_text_class::{CompoundText, ScoreMod};
use crate::dict::get_kana::get_kana;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kani_word::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};

/// Mirrors the macro's `(when (listp pw) (setf score-base (second pw)
/// pw (first pw)))` two-shape input. A bare word is `Bare(w)`; a
/// `(word score-base)` cons cell from `pair-words-by-conj` style
/// producers is `WithScoreBase(w, base)`.
pub enum PrimaryWord {
    Bare(KaniWordDispatchEnum),
    WithScoreBase(KaniWordDispatchEnum, KaniWordDispatchEnum),
}

impl From<KaniWordDispatchEnum> for PrimaryWord {
    fn from(word: KaniWordDispatchEnum) -> Self {
        PrimaryWord::Bare(word)
    }
}

/// The `def-simple-suffix` macro's keyword arguments + the
/// `patch-var` slot. `patch` mirrors a non-nil `,patch-var` value
/// `(car . cdr)`; the helper's kana branch is
/// `(destem k (length car)) + cdr` for `Some((car, cdr))` and
/// `(destem k stem)` for `None`.
pub struct DefSimpleSuffixOpts<'a> {
    pub stem: usize,
    pub score: ScoreMod,
    pub connector: &'a str,
    pub patch: Option<(&'a str, &'a str)>,
}

pub async fn def_simple_suffix_body(
    ctx: &KaniranContext,
    primary_words: Vec<PrimaryWord>,
    root: &str,
    suffix: &str,
    kf: &KanaText,
    opts: &DefSimpleSuffixOpts<'_>,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    let mut out = Vec::with_capacity(primary_words.len());
    for entry in primary_words {
        // dict-grammar.lisp:352-354 (when (listp pw) (setf score-base (second pw) pw (first pw)))
        let (pw, score_base) = match entry {
            PrimaryWord::Bare(word) => (word, None),
            PrimaryWord::WithScoreBase(word, base) => (word, Some(base)),
        };

        // dict-grammar.lisp:357 (let ((k (get-kana pw))) …); nil → ""
        let pw_kana = get_kana(ctx, &pw).await?.unwrap_or_default();

        // dict-grammar.lisp:358-363 — patch-or-stem branch.
        let pw_kana_trimmed = match opts.patch {
            // (concatenate 'string (destem k (length (car patch-var))) (cdr patch-var))
            Some((car, cdr)) => {
                let car_len = car.chars().count();
                format!("{}{}", destem(&pw_kana, car_len, CharClass::Kana), cdr)
            }
            // (destem k stem)
            None => destem(&pw_kana, opts.stem, CharClass::Kana),
        };

        // dict-grammar.lisp:356 — (:text (concatenate 'string root suf-var))
        let text = format!("{}{}", root, suffix);
        // dict-grammar.lisp:357-365 — (:kana (concatenate 'string <pw_kana_trimmed> connector suf-var))
        let kana = format!("{}{}{}", pw_kana_trimmed, opts.connector, suffix);

        // dict-grammar.lisp:355 — (adjoin-word pw suf :text … :kana … :score-mod score :score-base score-base)
        let compound = adjoin_word(
            ctx,
            pw,
            KaniSimpleTextDispatchEnum::Kana(kf.clone()),
            Some(text),
            Some(kana),
            Some(opts.score.clone()),
            score_base,
        )
        .await?;
        out.push(compound);
    }
    Ok(out)
}
