//! Port of `ichiran/dict:word-info-str` (`dict.lisp:1745`).
//!
//! Renders a [`WordInfo`] to its human-readable string form — one
//! numbered block per component for an alternative word-info,
//! otherwise a single block.

use std::fmt::Write;

use super::get_senses_str::get_senses_str;
use super::grammar::suffix::init::get_suffix_description;
use super::print_conj_info::print_conj_info;
use super::simple_text_class::WordConjugations;
use super::word_info_class::{WordInfo, WordInfoSeq};
use crate::conn::kani_context::KaniranContext;

pub async fn word_info_str(
    ctx: &KaniranContext,
    word_info: &WordInfo,
) -> Result<String, sqlx::Error> {
    let mut s = String::new();
    if word_info.alternative {
        // dict.lisp:1775-1779 (loop for wi … for i from 1 when (> i 1) do (terpri s) do (format s "<~a>. " i) (inner wi nil nil))
        for (index, wi) in word_info.components.iter().enumerate() {
            let i = index + 1;
            if i > 1 {
                s.push('\n');
            }
            write!(s, "<{}>. ", i).unwrap();
            inner(ctx, wi, false, false, &mut s).await?;
        }
    } else {
        inner(ctx, word_info, false, false, &mut s).await?;
    }
    Ok(s)
}

// dict.lisp:1748 (labels inner (word-info &optional suffix marker))
async fn inner(
    ctx: &KaniranContext,
    word_info: &WordInfo,
    suffix: bool,
    marker: bool,
    s: &mut String,
) -> Result<(), sqlx::Error> {
    if marker {
        s.push_str(" * ");
    }
    // (princ (reading-str word-info) s)
    s.push_str(word_info.reading_str().as_deref().unwrap_or("NIL"));
    if !word_info.components.is_empty() {
        // dict.lisp:1754 (format s " Compound word: ~{~a~^ + ~}" (mapcar #'word-info-text components))
        let texts: Vec<&str> = word_info
            .components
            .iter()
            .map(|comp| comp.text.as_str())
            .collect();
        write!(s, " Compound word: {}", texts.join(" + ")).unwrap();
        // dict.lisp:1755-1757 (dolist (comp components) (terpri s) (inner comp (not (word-info-primary comp)) t))
        for comp in &word_info.components {
            s.push('\n');
            Box::pin(inner(ctx, comp, !comp.primary, true, s)).await?;
        }
    } else if let Some((value, _ordinal)) = &word_info.counter {
        // dict.lisp:1759-1763 (destructuring-bind (value ordinal) (word-info-counter …) (terpri s) (princ value s) …)
        s.push('\n');
        s.push_str(value);
        if let Some(seq) = word_info_seq_single(word_info) {
            s.push('\n');
            s.push_str(&get_senses_str(ctx, seq).await?);
        }
    } else {
        // dict.lisp:1765-1774
        let seq = word_info_seq_single(word_info);
        let conjs = word_info.conjugations.as_ref();
        // (cond ((and suffix (setf desc (get-suffix-description seq))) …)
        //       ((or (not conjs) (eql conjs :root)) …))
        let mut desc: Option<&'static str> = None;
        if suffix {
            if let Some(seq) = seq {
                desc = get_suffix_description(ctx, seq);
            }
        }
        if let Some(desc) = desc {
            write!(s, "  [suffix]: {} ", desc).unwrap();
        } else if conjs.is_none() || matches!(conjs, Some(WordConjugations::Root)) {
            s.push('\n');
            match seq {
                Some(seq) => s.push_str(&get_senses_str(ctx, seq).await?),
                None => s.push_str("???"),
            }
        }
        // (when seq (print-conj-info seq :out s :conjugations conjs))
        if let Some(seq) = seq {
            print_conj_info(ctx, seq, conjs, s).await?;
        }
    }
    Ok(())
}

// (word-info-seq word-info) is a single int or nil in the counter and default
// branches; a list seq only occurs on a compound/alternative word-info, which
// the components branch and top-level alternative loop handle first.
fn word_info_seq_single(word_info: &WordInfo) -> Option<i32> {
    match &word_info.seq {
        Some(WordInfoSeq::Single(seq)) => Some(*seq),
        None => None,
        Some(WordInfoSeq::Multi(_)) => {
            panic!("word-info-str: non-compound word-info seq is WordInfoSeq::Multi")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::word_info_class::{WordInfoKana, WordInfoType};
    use std::sync::Arc;

    async fn ctx_from_env() -> Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    fn single(reading: &str) -> Option<WordInfoKana> {
        Some(WordInfoKana::Single(reading.to_string()))
    }

    /// REPL fixtures (.103, `(word-info-str (make-instance 'word-info …))`),
    /// 2026-05-24, after `(init-suffixes t)`. Each row builds one word-info and
    /// pins the exact output (blank lines included). Covers:
    /// - A: default branch, no conjugations → senses.
    /// - B: default branch, conjugations nil → empty senses + full conj-info.
    /// - C: conjugations `:root` → conj display suppressed (test2 still fires).
    /// - D: seq nil → "???".
    /// - E: counter + seq → value then senses.
    /// - F: counter, no seq → value only.
    /// - G: compound, non-primary suffix component → marker, suffix description.
    /// - G2: compound, non-primary component without a suffix description →
    ///   marker, falls through to senses.
    /// - H: alternative → "<i>. " prefixes, second reading a counter.
    #[tokio::test]
    async fn word_info_str_fixtures() {
        use WordInfoType::{Kana, Kanji};
        let ctx = ctx_from_env().await;

        let compound = |text: &str, kana: &str, seqs: &[i32], comps: Vec<WordInfo>| WordInfo {
            kind: Kanji,
            text: text.to_string(),
            kana: single(kana),
            seq: Some(WordInfoSeq::Multi(
                seqs.iter().map(|s| Some(WordInfoSeq::Single(*s))).collect(),
            )),
            components: comps,
            ..Default::default()
        };

        let cases: Vec<(&str, WordInfo, &str)> = vec![
            (
                "A",
                WordInfo {
                    kind: Kanji,
                    text: "日本".to_string(),
                    kana: single("にほん"),
                    seq: Some(WordInfoSeq::Single(1582710)),
                    ..Default::default()
                },
                "日本 【にほん】\n1. [n] Japan",
            ),
            (
                "B",
                WordInfo {
                    kind: Kanji,
                    text: "食べた".to_string(),
                    kana: single("たべた"),
                    seq: Some(WordInfoSeq::Single(10092229)),
                    ..Default::default()
                },
                "食べた 【たべた】\n\n[ Conjugation: [v1] Past (~ta) Affirmative Plain\n  食べる 【たべる】 : to eat ]",
            ),
            (
                "C",
                WordInfo {
                    kind: Kanji,
                    text: "食べた".to_string(),
                    kana: single("たべた"),
                    seq: Some(WordInfoSeq::Single(10092229)),
                    conjugations: Some(WordConjugations::Root),
                    ..Default::default()
                },
                "食べた 【たべた】\n",
            ),
            (
                "D",
                WordInfo {
                    kind: Kana,
                    text: "ねこねこ".to_string(),
                    kana: single("ねこねこ"),
                    seq: None,
                    ..Default::default()
                },
                "ねこねこ\n???",
            ),
            (
                "E",
                WordInfo {
                    kind: Kanji,
                    text: "三冊".to_string(),
                    kana: single("さんさつ"),
                    seq: Some(WordInfoSeq::Single(1298520)),
                    counter: Some(("Value: 3".to_string(), false)),
                    ..Default::default()
                },
                "三冊 【さんさつ】\nValue: 3\n1. [ctr] counter for books\n2. [n] volume",
            ),
            (
                "F",
                WordInfo {
                    kind: Kanji,
                    text: "三".to_string(),
                    kana: single("さん"),
                    seq: None,
                    counter: Some(("Value: 3".to_string(), false)),
                    ..Default::default()
                },
                "三 【さん】\nValue: 3",
            ),
            (
                "G",
                compound(
                    "食べたい",
                    "たべたい",
                    &[1358280, 2017560],
                    vec![
                        WordInfo {
                            kind: Kanji,
                            text: "食べ".to_string(),
                            kana: single("たべ"),
                            seq: Some(WordInfoSeq::Single(1358280)),
                            primary: true,
                            ..Default::default()
                        },
                        WordInfo {
                            kind: Kana,
                            text: "たい".to_string(),
                            kana: single("たい"),
                            seq: Some(WordInfoSeq::Single(2017560)),
                            primary: false,
                            ..Default::default()
                        },
                    ],
                ),
                "食べたい 【たべたい】 Compound word: 食べ + たい\n * 食べ 【たべ】\n1. [v1,vt] to eat\n2. [vt,v1] to live on (e.g. a salary); to live off; to subsist on\n * たい  [suffix]: want to... / would like to... ",
            ),
            (
                "G2",
                compound(
                    "日本語",
                    "にほんご",
                    &[1582710, 1576050],
                    vec![
                        WordInfo {
                            kind: Kanji,
                            text: "日本".to_string(),
                            kana: single("にほん"),
                            seq: Some(WordInfoSeq::Single(1582710)),
                            primary: true,
                            ..Default::default()
                        },
                        WordInfo {
                            kind: Kanji,
                            text: "語".to_string(),
                            kana: single("ご"),
                            seq: Some(WordInfoSeq::Single(1576050)),
                            primary: false,
                            ..Default::default()
                        },
                    ],
                ),
                "日本語 【にほんご】 Compound word: 日本 + 語\n * 日本 【にほん】\n1. [n] Japan\n * 語 【ご】\n1. [adv,n] day before yesterday",
            ),
            (
                "H",
                WordInfo {
                    kind: Kanji,
                    text: "一人".to_string(),
                    kana: single("ひとり"),
                    seq: Some(WordInfoSeq::Multi(vec![
                        Some(WordInfoSeq::Single(1576150)),
                        Some(WordInfoSeq::Single(2149890)),
                    ])),
                    alternative: true,
                    components: vec![
                        WordInfo {
                            kind: Kanji,
                            text: "一人".to_string(),
                            kana: single("ひとり"),
                            seq: Some(WordInfoSeq::Single(1576150)),
                            primary: false,
                            ..Default::default()
                        },
                        WordInfo {
                            kind: Kanji,
                            text: "一人".to_string(),
                            kana: single("ひとり"),
                            seq: Some(WordInfoSeq::Single(2149890)),
                            counter: Some(("Value: 1".to_string(), false)),
                            primary: false,
                            ..Default::default()
                        },
                    ],
                    ..Default::default()
                },
                "<1>. 一人 【ひとり】\n1. [n] 《esp. 一人, １人》 one person\n2. [n] being alone; being by oneself\n3. [n] 《esp. 独り》 being single; being unmarried\n4. [adv] by oneself; alone\n5. [adv] 《with neg. sentence》 just; only; simply\n<2>. 一人 【ひとり】\nValue: 1\n1. [ctr] counter for people",
            ),
        ];

        for (label, word_info, expected) in &cases {
            assert_eq!(
                &word_info_str(&ctx, word_info).await.unwrap(),
                expected,
                "case={label}"
            );
        }
    }
}
