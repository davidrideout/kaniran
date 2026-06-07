//! Port of `ichiran/dict:word-info-gloss-json` (`dict.lisp:1782`).
//!
//! Builds the gloss JSON object for a [`WordInfo`] — reading / text /
//! kana / score plus, per word kind, compound components, counter
//! value, suffix description, sense gloss, and conjugation info.

use std::future::Future;
use std::pin::Pin;

use serde_json::{Map, Value};

use crate::conn::kani_context::KaniranContext;

use super::conj_info_json::conj_info_json;
use super::filter_props::FilterPropsText;
use super::get_senses_json::get_senses_json;
use super::grammar::suffix::init::get_suffix_description;
use super::simple_text_class::WordConjugations;
use super::word_info_class::{WordInfo, WordInfoKana, WordInfoSeq};
use super::word_info_reading::word_info_reading;

// jsown renders CL `nil` as `[]`.
fn nil() -> Value {
    Value::Array(Vec::new())
}

// `(word-info-kana word-info)` handed straight to jsown: a string prints as
// itself, nil as `[]`, a list as a JSON array (mirrors word-info-json's slot).
fn kana_json(kana: &WordInfoKana) -> Value {
    match kana {
        WordInfoKana::Single(reading) => Value::String(reading.clone()),
        WordInfoKana::Multi(readings) => Value::Array(
            readings
                .iter()
                .map(|reading| reading.as_ref().map_or_else(nil, kana_json))
                .collect(),
        ),
    }
}

// The counter and `(t ...)` cond arms run only when components is empty; a
// list seq co-occurs with components (word-info-from-segment-list), handled by
// the compound arm first. Upstream `(get-senses-json (list …))` would raise a
// PostgreSQL `integer = record` error on a list seq reaching here.
fn single_seq(seq: &Option<WordInfoSeq>) -> Option<i32> {
    match seq {
        None => None,
        Some(WordInfoSeq::Single(seq)) => Some(*seq),
        Some(WordInfoSeq::Multi(_)) => panic!(
            "word-info-gloss-json: list seq in a non-compound branch \
             — upstream `(get-senses-json (list …))` raises `integer = record`"
        ),
    }
}

fn inner<'a>(
    ctx: &'a KaniranContext,
    word_info: &'a WordInfo,
    suffix: bool,
    root_only: bool,
) -> Pin<Box<dyn Future<Output = Result<Value, sqlx::Error>> + 'a>> {
    Box::pin(async move {
        let mut js = Map::new();
        // ("reading" (reading-str word-info)) / ("text" …) / ("kana" …)
        js.insert(
            "reading".to_owned(),
            word_info.reading_str().map_or_else(nil, Value::String),
        );
        js.insert("text".to_owned(), Value::String(word_info.text.clone()));
        js.insert(
            "kana".to_owned(),
            word_info.kana.as_ref().map_or_else(nil, kana_json),
        );
        // (when (word-info-score word-info) ("score" …)) — 0 is truthy; only nil skips.
        if let Some(score) = word_info.score {
            js.insert("score".to_owned(), Value::from(score));
        }

        if !word_info.components.is_empty() {
            // ((word-info-components word-info) …)
            js.insert(
                "compound".to_owned(),
                Value::Array(
                    word_info
                        .components
                        .iter()
                        .map(|component| Value::String(component.text.clone()))
                        .collect(),
                ),
            );
            let mut components = Vec::with_capacity(word_info.components.len());
            for component in &word_info.components {
                // (inner wi (not (word-info-primary wi)))
                components.push(inner(ctx, component, !component.primary, root_only).await?);
            }
            js.insert("components".to_owned(), Value::Array(components));
        } else if let Some((value, ordinal)) = &word_info.counter {
            // ((word-info-counter word-info) …)
            let mut counter = Map::new();
            counter.insert("value".to_owned(), Value::String(value.clone()));
            counter.insert(
                "ordinal".to_owned(),
                if *ordinal { Value::Bool(true) } else { nil() },
            );
            js.insert("counter".to_owned(), Value::Object(counter));
            // (let ((seq (word-info-seq word-info))) (when seq …))
            if let Some(seq) = single_seq(&word_info.seq) {
                js.insert("seq".to_owned(), Value::from(seq));
                // (get-senses-json seq :pos-list '("ctr") :reading-getter reading-getter)
                let pos_list = [String::from("ctr")];
                let gloss = get_senses_json(
                    ctx,
                    seq,
                    &pos_list,
                    None,
                    Some(word_info_reading(ctx, word_info)),
                )
                .await?;
                // (when gloss …)
                if !gloss.is_empty() {
                    js.insert("gloss".to_owned(), Value::Array(gloss));
                }
            }
        } else {
            // (t …)
            let seq = single_seq(&word_info.seq);
            let conjs = word_info.conjugations.as_ref();
            // (when seq ("seq" seq))
            if let Some(seq) = seq {
                js.insert("seq".to_owned(), Value::from(seq));
            }
            let mut has_gloss = false;
            if root_only {
                // (return-from inner (extend-js js ("gloss" (get-senses-json seq :reading-getter …))))
                let gloss = match seq {
                    Some(seq) => {
                        get_senses_json(ctx, seq, &[], None, Some(word_info_reading(ctx, word_info)))
                            .await?
                    }
                    None => Vec::new(),
                };
                js.insert("gloss".to_owned(), Value::Array(gloss));
                return Ok(Value::Object(js));
            }
            // ((and suffix (setf desc (get-suffix-description seq))) …)
            let mut suffix_done = false;
            if suffix {
                if let Some(seq) = seq {
                    if let Some(desc) = get_suffix_description(ctx, seq) {
                        js.insert("suffix".to_owned(), Value::String(desc.to_owned()));
                        suffix_done = true;
                    }
                }
            }
            // ((and seq (or (not conjs) (eql conjs :root))) …)
            if !suffix_done {
                if let Some(seq) = seq {
                    if conjs.is_none() || matches!(conjs, Some(WordConjugations::Root)) {
                        let gloss = get_senses_json(
                            ctx,
                            seq,
                            &[],
                            None,
                            Some(word_info_reading(ctx, word_info)),
                        )
                        .await?;
                        // (when gloss (setf has-gloss t) ("gloss" gloss))
                        if !gloss.is_empty() {
                            has_gloss = true;
                            js.insert("gloss".to_owned(), Value::Array(gloss));
                        }
                    }
                }
            }
            // (when seq ("conj" (conj-info-json seq :conjugations … :text … :has-gloss …)))
            if let Some(seq) = seq {
                let text = match &word_info.true_text {
                    Some(true_text) => FilterPropsText::One(true_text),
                    None => FilterPropsText::None,
                };
                let conj =
                    conj_info_json(ctx, seq, word_info.conjugations.as_ref(), text, has_gloss)
                        .await?;
                js.insert("conj".to_owned(), Value::Array(conj));
            }
        }
        Ok(Value::Object(js))
    })
}

pub async fn word_info_gloss_json(
    ctx: &KaniranContext,
    word_info: &WordInfo,
    root_only: bool,
) -> Result<Value, sqlx::Error> {
    if word_info.alternative {
        // (jsown:new-js ("alternative" (mapcar #'inner (word-info-components word-info))))
        let mut alternatives = Vec::with_capacity(word_info.components.len());
        for component in &word_info.components {
            alternatives.push(inner(ctx, component, false, root_only).await?);
        }
        let mut js = Map::new();
        js.insert("alternative".to_owned(), Value::Array(alternatives));
        Ok(Value::Object(js))
    } else {
        inner(ctx, word_info, false, root_only).await
    }
}

#[cfg(test)]
mod tests {
    //! Ground truth captured from `(jsown:to-json (word-info-gloss-json …))`
    //! and `find-word-info-json` on .103 (2026-05-25) after `(init-suffixes t t)`.
    //! jsown emits `\uXXXX`; the expected strings below are the
    //! raw-UTF-8 round-trip serde_json produces (identical JSON). Tests run
    //! against the local DB per project policy.
    use super::*;
    use crate::dict::find_word_info::find_word_info;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    fn json(value: &Value) -> String {
        serde_json::to_string(value).unwrap()
    }

    /// Drives word-info-gloss-json through find-word-info (non-root). One row
    /// per cond branch: simple noun (`(t)` arm, top-level gloss + has-gloss →
    /// empty conj), conjugated verb (conjs is a list → no top-level gloss; conj
    /// carries it), ordinal counter (counter object, `ordinal:true`), and
    /// compound (compound list + recursive components, the non-primary いる
    /// reaching the suffix arm).
    #[tokio::test]
    async fn word_info_gloss_json_branches() {
        let ctx = ctx_from_env().await;
        // (text, expected single-object json)
        let cases: &[(&str, &str)] = &[
            // simple noun — no conjs → top-level gloss, has-gloss → empty conj.
            ("政府", r#"{"reading":"政府 【せいふ】","text":"政府","kana":"せいふ","score":325,"seq":1376070,"gloss":[{"pos":"[n]","gloss":"government; administration; ministry"}],"conj":[]}"#),
            // conjugated verb — conjs is a list (not nil/:root) → no top-level
            // gloss; conj-info-json (has-gloss nil) carries the gloss.
            ("書いた", r#"{"reading":"書いた 【かいた】","text":"書いた","kana":"かいた","score":336,"seq":10526928,"conj":[{"prop":[{"pos":"v5k","type":"Past (~ta)"}],"reading":"書く 【かく】","gloss":[{"pos":"[v5k,vt]","gloss":"to write; to compose; to pen"},{"pos":"[vt,v5k]","gloss":"to draw; to paint"}],"readok":true}]}"#),
            // ordinal counter — counter object with ordinal:true.
            ("5番目", r#"{"reading":"5番目 【ごばんめ】","text":"5番目","kana":"ごばんめ","score":667,"counter":{"value":"Value: 5th","ordinal":true},"seq":1482410,"gloss":[{"pos":"[ctr]","gloss":"the nth ...","info":"indicates position in a sequence"}]}"#),
            // compound — compound list + components, the non-primary いる
            // component reaches the suffix arm (suffix t, has a description).
            ("食べてる", r#"{"reading":"食べてる 【たべてる】","text":"食べてる","kana":"たべてる","score":434,"compound":["食べて","いる"],"components":[{"reading":"食べて 【たべて】","text":"食べて","kana":"たべて","score":0,"seq":10092233,"conj":[{"prop":[{"pos":"v1","type":"Conjunctive (~te)"}],"reading":"食べる 【たべる】","gloss":[{"pos":"[v1,vt]","gloss":"to eat"},{"pos":"[vt,v1]","gloss":"to live on (e.g. a salary); to live off; to subsist on"}],"readok":true}]},{"reading":"いる","text":"いる","kana":"いる","score":0,"seq":1577980,"suffix":"indicates continuing action (to be ...ing)","conj":[]}]}"#),
        ];
        for (text, expected) in cases {
            let wis = find_word_info(&ctx, text, None, false).await.unwrap();
            assert!(!wis.is_empty(), "text={text}");
            let result = word_info_gloss_json(&ctx, &wis[0], false).await.unwrap();
            assert_eq!(json(&result), *expected, "text={text}");
        }
    }

    /// `root_only=true` on a non-root word-info: the `(t)` arm adds gloss
    /// unconditionally and returns before the conj key. 政府 yields a
    /// populated gloss; 書いた's conjugated seq has no direct senses, so the
    /// gloss is the empty list (still emitted, unlike the guarded non-root arm).
    #[tokio::test]
    async fn root_only_t_arm() {
        let ctx = ctx_from_env().await;
        let cases: &[(&str, &str)] = &[
            ("政府", r#"{"reading":"政府 【せいふ】","text":"政府","kana":"せいふ","score":325,"seq":1376070,"gloss":[{"pos":"[n]","gloss":"government; administration; ministry"}]}"#),
            ("書いた", r#"{"reading":"書いた 【かいた】","text":"書いた","kana":"かいた","score":336,"seq":10526928,"gloss":[]}"#),
        ];
        for (text, expected) in cases {
            let wis = find_word_info(&ctx, text, None, false).await.unwrap();
            assert!(!wis.is_empty(), "text={text}");
            let result = word_info_gloss_json(&ctx, &wis[0], true).await.unwrap();
            assert_eq!(json(&result), *expected, "text={text}");
        }
    }

    /// 5個 → two counter readings (ごこ / ごか); each carries a non-ordinal
    /// counter object (`ordinal:[]`) and a pos-list `("ctr")`-filtered gloss.
    #[tokio::test]
    async fn counter_two_readings() {
        let ctx = ctx_from_env().await;
        let expected = [
            r#"{"reading":"5個 【ごこ】","text":"5個","kana":"ごこ","score":128,"counter":{"value":"Value: 5","ordinal":[]},"seq":1264740,"gloss":[{"pos":"[ctr]","gloss":"counter for (small) things or pieces","info":"also written as ヶ"},{"pos":"[ctr]","gloss":"counter for military units"}]}"#,
            r#"{"reading":"5個 【ごか】","text":"5個","kana":"ごか","score":40,"counter":{"value":"Value: 5","ordinal":[]},"seq":2220320,"gloss":[{"pos":"[ctr]","gloss":"counter used with Sino-Japanese words","info":"precedes the thing being counted; e.g. ３か月, ５か所; also written as ヶ or ケ"}]}"#,
        ];
        let wis = find_word_info(&ctx, "5個", None, false).await.unwrap();
        assert_eq!(wis.len(), 2);
        for (wi, expected) in wis.iter().zip(expected.iter()) {
            let result = word_info_gloss_json(&ctx, wi, false).await.unwrap();
            assert_eq!(json(&result), *expected);
        }
    }

    /// Alternative branch: `{"alternative": [inner(c) …]}`. Built from a real
    /// alternative word-info (the segmenter shape: alternative=t, components =
    /// the surviving word-infos). find-word-info never produces alternatives,
    /// so the components are assembled from its 何 (なに / なん) word-infos.
    #[tokio::test]
    async fn alternative_branch() {
        let ctx = ctx_from_env().await;
        let components = find_word_info(&ctx, "何", None, false).await.unwrap();
        assert_eq!(components.len(), 2);
        let alt = WordInfo {
            kind: crate::dict::word_info_class::WordInfoType::Kanji,
            text: "何".to_owned(),
            alternative: true,
            components,
            start: Some(0),
            end: Some(1),
            ..WordInfo::default()
        };
        let result = word_info_gloss_json(&ctx, &alt, false).await.unwrap();
        let expected = r#"{"alternative":[{"reading":"何 【なに】","text":"何","kana":"なに","score":24,"seq":1577100,"gloss":[{"pos":"[pn]","gloss":"what"},{"pos":"[pn]","gloss":"you-know-what; that thing"},{"pos":"[pn]","gloss":"whatsit; whachamacallit; what's-his-name; what's-her-name"},{"pos":"[n]","gloss":"penis; (one's) thing; dick","info":"esp. ナニ"},{"pos":"[adv]","gloss":"(not) at all; (not) in the slightest","info":"with neg. sentence"},{"pos":"[int]","gloss":"what?; huh?","info":"indicates surprise"},{"pos":"[int]","gloss":"hey!; come on!","info":"indicates anger or irritability"},{"pos":"[int]","gloss":"oh, no (it's fine); why (it's nothing); oh (certainly not)","info":"used to dismiss someone's worries, concerns, etc."}],"conj":[]},{"reading":"何 【なん】","text":"何","kana":"なん","score":16,"seq":2846738,"gloss":[{"pos":"[pn]","gloss":"what"},{"pos":"[pref]","gloss":"how many","info":"followed by a counter"},{"pos":"[pref]","gloss":"many; a lot of","info":"followed by (optional number), counter and も"},{"pos":"[pref]","gloss":"several; a few; some","info":"followed by a counter and か"}],"conj":[]}]}"#;
        assert_eq!(json(&result), expected);
    }
}
