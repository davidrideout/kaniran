//! Port of `ichiran/dict:suffix-sou-base` (`dict-grammar.lisp:445`).
//!
//! ```lisp
//! (defmacro suffix-sou-base (root patch)
//!   `(cond ((alexandria:ends-with-subseq "なさ" ,root)
//!           (setf ,patch '("い" . "さ"))
//!           (let ((root (apply-patch ,root ,patch))
//!                 (*suffix-map-temp* nil))
//!             (find-word-with-conj-prop root (lambda (cdata)
//!                                              (conj-neg (conj-data-prop cdata))))))
//!          ((not (member ,root '("な" "よ" "よさ" "に" "き") :test 'equal))
//!           (find-word-with-conj-type ,root 13 +conj-adjective-stem+ +conj-adverbial+))))
//! ```
//!
//! Shared body for `suffix-sou` (`dict-grammar.lisp:454`) and
//! `suffix-sou+` (`dict-grammar.lisp:468`). Per CONVENTIONS §4.6 case (c),
//! factored as a single helper rather than duplicated at each callsite.
//!
//! `+conj-adjective-stem+` is `51` and `+conj-adverbial+` is `50`
//! (`dict-errata.lisp:1236-1237`); the `&rest` conj-type set is
//! `(13 51 50)`.

use crate::conn::kani_context::KaniranContext;
use crate::dict::apply_patch::apply_patch;
use crate::dict::def_simple_suffix_macro::PrimaryWord;
use crate::dict::find_word_with_conj_prop::find_word_with_conj_prop;
use crate::dict::find_word_with_conj_type::find_word_with_conj_type;

pub async fn suffix_sou_base_body(
    ctx: &KaniranContext,
    root: &str,
) -> Result<(Vec<PrimaryWord>, Option<(&'static str, &'static str)>), sqlx::Error> {
    // dict-grammar.lisp:446 (alexandria:ends-with-subseq "なさ" ,root)
    if root.ends_with("なさ") {
        // dict-grammar.lisp:447 (setf ,patch '("い" . "さ"))
        let patch = ("い", "さ");
        // dict-grammar.lisp:448 (let ((root (apply-patch ,root ,patch))
        //                              (*suffix-map-temp* nil)) …)
        let new_root = apply_patch(root, patch);
        let ctx_inner = ctx.with_suffix_map_temp(None);
        // dict-grammar.lisp:450-451 (find-word-with-conj-prop root
        //   (lambda (cdata) (conj-neg (conj-data-prop cdata))))
        let words = find_word_with_conj_prop(
            &ctx_inner,
            &new_root,
            |cd| cd.prop.as_ref().is_some_and(|p| p.neg != Some(false)),
            false,
        )
        .await?;
        let primary_words = words.into_iter().map(PrimaryWord::from).collect();
        Ok((primary_words, Some(patch)))
    } else if !matches!(root, "な" | "よ" | "よさ" | "に" | "き") {
        // dict-grammar.lisp:452-453 ((not (member ,root '("な" "よ" "よさ" "に" "き") :test 'equal))
        //   (find-word-with-conj-type ,root 13 +conj-adjective-stem+ +conj-adverbial+))
        let words = find_word_with_conj_type(ctx, root, &[13, 51, 50]).await?;
        let primary_words = words.into_iter().map(PrimaryWord::from).collect();
        Ok((primary_words, None))
    } else {
        Ok((Vec::new(), None))
    }
}
