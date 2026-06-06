//! Port of `ichiran/dict:word-info-rec-find` (`dict.lisp:1409`).
//!
//! Walks `wi-list` and each word-info's `components` recursively,
//! returning every `(matched, following)` pair where `matched`
//! satisfies `test-fn`. The `following` word-info is the one after
//! `matched` in its list, falling back to the parent's next for a
//! matched last component, or `None` at the very end.

use crate::dict::word_info_class::WordInfo;

pub fn word_info_rec_find<'a, F>(
    wi_list: &'a [WordInfo],
    test_fn: &F,
) -> Vec<(&'a WordInfo, Option<&'a WordInfo>)>
where
    F: Fn(&WordInfo) -> bool,
{
    let mut result = Vec::new();
    // dict.lisp:1411 (loop for (wi wi-next) on wi-list)
    for (idx, wi) in wi_list.iter().enumerate() {
        let wi_next = wi_list.get(idx + 1);
        // dict.lisp:1412 (for components = (word-info-components wi))
        let components = &wi.components;
        // dict.lisp:1413 (if (funcall test-fn wi) nconc (list (cons wi wi-next)))
        if test_fn(wi) {
            result.push((wi, wi_next));
        }
        // dict.lisp:1414-1415 (nconc (loop for (wf . wf-next) in (word-info-rec-find components test-fn)
        //                                collect (cons wf (or wf-next wi-next))))
        for (wf, wf_next) in word_info_rec_find(components, test_fn) {
            result.push((wf, wf_next.or(wi_next)));
        }
    }
    result
}

#[cfg(test)]
mod tests {
    //! REPL fixtures (.103, ichiran/dict:word-info-rec-find), 2026-05-25.
    //! Each case runs `word-info-rec-find` over a synthetic word-info
    //! tree (parent `P` with components `こ` / `ねこ`, sibling `S`) and
    //! a `test-fn` matching by text — the same construction probed in
    //! the REPL. Pairs are compared by `(text, next-text)`.
    use super::*;
    use crate::dict::word_info_class::{WordInfo, WordInfoType};

    fn wi(text: &str, components: Vec<WordInfo>) -> WordInfo {
        WordInfo {
            kind: WordInfoType::Kana,
            text: text.to_string(),
            components,
            ..Default::default()
        }
    }

    fn tree() -> Vec<WordInfo> {
        let c1 = wi("こ", Vec::new());
        let c2 = wi("ねこ", Vec::new());
        let parent = wi("P", vec![c1, c2]);
        let sibling = wi("S", Vec::new());
        vec![parent, sibling]
    }

    fn pairs<'a>(
        result: &[(&'a WordInfo, Option<&'a WordInfo>)],
    ) -> Vec<(String, Option<String>)> {
        result
            .iter()
            .map(|(car, cdr)| (car.text.clone(), cdr.map(|wi| wi.text.clone())))
            .collect()
    }

    #[test]
    fn rec_find_paths() {
        let tree = tree();
        let matches = |texts: &'static [&'static str]| {
            move |wi: &WordInfo| texts.contains(&wi.text.as_str())
        };

        // all-match: ((P . S) (こ . ねこ) (ねこ . S)) — parent emits before
        // its components; the last component's nil cdr falls back to wi-next S.
        assert_eq!(
            pairs(&word_info_rec_find(&tree, &matches(&["こ", "ねこ", "P"]))),
            vec![
                ("P".into(), Some("S".into())),
                ("こ".into(), Some("ねこ".into())),
                ("ねこ".into(), Some("S".into())),
            ]
        );

        // comp-only: parent fails the test; only the components match.
        assert_eq!(
            pairs(&word_info_rec_find(&tree, &matches(&["こ", "ねこ"]))),
            vec![
                ("こ".into(), Some("ねこ".into())),
                ("ねこ".into(), Some("S".into())),
            ]
        );

        // last-only: S is the final element → cdr is nil.
        assert_eq!(
            pairs(&word_info_rec_find(&tree, &matches(&["S"]))),
            vec![("S".into(), None)]
        );

        // parent-only: just the top-level match.
        assert_eq!(
            pairs(&word_info_rec_find(&tree, &matches(&["P"]))),
            vec![("P".into(), Some("S".into()))]
        );

        // no-match / empty list both yield nothing.
        assert!(word_info_rec_find(&tree, &|_: &WordInfo| false).is_empty());
        assert!(word_info_rec_find(&[], &|_: &WordInfo| true).is_empty());
    }
}
