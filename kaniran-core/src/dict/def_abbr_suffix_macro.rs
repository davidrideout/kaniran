//! Port of `ichiran/dict:def-abbr-suffix` (`dict-grammar.lisp:547-579`).
//!
//! ```lisp
//! (defmacro def-abbr-suffix (name keyword stem
//!                            (root-var &optional suf-var patch-var)
//!                            &body get-primary-words)
//!   …
//!   `(defsuffix ,name ,keyword (,root-var ,suf-var ,suf)
//!      (declare (ignore ,suf))
//!      (let* ((*suffix-map-temp* nil)
//!             (,patch-var nil)
//!             (,primary-words (progn ,@get-primary-words)))
//!        (mapcar (lambda (pw)
//!                  (let ((text (concatenate 'string ,root-var ,suf-var))
//!                        (kana (let ((k (get-kana pw)))
//!                                (concatenate 'string
//!                                             (if ,patch-var
//!                                                 (concatenate 'string
//!                                                              (destem k (length (car ,patch-var)))
//!                                                              (cdr ,patch-var))
//!                                                 (destem k ,stem))
//!                                             ,suf-var))))
//!                    (etypecase pw
//!                      (simple-text
//!                       (make-instance 'proxy-text
//!                                      :source pw
//!                                      :text text
//!                                      :kana kana
//!                                      :hintedp t))
//!                      (compound-text
//!                       (with-slots ((stext text) (skana kana)) pw
//!                         (setf stext text skana kana))
//!                       pw))))
//!                ,primary-words))))
//! ```
//!
//! CONVENTIONS §4.6 case (c). [`def_abbr_suffix_body`] holds the
//! mapcar tail; each per-callsite port computes its primary-words
//! (under a rebound ctx with `suffix_map_temp = None`) then calls
//! the helper with the macro's `stem` and optional `patch-var`
//! value.
//!
//! ## Divergences from Lisp
//!
//! - **Ctx-injected** per CONVENTIONS §4.8.
//! - **`*suffix-map-temp* nil` rebind is the caller's responsibility.**
//!   Each `(def-abbr-suffix …)` callsite builds `ctx2 =
//!   ctx.with_suffix_map_temp(None)` (CONVENTIONS §4.10) and threads
//!   that rebound ctx into its primary-words producer and into this
//!   helper.
//! - **`patch-var` exposed as a parameter** of the helper (not a
//!   mutated lexical). Only `abbr-nx` ever sets it; every other
//!   callsite passes `None`.

use crate::characters::char_class_type::CharClass;
use crate::characters::destem::destem;
use crate::conn::kani_context::KaniranContext;
use crate::dict::get_kana::get_kana;
use crate::dict::kani_word::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use crate::dict::proxy_text_class::ProxyText;
use crate::dict::simple_text_class::SimpleText;

pub async fn def_abbr_suffix_body(
    ctx: &KaniranContext,
    primary_words: Vec<KaniWordDispatchEnum>,
    root: &str,
    suf_var: &str,
    stem: usize,
    patch: Option<(&str, &str)>,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    let mut out = Vec::with_capacity(primary_words.len());
    for pw in primary_words {
        // dict-grammar.lisp:559 — (concatenate 'string root-var suf-var)
        let text = format!("{}{}", root, suf_var);
        // dict-grammar.lisp:560 — (let ((k (get-kana pw))) …); nil → ""
        let pw_kana = get_kana(ctx, &pw).await?.unwrap_or_default();
        // dict-grammar.lisp:561-566 — (if patch-var (destem k (length car) + cdr) (destem k stem))
        let pw_kana_trimmed = match patch {
            Some((car, cdr)) => {
                let car_len = car.chars().count();
                format!("{}{}", destem(&pw_kana, car_len, CharClass::Kana), cdr)
            }
            None => destem(&pw_kana, stem, CharClass::Kana),
        };
        // dict-grammar.lisp:567 — (concatenate 'string <pw_kana_trimmed> suf-var)
        let kana = format!("{}{}", pw_kana_trimmed, suf_var);
        // dict-grammar.lisp:568-578 — (etypecase pw (simple-text …) (compound-text …))
        match pw {
            // dict-grammar.lisp:569-574 (simple-text arm) —
            // (make-instance 'proxy-text :source pw :text :kana :hintedp t)
            KaniWordDispatchEnum::Kanji(k) => {
                out.push(KaniWordDispatchEnum::Proxy(ProxyText {
                    text,
                    kana,
                    source: Box::new(KaniSimpleTextDispatchEnum::Kanji(k)),
                    state: SimpleText {
                        conjugations: None,
                        hintedp: true,
                    },
                }));
            }
            KaniWordDispatchEnum::Kana(k) => {
                out.push(KaniWordDispatchEnum::Proxy(ProxyText {
                    text,
                    kana,
                    source: Box::new(KaniSimpleTextDispatchEnum::Kana(k)),
                    state: SimpleText {
                        conjugations: None,
                        hintedp: true,
                    },
                }));
            }
            KaniWordDispatchEnum::Proxy(p) => {
                out.push(KaniWordDispatchEnum::Proxy(ProxyText {
                    text,
                    kana,
                    source: Box::new(KaniSimpleTextDispatchEnum::Proxy(p)),
                    state: SimpleText {
                        conjugations: None,
                        hintedp: true,
                    },
                }));
            }
            // dict-grammar.lisp:575-578 (compound-text arm) —
            // (with-slots ((stext text) (skana kana)) pw (setf stext text skana kana)) pw
            KaniWordDispatchEnum::Compound(mut c) => {
                c.text = text;
                c.kana = kana;
                out.push(KaniWordDispatchEnum::Compound(c));
            }
            // dict-grammar.lisp:568 (etypecase) — counter-text is
            // not a `simple-text` or `compound-text`; upstream raises
            // a TYPE-ERROR. No upstream callsite reaches here (abbr
            // primary-words sources are find-word-full / find-word-
            // with-conj-prop / find-word-conj-of with no :counter).
            KaniWordDispatchEnum::Counter(_) => {
                panic!("def-abbr-suffix etypecase received counter-text")
            }
        }
    }
    Ok(out)
}
