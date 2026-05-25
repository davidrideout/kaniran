//! Port of `ichiran:romanize*` (`romanize.lisp:273-290`).
//!
//! ```lisp
//! (defun romanize* (input &key (method *default-romanization-method*) (limit 5) (wordprop-fn (constantly nil)))
//!   (setf input (normalize input :context method))
//!   (loop for (split-type . split-text) in (basic-split input)
//!      collect
//!        (if (eql split-type :word)
//!            (mapcar (lambda (pair)
//!                      (let ((word-list (car pair))
//!                            (score (cdr pair)))
//!                        (list
//!                         (mapcar (lambda (word)
//!                                   (let* ((romanized (romanize-word-info word :method method))
//!                                          (prop (funcall wordprop-fn romanized word)))
//!                                     (list romanized word prop)))
//!                                 word-list)
//!                         score)))
//!                    (dict-segment split-text :limit limit))
//!            split-text)))
//! ```
//!
//! Each `basic-split` segment becomes a [`RomanizeStarSegment`]: a `:misc`
//! split is `Misc(text)`, a `:word` split is `Word` holding one
//! `(word-prop-list, score)` pair per `dict-segment` alternative. The
//! `(list romanized word prop)` triple becomes a `(String, WordInfo, P)`
//! tuple; `wordprop-fn` is a generic `Fn(&str, &WordInfo) -> P` whose result
//! is the `prop`. `:context method` reduces to a [`NormalizationContext`].

use super::kani_romanize_method::KaniRomanizeMethod;
use super::romanize_word_info::romanize_word_info;
use crate::characters::basic_split::{basic_split, SegmentKind};
use crate::characters::normalize::normalize;
use crate::characters::to_normal_char::NormalizationContext;
use crate::conn::kani_context::KaniranContext;
use crate::dict::dict_segment::dict_segment;
use crate::dict::word_info_class::WordInfo;

#[derive(Debug)]
pub enum RomanizeStarSegment<P> {
    Misc(String),
    Word(Vec<(Vec<(String, WordInfo, P)>, i32)>),
}

pub async fn romanize_star_<P>(
    ctx: &KaniranContext,
    input: &str,
    method: KaniRomanizeMethod<'_>,
    limit: Option<usize>,
    wordprop_fn: impl Fn(&str, &WordInfo) -> P,
) -> Result<Vec<RomanizeStarSegment<P>>, sqlx::Error> {
    // (normalize input :context method) — characters.lisp:230 tests (eql context :kana)
    let context = match method {
        KaniRomanizeMethod::Kana => NormalizationContext::Kana,
        KaniRomanizeMethod::Method(_) => NormalizationContext::Default,
    };
    let input = normalize(input, context);
    let mut result: Vec<RomanizeStarSegment<P>> = Vec::new();
    for (split_type, split_text) in basic_split(&input) {
        if split_type == SegmentKind::Word {
            let mut alternatives = Vec::new();
            for (word_list, score) in dict_segment(ctx, &split_text, limit).await? {
                let mut word_props = Vec::new();
                for word in word_list {
                    let romanized = romanize_word_info(&word, method);
                    let prop = wordprop_fn(&romanized, &word);
                    word_props.push((romanized, word, prop));
                }
                alternatives.push((word_props, score));
            }
            result.push(RomanizeStarSegment::Word(alternatives));
        } else {
            result.push(RomanizeStarSegment::Misc(split_text));
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    //! REPL fixtures (.103, `ichiran:romanize*`, 2026-05-24). Run with
    //! `cargo test ... -- --test-threads=1` per the DB-test convention.
    use super::*;
    use crate::core::_star_hepburn_traditional_star_::hepburn_traditional;
    use crate::core::generic_romanization_class::RomanizationMethod;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
    }

    fn traditional() -> KaniRomanizeMethod<'static> {
        KaniRomanizeMethod::Method(RomanizationMethod::TraditionalHepburn(hepburn_traditional()))
    }

    /// Projection of the result with `prop = ()` dropped: `:misc` splits map
    /// to their text, `:word` splits to `(score, [(romanized, word.text)])`
    /// per alternative — the shape this function owns.
    #[derive(Debug, PartialEq)]
    enum SegShape {
        Misc(String),
        Word(Vec<(i32, Vec<(String, String)>)>),
    }

    fn shape(result: &[RomanizeStarSegment<()>]) -> Vec<SegShape> {
        result
            .iter()
            .map(|segment| match segment {
                RomanizeStarSegment::Misc(text) => SegShape::Misc(text.clone()),
                RomanizeStarSegment::Word(alternatives) => SegShape::Word(
                    alternatives
                        .iter()
                        .map(|(word_props, score)| {
                            (
                                *score,
                                word_props
                                    .iter()
                                    .map(|(rom, word, _)| (rom.clone(), word.text.clone()))
                                    .collect(),
                            )
                        })
                        .collect(),
                ),
            })
            .collect()
    }

    #[tokio::test]
    async fn romanize_star_full_structure() {
        // REPL `(romanize* "Hello 世界！")`: a latin misc split, a word split
        // with 5 distinct-score alternatives, and a "! " misc split. The
        // alternatives carry the unromanizable kanji verbatim (e.g. "世" /
        // "世界") when no reading wins. wordprop-fn = (constantly nil) -> ().
        let ctx = ctx().await;
        let result = romanize_star_(&ctx, "Hello 世界！", traditional(), None, |_, _| ())
            .await
            .unwrap();
        let word = |rom: &str, text: &str| (rom.to_string(), text.to_string());
        assert_eq!(
            shape(&result),
            vec![
                SegShape::Misc("Hello ".to_string()),
                SegShape::Word(vec![
                    (325, vec![word("sekai", "世界")]),
                    (23, vec![word("yo", "世"), word("kai", "界")]),
                    (-487, vec![word("世", "世"), word("kai", "界")]),
                    (-490, vec![word("yo", "世"), word("界", "界")]),
                    (-1000, vec![word("世界", "世界")]),
                ]),
                SegShape::Misc("! ".to_string()),
            ]
        );
    }

    #[tokio::test]
    async fn romanize_star_misc_in_middle() {
        // REPL `(romanize* "ABCは試験的な略語です。")`: leading latin misc,
        // a word split, trailing ". " misc. Assert the segment kinds and the
        // top alternative's word sequence (top score 1091 is unique).
        let ctx = ctx().await;
        let result = romanize_star_(&ctx, "ABCは試験的な略語です。", traditional(), None, |_, _| ())
            .await
            .unwrap();
        let shaped = shape(&result);
        assert!(matches!(shaped[0], SegShape::Misc(ref t) if t == "ABC"));
        assert!(matches!(shaped[2], SegShape::Misc(ref t) if t == ". "));
        let SegShape::Word(ref alternatives) = shaped[1] else {
            panic!("segment 1 should be a word split, got {:?}", shaped[1]);
        };
        assert_eq!(alternatives.len(), 5);
        let (top_score, ref top_words) = alternatives[0];
        assert_eq!(top_score, 1091);
        let roms: Vec<&str> = top_words.iter().map(|(rom, _)| rom.as_str()).collect();
        assert_eq!(roms, vec!["wa", "shikenteki", "na", "ryakugo", "desu"]);
    }

    #[tokio::test]
    async fn romanize_star_wordprop_fn_receives_romanized_and_word() {
        // wordprop-fn is funcall'd with (romanized word); its result is the
        // prop in each triple. Here it returns (romanized-byte-len, word.text)
        // so both arguments are observed: "sekai" (5 bytes) over word "世界".
        let ctx = ctx().await;
        let result = romanize_star_(&ctx, "世界", traditional(), Some(1), |rom, word| {
            (rom.len(), word.text.clone())
        })
        .await
        .unwrap();
        let RomanizeStarSegment::Word(ref alternatives) = result[0] else {
            panic!("expected a word split, got {:?}", result[0]);
        };
        let (ref word_props, score) = alternatives[0];
        assert_eq!(score, 325);
        let (ref rom, ref word, ref prop) = word_props[0];
        assert_eq!(rom, "sekai");
        assert_eq!(word.text, "世界");
        assert_eq!(*prop, (5, "世界".to_string()));
    }
}
