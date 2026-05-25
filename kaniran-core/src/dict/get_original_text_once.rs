//! Transliteration of `ichiran/dict:get-original-text-once` (`dict.lisp:371`).
//!
//! ```lisp
//! (defun get-original-text-once (conj-datas texts)
//!   (unless (listp texts)
//!     (setf texts (list texts)))
//!   (unless (listp conj-datas)
//!     (setf conj-datas (list conj-datas)))
//!   (loop for conj-data in conj-datas
//!        nconc (loop for (txt src-txt) in (conj-data-src-map conj-data)
//!                 if (find txt texts :test 'equal) collect src-txt)))
//! ```
//!
//! For each conj-data, collects the source-text of every `src-map` pair
//! whose text is in `texts`, concatenated across all conj-datas in order.
//!
//! Diverges from the upstream lambda list `(conj-datas texts)` by taking
//! both arguments as slices; the Lisp coerces a lone conj-data / text to a
//! one-element list internally, the Rust caller wraps (matching
//! [`super::get_conj_data`]'s `texts: &[&str]`).

use super::conj_data_struct::ConjData;

pub fn get_original_text_once(conj_datas: &[ConjData], texts: &[&str]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for conj_data in conj_datas {
        for (txt, src_txt) in &conj_data.src_map {
            if texts.contains(&txt.as_str()) {
                out.push(src_txt.clone());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::make_conj_data::make_conj_data;

    fn cd(pairs: &[(&str, &str)]) -> ConjData {
        make_conj_data(
            None,
            None,
            None,
            None,
            pairs
                .iter()
                .map(|(txt, src_txt)| (txt.to_string(), src_txt.to_string()))
                .collect(),
        )
    }

    /// REPL fixtures (.103, `ichiran/dict::get-original-text-once` over
    /// `make-conj-data` built from the real 食べる conj-source-reading
    /// rows), 2026-05-24. Output order tracks `src-map` iteration order,
    /// not `texts` order — both two-text rows below collect
    /// `("たべる" "食べる")` regardless of how the texts are ordered.
    #[test]
    fn get_original_text_once_fixtures() {
        let cd1 = cd(&[
            ("たべます", "たべる"),
            ("喰べます", "喰べる"),
            ("食べます", "食べる"),
        ]);
        let cd2 = cd(&[
            ("たべない", "たべる"),
            ("喰べない", "喰べる"),
            ("食べない", "食べる"),
        ]);
        let cases: &[(&[ConjData], &[&str], &[&str])] = &[
            (std::slice::from_ref(&cd1), &["食べます"], &["食べる"]),
            (std::slice::from_ref(&cd1), &["たべます"], &["たべる"]),
            (
                std::slice::from_ref(&cd1),
                &["食べます", "たべます"],
                &["たべる", "食べる"],
            ),
            (
                std::slice::from_ref(&cd1),
                &["たべます", "食べます"],
                &["たべる", "食べる"],
            ),
            (std::slice::from_ref(&cd1), &["xyz"], &[]),
            (std::slice::from_ref(&cd1), &[], &[]),
            (
                std::slice::from_ref(&cd1),
                &["たべます", "喰べます", "食べます"],
                &["たべる", "喰べる", "食べる"],
            ),
            (
                &[cd1.clone(), cd2.clone()],
                &["食べます", "食べない"],
                &["食べる", "食べる"],
            ),
            (std::slice::from_ref(&cd2), &["食べない"], &["食べる"]),
            (&[], &["食べます"], &[]),
        ];
        for (conj_datas, texts, expected) in cases {
            let actual = get_original_text_once(conj_datas, texts);
            let actual_refs: Vec<&str> = actual.iter().map(String::as_str).collect();
            assert_eq!(actual_refs.as_slice(), *expected, "texts={texts:?}");
        }
    }
}
