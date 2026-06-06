//! Port of `ichiran/dict:conj-info-json*` (`dict.lisp:1664`).
//!
//! Builds the per-conjugation JSON objects (prop list, reading, gloss,
//! and any `via` chain) for an entry's conjugation data, recursing into
//! [`super::conj_info_json::conj_info_json`] for `via`-linked sources.

use serde_json::{Map, Value};

use crate::conn::kani_context::KaniranContext;

use super::conj_info_json::conj_info_json;
use super::conj_prop_json::conj_prop_json;
use super::filter_props::FilterPropsText;
use super::find_words_seqs::find_words_seqs;
use super::get_conj_data::{get_conj_data, FromOrConjIds};
use super::get_original_text_once::get_original_text_once;
use super::get_senses_json::get_senses_json;
use super::kani_word::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use super::reading_str_seq::reading_str_seq;
use super::select_conjs_and_props::select_conjs_and_props;
use super::simple_text_class::WordConjugations;

pub async fn conj_info_json_star_(
    ctx: &KaniranContext,
    seq: i32,
    conjugations: Option<&WordConjugations>,
    text: FilterPropsText<'_>,
    has_gloss: bool,
) -> Result<Vec<Value>, sqlx::Error> {
    // (get-original-text-once … text) coerces a lone string to a list.
    let one_text;
    let texts: &[&str] = match text {
        FilterPropsText::None => &[],
        FilterPropsText::One(text) => {
            one_text = [text];
            &one_text
        }
        FilterPropsText::Many(text) => text,
    };

    let mut via_used: Vec<i32> = Vec::new();
    let mut result: Vec<Value> = Vec::new();

    for (conj, props, _key) in select_conjs_and_props(ctx, seq, conjugations, text).await? {
        let via = conj.seq_via;
        // unless (member via via-used) — via-used only ever holds non-null vias
        if let Some(via) = via {
            if via_used.contains(&via) {
                continue;
            }
        }

        // (get-original-text-once (get-conj-data seq (list (id conj))) text)
        let conj_datas =
            get_conj_data(ctx, seq, FromOrConjIds::ConjIds(vec![conj.id]), &[]).await?;
        let orig_text = get_original_text_once(&conj_datas, texts);
        let orig_text_refs: Vec<&str> = orig_text.iter().map(String::as_str).collect();

        // ("prop" (loop … do (push (pos conj-prop) conj-pos) collect (conj-prop-json conj-prop)))
        let mut conj_pos: Vec<String> = Vec::new();
        let mut prop_array: Vec<Value> = Vec::new();
        for conj_prop in &props {
            conj_pos.push(conj_prop.pos.clone());
            prop_array.push(conj_prop_json(conj_prop));
        }
        let mut js = Map::new();
        js.insert("prop".to_owned(), Value::Array(prop_array));

        match via {
            // dict.lisp:1676 ((eql via :null))
            None => {
                let orig_reading: Option<KaniWordDispatchEnum> = if orig_text.is_empty() {
                    None
                } else {
                    // (car (find-words-seqs orig-text (seq-from conj)))
                    find_words_seqs(ctx, &orig_text_refs, &[conj.seq_from])
                        .await?
                        .into_iter()
                        .next()
                };
                // (when (and has-gloss (not orig-reading)) (return-from outer nil))
                if has_gloss && orig_reading.is_none() {
                    continue;
                }
                let has_orig_reading = orig_reading.is_some();
                // ("reading" (reading-str (or orig-reading (seq-from conj))))
                let reading = match &orig_reading {
                    Some(KaniWordDispatchEnum::Kanji(kanji_text)) => {
                        KaniSimpleTextDispatchEnum::Kanji(kanji_text.clone())
                            .reading_str(ctx)
                            .await?
                    }
                    Some(KaniWordDispatchEnum::Kana(kana_text)) => {
                        KaniSimpleTextDispatchEnum::Kana(kana_text.clone())
                            .reading_str(ctx)
                            .await?
                    }
                    Some(_) => panic!("find-words-seqs returns only kanji-text / kana-text"),
                    None => reading_str_seq(ctx, conj.seq_from).await?,
                };
                let reading = match reading {
                    Some(reading) => Value::String(reading),
                    None => Value::Array(Vec::new()),
                };
                // ("gloss" (get-senses-json (seq-from conj) :pos-list conj-pos
                //           :reading-getter (lambda () orig-reading)))
                let gloss = get_senses_json(
                    ctx,
                    conj.seq_from,
                    &conj_pos,
                    None,
                    Some(std::future::ready(Ok(orig_reading))),
                )
                .await?;
                js.insert("reading".to_owned(), reading);
                js.insert("gloss".to_owned(), Value::Array(gloss));
                // ("readok" (when orig-reading t))
                js.insert(
                    "readok".to_owned(),
                    if has_orig_reading {
                        Value::Bool(true)
                    } else {
                        Value::Array(Vec::new())
                    },
                );
            }
            // dict.lisp:1688 (progn (let ((cij (conj-info-json via …))) …) (push via via-used))
            Some(via) => {
                let cij = Box::pin(conj_info_json(
                    ctx,
                    via,
                    None,
                    FilterPropsText::Many(&orig_text_refs),
                    has_gloss,
                ))
                .await?;
                if !cij.is_empty() {
                    // ("readok" (jsown:val (car cij) "readok")) — jsown:val errors on an
                    // absent readok (a via chain whose recursive result is empty); the
                    // Rust treats absent as the nil value, unreachable with real data.
                    let readok = cij[0]
                        .get("readok")
                        .cloned()
                        .unwrap_or(Value::Array(Vec::new()));
                    js.insert("via".to_owned(), Value::Array(cij));
                    js.insert("readok".to_owned(), readok);
                }
                via_used.push(via);
            }
        }

        // (list js)
        result.push(Value::Object(js));
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    async fn ctx_from_env() -> Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    fn json(values: &[Value]) -> String {
        serde_json::to_string(values).unwrap()
    }

    /// REPL fixtures (.103, `(jsown:to-json (conj-info-json* …))`),
    /// 2026-05-24. seq 10175587 is the past-tense (~ta) conjugation of
    /// 1370080 (尽き果てる), via-null. The kana / kanji surface both resolve
    /// the original reading (readok true); has-gloss true keeps the entry
    /// because the reading resolves; a non-matching surface and nil text
    /// leave the original reading nil (`readok` `[]`, `reading` from
    /// `reading-str-seq` of seq-from). With has-gloss true AND no resolved
    /// reading, `(return-from outer nil)` drops the entry entirely (`[]`).
    #[tokio::test]
    async fn via_null_paths() {
        let ctx = ctx_from_env().await;
        let found = r#"[{"prop":[{"pos":"v1","type":"Past (~ta)"}],"reading":"尽き果てる 【つきはてる】","gloss":[{"pos":"[vi,v1]","gloss":"to be exhausted"}],"readok":true}]"#;
        let unresolved = r#"[{"prop":[{"pos":"v1","type":"Past (~ta)"}],"reading":"尽き果てる 【つきはてる】","gloss":[{"pos":"[vi,v1]","gloss":"to be exhausted"}],"readok":[]}]"#;
        let dropped = "[]";

        struct Case {
            label: &'static str,
            text: FilterPropsText<'static>,
            has_gloss: bool,
            expected: &'static str,
        }
        let cases = [
            Case { label: "kana surface", text: FilterPropsText::One("つきはてた"), has_gloss: false, expected: found },
            Case { label: "kanji surface", text: FilterPropsText::One("尽き果てた"), has_gloss: false, expected: found },
            Case { label: "has-gloss, resolved", text: FilterPropsText::One("つきはてた"), has_gloss: true, expected: found },
            Case { label: "non-matching surface", text: FilterPropsText::One("存在しない"), has_gloss: false, expected: unresolved },
            Case { label: "nil text", text: FilterPropsText::None, has_gloss: false, expected: unresolved },
            Case { label: "has-gloss, non-matching → dropped", text: FilterPropsText::One("存在しない"), has_gloss: true, expected: dropped },
            Case { label: "has-gloss, nil text → dropped", text: FilterPropsText::None, has_gloss: true, expected: dropped },
        ];
        for case in &cases {
            let result = conj_info_json_star_(&ctx, 10175587, None, case.text, case.has_gloss)
                .await
                .unwrap();
            assert_eq!(json(&result), case.expected, "case={}", case.label);
        }
    }

    /// REPL: `(conj-info-json* 10670519 :conjugations nil :text "あくどくさせた"
    /// :has-gloss …)`. seq 10670519 is the causative-past of 1000260
    /// (悪どい) via 10155281 (causative), exercising the via-not-null
    /// recursion: the entry nests the via's own conj-info-json under
    /// `"via"` and copies its `readok`.
    #[tokio::test]
    async fn via_not_null_recursion() {
        let ctx = ctx_from_env().await;
        let expected = r#"[{"prop":[{"pos":"v1","type":"Past (~ta)"}],"via":[{"prop":[{"pos":"adj-i","type":"Causative"}],"reading":"悪どい 【あくどい】","gloss":[{"pos":"[adj-i]","gloss":"gaudy; showy; garish; loud"},{"pos":"[adj-i]","gloss":"crooked; vicious; wicked; nasty; unscrupulous; dishonest"}],"readok":true}],"readok":true}]"#;
        for has_gloss in [false, true] {
            let result =
                conj_info_json_star_(&ctx, 10670519, None, FilterPropsText::One("あくどくさせた"), has_gloss)
                    .await
                    .unwrap();
            assert_eq!(json(&result), expected, "has_gloss={has_gloss}");
        }
    }

    /// REPL: `(conj-info-json* 1156880 …)`. seq 1156880 (慰め) carries two
    /// via-null conjugations — 慰める (v1 continuative) and 慰む (v5m
    /// imperative) — so the loop emits two entries in `select-conjs-and-props`
    /// order. nil text leaves both readings unresolved (`readok` `[]`);
    /// restricting `conjugations` to one conj id (661748) emits a single
    /// entry.
    #[tokio::test]
    async fn multi_entry_and_conj_ids() {
        let ctx = ctx_from_env().await;
        let both = r#"[{"prop":[{"pos":"v1","type":"Continuative (~i)"}],"reading":"慰める 【なぐさめる】","gloss":[{"pos":"[vt,v1]","gloss":"to comfort; to console; to amuse"}],"readok":true},{"prop":[{"pos":"v5m","type":"Imperative"}],"reading":"慰む 【なぐさむ】","gloss":[{"pos":"[v5m,vi]","gloss":"to feel comforted; to be in good spirits; to feel better; to forget one's worries"},{"pos":"[vt,v5m]","gloss":"to trifle with; to fool around with"}],"readok":true}]"#;
        let both_unresolved = r#"[{"prop":[{"pos":"v1","type":"Continuative (~i)"}],"reading":"慰める 【なぐさめる】","gloss":[{"pos":"[vt,v1]","gloss":"to comfort; to console; to amuse"}],"readok":[]},{"prop":[{"pos":"v5m","type":"Imperative"}],"reading":"慰む 【なぐさむ】","gloss":[{"pos":"[v5m,vi]","gloss":"to feel comforted; to be in good spirits; to feel better; to forget one's worries"},{"pos":"[vt,v5m]","gloss":"to trifle with; to fool around with"}],"readok":[]}]"#;
        let only_one = r#"[{"prop":[{"pos":"v1","type":"Continuative (~i)"}],"reading":"慰める 【なぐさめる】","gloss":[{"pos":"[vt,v1]","gloss":"to comfort; to console; to amuse"}],"readok":true}]"#;

        let result = conj_info_json_star_(&ctx, 1156880, None, FilterPropsText::One("慰め"), false)
            .await
            .unwrap();
        assert_eq!(json(&result), both, "慰め");

        let result = conj_info_json_star_(&ctx, 1156880, None, FilterPropsText::None, false)
            .await
            .unwrap();
        assert_eq!(json(&result), both_unresolved, "nil text");

        let ids = WordConjugations::Ids(vec![661748]);
        let result = conj_info_json_star_(&ctx, 1156880, Some(&ids), FilterPropsText::One("慰め"), false)
            .await
            .unwrap();
        assert_eq!(json(&result), only_one, "conj-ids 661748");
    }
}
