//! Port of the dict.lisp conjugation-data layer — conj-data struct +
//! accessors, get-conj-data, conj-info-short/json/json* /
//! conj-prop-json, select-conjs[-and-props], filter-props,
//! make-conj-data, print-conj-info, no-conj-data, conj-type-order.

use crate::conn::kani_context::KaniranContext;
use crate::dict::best_text::get_original_text_once;
use crate::dict::dao::{ConjProp, Conjugation};
use crate::dict::find_word::find_words_seqs;
use crate::dict::kani::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use crate::dict::load::{get_conj_description, lex_compare};
use crate::dict::senses::{entry_info_short, get_senses_json, is_rareru, reading_str_seq};
use crate::dict::text_classes::WordConjugations;
use serde_json::{Map, Value};
use sqlx::{PgPool, Row};
use std::cmp::Ordering;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct ConjData {
    pub seq: Option<i32>,
    pub from: Option<i32>,
    pub via: Option<i32>,
    pub prop: Option<ConjProp>,
    pub src_map: Vec<(String, String)>,
}

pub fn conj_data_from(cd: &ConjData) -> Option<i32> {
    cd.from
}

pub fn conj_data_prop(cd: &ConjData) -> Option<ConjProp> {
    cd.prop.clone()
}

/// `_cache` suffix because the bare name `no_conj_data` is reserved
/// for the predicate function port at `dict.lisp:339`
/// ([`super::no_conj_data`]).
pub fn no_conj_data_cache(ctx: &KaniranContext) -> &HashSet<i32> {
    &ctx.no_conj_data
}

pub async fn build_no_conj_data(pool: &PgPool) -> Result<HashSet<i32>, sqlx::Error> {
    let seqs: Vec<i32> = sqlx::query_scalar(
        "SELECT entry.seq FROM entry \
         LEFT JOIN conjugation c ON entry.seq = c.seq \
         WHERE c.seq IS NULL",
    )
    .fetch_all(pool)
    .await?;
    Ok(seqs.into_iter().collect())
}

pub fn no_conj_data(ctx: &KaniranContext, seq: i32) -> bool {
    ctx.no_conj_data.contains(&seq)
}

#[derive(Debug, Clone)]
pub enum FromOrConjIds {
    /// Lisp `NIL` — fetch every conjugation row for the seq.
    All,
    /// Lisp `:root` — short-circuit to an empty result.
    Root,
    /// Lisp integer `from/conj-ids` — fetch conjugations whose
    /// `conjugation.from` column equals the integer.
    From(i32),
    /// Lisp list `from/conj-ids` — fetch conjugations whose `id` is in
    /// the list.
    ConjIds(Vec<i32>),
}

pub async fn get_conj_data(
    ctx: &KaniranContext,
    seq: i32,
    from_or_conj_ids: FromOrConjIds,
    texts: &[&str],
) -> Result<Vec<ConjData>, sqlx::Error> {
    if matches!(from_or_conj_ids, FromOrConjIds::Root) || no_conj_data(ctx, seq) {
        return Ok(Vec::new());
    }

    let filter_by_texts = !texts.is_empty();
    let texts_owned: Vec<String> = texts.iter().map(|s| (*s).to_string()).collect();

    let conjs: Vec<Conjugation> = match &from_or_conj_ids {
        FromOrConjIds::All => {
            sqlx::query_as("SELECT * FROM conjugation WHERE seq = $1")
                .bind(seq)
                .fetch_all(&ctx.pool)
                .await?
        }
        FromOrConjIds::ConjIds(ids) => {
            sqlx::query_as("SELECT * FROM conjugation WHERE seq = $1 AND id = ANY($2)")
                .bind(seq)
                .bind(ids)
                .fetch_all(&ctx.pool)
                .await?
        }
        FromOrConjIds::From(from) => {
            sqlx::query_as("SELECT * FROM conjugation WHERE seq = $1 AND \"from\" = $2")
                .bind(seq)
                .bind(*from)
                .fetch_all(&ctx.pool)
                .await?
        }
        FromOrConjIds::Root => unreachable!("filtered out at the top of the function"),
    };

    let mut out: Vec<ConjData> = Vec::new();
    for conj in conjs {
        let src_rows = if filter_by_texts {
            sqlx::query(
                "SELECT text, source_text FROM conj_source_reading \
                 WHERE conj_id = $1 AND text = ANY($2)",
            )
            .bind(conj.id)
            .bind(&texts_owned)
            .fetch_all(&ctx.pool)
            .await?
        } else {
            sqlx::query("SELECT text, source_text FROM conj_source_reading WHERE conj_id = $1")
                .bind(conj.id)
                .fetch_all(&ctx.pool)
                .await?
        };
        let src_map: Vec<(String, String)> = src_rows
            .into_iter()
            .map(|row| -> Result<(String, String), sqlx::Error> {
                Ok((row.try_get("text")?, row.try_get("source_text")?))
            })
            .collect::<Result<_, _>>()?;
        if filter_by_texts && src_map.is_empty() {
            continue;
        }

        let props: Vec<ConjProp> = sqlx::query_as("SELECT * FROM conj_prop WHERE conj_id = $1")
            .bind(conj.id)
            .fetch_all(&ctx.pool)
            .await?;
        for prop in props {
            out.push(make_conj_data(
                Some(conj.seq),
                Some(conj.seq_from),
                conj.seq_via,
                Some(prop),
                src_map.clone(),
            ));
        }
    }
    Ok(out)
}

pub fn conj_info_short(obj: &ConjProp) -> String {
    // dict.lisp:277 — "[~a] ~a~@[~[ Affirmative~; Negative~]~]~@[~[ Plain~; Formal~]~]"
    let neg = match obj.neg {
        Some(false) => " Affirmative",
        Some(true) => " Negative",
        None => "",
    };
    let fml = match obj.fml {
        Some(false) => " Plain",
        Some(true) => " Formal",
        None => "",
    };
    format!(
        "[{}] {}{}{}",
        obj.pos,
        // ~a of nil prints "NIL"
        get_conj_description(obj.conj_type).unwrap_or("NIL"),
        neg,
        fml,
    )
}

// (jsown:val c "readok") as a generalized boolean — t is truthy, jsown's
// nil-rendering [] is falsy.
fn readok_truthy(entry: &Value) -> bool {
    match entry.get("readok") {
        Some(Value::Bool(readok)) => *readok,
        Some(Value::Array(readok)) => !readok.is_empty(),
        Some(Value::Null) | None => false,
        Some(_) => true,
    }
}

pub async fn conj_info_json(
    ctx: &KaniranContext,
    seq: i32,
    conjugations: Option<&WordConjugations>,
    text: FilterPropsText<'_>,
    has_gloss: bool,
) -> Result<Vec<Value>, sqlx::Error> {
    // (apply 'conj-info-json* seq rest)
    let cij = conj_info_json_star_(ctx, seq, conjugations, text, has_gloss).await?;
    // (remove-if-not (lambda (c) (jsown:val c "readok")) cij)
    let fcij: Vec<Value> = cij
        .iter()
        .filter(|entry| readok_truthy(entry))
        .cloned()
        .collect();
    // (or fcij cij)
    Ok(if fcij.is_empty() { cij } else { fcij })
}

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

pub fn conj_prop_json(obj: &ConjProp) -> Value {
    let mut js = Map::new();
    js.insert("pos".to_owned(), Value::String(obj.pos.clone()));
    // dict.lisp:288 — jsown serializes a nil description as []
    js.insert(
        "type".to_owned(),
        match get_conj_description(obj.conj_type) {
            Some(desc) => Value::String(desc.to_owned()),
            None => Value::Array(Vec::new()),
        },
    );
    let neg = obj.neg;
    let fml = obj.fml;
    // dict.lisp:291,293 — (unless (or (not x) (eql x :null)) ...): only the t state extends
    if neg == Some(true) {
        js.insert("neg".to_owned(), Value::Bool(true));
    }
    if fml == Some(true) {
        js.insert("fml".to_owned(), Value::Bool(true));
    }
    Value::Object(js)
}

pub fn conj_type_order(conj_type: i32) -> i32 {
    match conj_type {
        10 => 13,
        13 => 10,
        _ => conj_type,
    }
}

pub async fn select_conjs(
    ctx: &KaniranContext,
    seq: i32,
    conj_ids: Option<&WordConjugations>,
) -> Result<Vec<Conjugation>, sqlx::Error> {
    match conj_ids {
        // (if conj-ids …) truthy branch
        Some(WordConjugations::Root) => Ok(Vec::new()),
        // (select-dao 'conjugation (:and (:= 'seq seq) (:in 'id (:set conj-ids))))
        Some(WordConjugations::Ids(ids)) => {
            sqlx::query_as("SELECT * FROM conjugation WHERE seq = $1 AND id = ANY($2)")
                .bind(seq)
                .bind(ids)
                .fetch_all(&ctx.pool)
                .await
        }
        // (or (select-dao … (:is-null 'via)) (select-dao … seq))
        None => {
            let with_null_via: Vec<Conjugation> =
                sqlx::query_as("SELECT * FROM conjugation WHERE seq = $1 AND via IS NULL")
                    .bind(seq)
                    .fetch_all(&ctx.pool)
                    .await?;
            if with_null_via.is_empty() {
                sqlx::query_as("SELECT * FROM conjugation WHERE seq = $1")
                    .bind(seq)
                    .fetch_all(&ctx.pool)
                    .await
            } else {
                Ok(with_null_via)
            }
        }
    }
}

pub async fn select_conjs_and_props(
    ctx: &KaniranContext,
    seq: i32,
    conj_ids: Option<&WordConjugations>,
    text: FilterPropsText<'_>,
) -> Result<Vec<(Conjugation, Vec<ConjProp>, [i32; 2])>, sqlx::Error> {
    let mut result: Vec<(Conjugation, Vec<ConjProp>, [i32; 2])> = Vec::new();
    // (loop for conj in (select-conjs seq conj-ids) …)
    for conj in select_conjs(ctx, seq, conj_ids).await? {
        // (select-dao 'conj-prop (:= 'conj-id (id conj)))
        let props: Vec<ConjProp> = sqlx::query_as("SELECT * FROM conj_prop WHERE conj_id = $1")
            .bind(conj.id)
            .fetch_all(&ctx.pool)
            .await?;
        // (loop for prop in props minimizing (conj-type-order (conj-type prop)))
        let val = props
            .iter()
            .map(|prop| conj_type_order(prop.conj_type))
            .min()
            .unwrap_or(0);
        // (filter-props props text)
        let fprops: Vec<ConjProp> = filter_props(&props, text).into_iter().cloned().collect();
        // (list (if (eql (seq-via conj) :null) 0 1) val)
        let key = [if conj.seq_via.is_none() { 0 } else { 1 }, val];
        result.push((conj, fprops, key));
    }
    // (sort … (lex-compare '<) :key 'third)
    let key_cmp = lex_compare(|left: &i32, right: &i32| left < right);
    result.sort_by(|left, right| {
        if key_cmp(&left.2, &right.2) {
            Ordering::Less
        } else if key_cmp(&right.2, &left.2) {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    });
    Ok(result)
}

#[derive(Clone, Copy)]
pub enum FilterPropsText<'a> {
    None,
    One(&'a str),
    Many(&'a [&'a str]),
}

pub fn filter_props<'a>(props: &'a [ConjProp], text: FilterPropsText<'_>) -> Vec<&'a ConjProp> {
    let mut result = Vec::new();
    for prop in props {
        // (and text (= (conj-type prop) 6) (find (pos prop) '("v1" "v1s" "vk")) (not …))
        let drop_passive = match text {
            // text nil → (and text …) short-circuits → keep prop
            FilterPropsText::None => false,
            FilterPropsText::One(text) => {
                prop.conj_type == 6
                    && ["v1", "v1s", "vk"].contains(&prop.pos.as_str())
                    && !is_rareru(text)
            }
            // empty list is nil → (and text …) short-circuits → keep prop
            FilterPropsText::Many(text) => {
                !text.is_empty()
                    && prop.conj_type == 6
                    && ["v1", "v1s", "vk"].contains(&prop.pos.as_str())
                    && !text.iter().any(|text| is_rareru(text))
            }
        };
        if !drop_passive {
            result.push(prop);
        }
    }
    result
}

pub fn make_conj_data(
    seq: Option<i32>,
    from: Option<i32>,
    via: Option<i32>,
    prop: Option<ConjProp>,
    src_map: Vec<(String, String)>,
) -> ConjData {
    ConjData {
        seq,
        from,
        via,
        prop,
        src_map,
    }
}

pub async fn print_conj_info(
    ctx: &KaniranContext,
    seq: i32,
    conjugations: Option<&WordConjugations>,
    out: &mut String,
) -> Result<(), sqlx::Error> {
    let mut via_used: Vec<Option<i32>> = Vec::new();
    // (select-conjs-and-props seq conjugations) — print-conj-info passes no text
    for (conj, props, _) in
        select_conjs_and_props(ctx, seq, conjugations, FilterPropsText::None).await?
    {
        let via = conj.seq_via;
        // unless (member via via-used)
        if via_used.contains(&via) {
            continue;
        }
        // dict.lisp:1655 — "~%~:[ ~;[~] Conjugation: ~a" (first → "[", else " ")
        let mut first = true;
        for conj_prop in &props {
            out.push_str(&format!(
                "\n{} Conjugation: {}",
                if first { "[" } else { " " },
                conj_info_short(conj_prop)
            ));
            first = false;
        }
        // (if (eql via :null) ...)
        match via {
            None => {
                // (format out "~%  ~a" (entry-info-short (seq-from conj)))
                out.push_str(&format!(
                    "\n  {}",
                    entry_info_short(ctx, conj.seq_from, None).await?
                ));
            }
            Some(via_seq) => {
                // (format out "~% --(via)--")
                out.push_str("\n --(via)--");
                // (print-conj-info via :out out)
                Box::pin(print_conj_info(ctx, via_seq, None, out)).await?;
                // (push via via-used)
                via_used.push(via);
            }
        }
        // (princ " ]" out)
        out.push_str(" ]");
    }
    Ok(())
}

#[cfg(test)]
mod test_conj_info_short {
    use super::*;

    fn prop(pos: &str, conj_type: i32, neg: Option<bool>, fml: Option<bool>) -> ConjProp {
        ConjProp {
            id: 0,
            conj_id: 0,
            conj_type,
            pos: pos.to_string(),
            neg,
            fml,
        }
    }

    /// REPL fixtures (.103, ichiran/dict::conj-info-short on conj_prop
    /// rows), 2026-05-24. Covers every neg/fml state — Some(false)
    /// (Lisp nil), Some(true) (Lisp t), None (db-null) — plus the
    /// missing-description path (`~a` of nil → "NIL").
    #[test]
    fn conj_info_short_fixtures() {
        let cases: &[(ConjProp, &str)] = &[
            (
                prop("v5k", 2, Some(false), Some(false)),
                "[v5k] Past (~ta) Affirmative Plain",
            ),
            (
                prop("v5k", 1, Some(false), Some(true)),
                "[v5k] Non-past Affirmative Formal",
            ),
            (
                prop("v5k", 1, Some(true), Some(false)),
                "[v5k] Non-past Negative Plain",
            ),
            (
                prop("v5k", 1, Some(true), Some(true)),
                "[v5k] Non-past Negative Formal",
            ),
            (
                prop("adj-i", 3, Some(false), None),
                "[adj-i] Conjunctive (~te) Affirmative",
            ),
            (
                prop("v5k", 52, Some(true), None),
                "[v5k] Negative Stem Negative",
            ),
            (
                prop("adj-i", 9, None, Some(false)),
                "[adj-i] Volitional Plain",
            ),
            (
                prop("adj-i", 9, None, Some(true)),
                "[adj-i] Volitional Formal",
            ),
            (prop("v5k", 13, None, None), "[v5k] Continuative (~i)"),
            (
                prop("v5k", 999, Some(false), Some(false)),
                "[v5k] NIL Affirmative Plain",
            ),
        ];
        for (obj, expected) in cases {
            assert_eq!(&conj_info_short(obj), expected, "obj={obj:?}");
        }
    }
}

#[cfg(test)]
mod test_conj_info_json {
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

    /// REPL fixtures (.103, `(jsown:to-json (conj-info-json …))`), 2026-05-24.
    /// seq 10175587 (尽き果てる, ~ta) resolves the original reading with a
    /// matching surface (readok true → kept). With nil text every entry's
    /// readok is `[]`, so `remove-if-not` empties the filtered list and the
    /// `(or fcij cij)` fallback returns the unfiltered list unchanged.
    #[tokio::test]
    async fn readok_filter_and_fallback() {
        let ctx = ctx_from_env().await;
        let found = r#"[{"prop":[{"pos":"v1","type":"Past (~ta)"}],"reading":"尽き果てる 【つきはてる】","gloss":[{"pos":"[vi,v1]","gloss":"to be exhausted"}],"readok":true}]"#;
        let unresolved = r#"[{"prop":[{"pos":"v1","type":"Past (~ta)"}],"reading":"尽き果てる 【つきはてる】","gloss":[{"pos":"[vi,v1]","gloss":"to be exhausted"}],"readok":[]}]"#;

        let result = conj_info_json(
            &ctx,
            10175587,
            None,
            FilterPropsText::One("つきはてた"),
            false,
        )
        .await
        .unwrap();
        assert_eq!(json(&result), found, "resolved surface");

        let result = conj_info_json(&ctx, 10175587, None, FilterPropsText::None, false)
            .await
            .unwrap();
        assert_eq!(json(&result), unresolved, "nil text → fallback to cij");

        // has-gloss + unresolved reading drops the only entry in conj-info-json*,
        // so cij is empty and (or fcij cij) is the empty list.
        let result = conj_info_json(
            &ctx,
            10175587,
            None,
            FilterPropsText::One("存在しない"),
            true,
        )
        .await
        .unwrap();
        assert_eq!(json(&result), "[]", "has-gloss drop → empty");
    }

    /// REPL: `(conj-info-json 10670519 :conjugations nil :text "あくどくさせた"
    /// :has-gloss nil)`. The via-not-null entry keeps its recursive `via`
    /// payload (readok copied from the via's first element → true).
    #[tokio::test]
    async fn via_recursion_kept() {
        let ctx = ctx_from_env().await;
        let expected = r#"[{"prop":[{"pos":"v1","type":"Past (~ta)"}],"via":[{"prop":[{"pos":"adj-i","type":"Causative"}],"reading":"悪どい 【あくどい】","gloss":[{"pos":"[adj-i]","gloss":"gaudy; showy; garish; loud"},{"pos":"[adj-i]","gloss":"crooked; vicious; wicked; nasty; unscrupulous; dishonest"}],"readok":true}],"readok":true}]"#;
        let result = conj_info_json(
            &ctx,
            10670519,
            None,
            FilterPropsText::One("あくどくさせた"),
            false,
        )
        .await
        .unwrap();
        assert_eq!(json(&result), expected);
    }

    /// REPL: `(conj-info-json 1156880 :conjugations nil :text "慰め")`. Both
    /// via-null entries resolve (readok true), so the filtered list equals
    /// the full two-entry list.
    #[tokio::test]
    async fn multi_entry_all_kept() {
        let ctx = ctx_from_env().await;
        let both = r#"[{"prop":[{"pos":"v1","type":"Continuative (~i)"}],"reading":"慰める 【なぐさめる】","gloss":[{"pos":"[vt,v1]","gloss":"to comfort; to console; to amuse"}],"readok":true},{"prop":[{"pos":"v5m","type":"Imperative"}],"reading":"慰む 【なぐさむ】","gloss":[{"pos":"[v5m,vi]","gloss":"to feel comforted; to be in good spirits; to feel better; to forget one's worries"},{"pos":"[vt,v5m]","gloss":"to trifle with; to fool around with"}],"readok":true}]"#;
        let result = conj_info_json(&ctx, 1156880, None, FilterPropsText::One("慰め"), false)
            .await
            .unwrap();
        assert_eq!(json(&result), both);
    }
}

#[cfg(test)]
mod test_conj_info_json_star {
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
            Case {
                label: "kana surface",
                text: FilterPropsText::One("つきはてた"),
                has_gloss: false,
                expected: found,
            },
            Case {
                label: "kanji surface",
                text: FilterPropsText::One("尽き果てた"),
                has_gloss: false,
                expected: found,
            },
            Case {
                label: "has-gloss, resolved",
                text: FilterPropsText::One("つきはてた"),
                has_gloss: true,
                expected: found,
            },
            Case {
                label: "non-matching surface",
                text: FilterPropsText::One("存在しない"),
                has_gloss: false,
                expected: unresolved,
            },
            Case {
                label: "nil text",
                text: FilterPropsText::None,
                has_gloss: false,
                expected: unresolved,
            },
            Case {
                label: "has-gloss, non-matching → dropped",
                text: FilterPropsText::One("存在しない"),
                has_gloss: true,
                expected: dropped,
            },
            Case {
                label: "has-gloss, nil text → dropped",
                text: FilterPropsText::None,
                has_gloss: true,
                expected: dropped,
            },
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
            let result = conj_info_json_star_(
                &ctx,
                10670519,
                None,
                FilterPropsText::One("あくどくさせた"),
                has_gloss,
            )
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
        let result = conj_info_json_star_(
            &ctx,
            1156880,
            Some(&ids),
            FilterPropsText::One("慰め"),
            false,
        )
        .await
        .unwrap();
        assert_eq!(json(&result), only_one, "conj-ids 661748");
    }
}

#[cfg(test)]
mod test_conj_prop_json {
    use super::*;

    fn prop(conj_type: i32, pos: &str, neg: Option<bool>, fml: Option<bool>) -> ConjProp {
        ConjProp {
            id: 0,
            conj_id: 0,
            conj_type,
            pos: pos.to_owned(),
            neg,
            fml,
        }
    }

    /// REPL fixtures (.103, `jsown:to-json` of `conj-prop-json` on real
    /// conj_prop rows), 2026-05-24. Rows cover every neg/fml state
    /// (`:null`→None, `t`→Some(true), `nil`→Some(false)); the no-description
    /// row pins jsown's nil→[] rendering of "type".
    #[test]
    fn conj_prop_json_fixtures() {
        let cases = [
            // id=52: neg :null, fml :null
            (
                prop(13, "v5k", None, None),
                r#"{"pos":"v5k","type":"Continuative (~i)"}"#,
            ),
            // id=3: neg t, fml t
            (
                prop(1, "v5k", Some(true), Some(true)),
                r#"{"pos":"v5k","type":"Non-past","neg":true,"fml":true}"#,
            ),
            // id=2: neg t, fml nil
            (
                prop(1, "v5k", Some(true), Some(false)),
                r#"{"pos":"v5k","type":"Non-past","neg":true}"#,
            ),
            // id=1: neg nil, fml t
            (
                prop(1, "v5k", Some(false), Some(true)),
                r#"{"pos":"v5k","type":"Non-past","fml":true}"#,
            ),
            // id=4: neg nil, fml nil
            (
                prop(2, "v5k", Some(false), Some(false)),
                r#"{"pos":"v5k","type":"Past (~ta)"}"#,
            ),
            // id=226: neg :null, fml t
            (
                prop(9, "adj-i", None, Some(true)),
                r#"{"pos":"adj-i","type":"Volitional","fml":true}"#,
            ),
            // id=53: neg t, fml :null
            (
                prop(52, "v5k", Some(true), None),
                r#"{"pos":"v5k","type":"Negative Stem","neg":true}"#,
            ),
            // no description for conj_type → jsown nil renders as []
            (prop(999, "v5k", None, None), r#"{"pos":"v5k","type":[]}"#),
        ];
        for (obj, expected) in &cases {
            let actual = serde_json::to_string(&conj_prop_json(obj)).unwrap();
            assert_eq!(
                actual.as_str(),
                *expected,
                "conj_type={} pos={}",
                obj.conj_type,
                obj.pos
            );
        }
    }
}

#[cfg(test)]
mod test_conj_type_order {
    use super::*;

    /// REPL fixtures (.103, `ichiran/dict::conj-type-order`), 2026-05-24.
    /// Covers the 10↔13 swap and the identity fall-through.
    #[test]
    fn conj_type_order_fixtures() {
        let cases: &[(i32, i32)] = &[(10, 13), (13, 10), (1, 1), (0, 0), (99, 99)];
        for (conj_type, expected) in cases {
            assert_eq!(
                conj_type_order(*conj_type),
                *expected,
                "conj_type={conj_type}"
            );
        }
    }
}

#[cfg(test)]
mod test_select_conjs {
    use super::*;
    use std::sync::Arc;

    async fn ctx_from_env() -> Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    /// REPL fixtures (.103, `ichiran/dict::select-conjs`), 2026-05-24.
    /// - 2028980: single via-null conjugation (id 2343254, from 2089020) —
    ///   mirrors `tests.lisp:651`.
    /// - 1156880: via-null branch returns two rows (366552, 661748); the
    ///   seq's via-not-null row (705712) is excluded.
    /// - 1257260: no via-null rows, so the `or` falls back to all rows
    ///   (1239109, 1239126), both via-not-null.
    #[tokio::test]
    async fn select_conjs_nil_conj_ids() {
        let ctx = ctx_from_env().await;

        let r2028980 = select_conjs(&ctx, 2028980, None).await.unwrap();
        let mut ids: Vec<i32> = r2028980.iter().map(|c| c.id).collect();
        ids.sort_unstable();
        assert_eq!(ids, vec![2343254]);
        assert_eq!(r2028980[0].seq_from, 2089020);
        assert_eq!(r2028980[0].seq_via, None);

        let r1156880 = select_conjs(&ctx, 1156880, None).await.unwrap();
        let mut ids: Vec<i32> = r1156880.iter().map(|c| c.id).collect();
        ids.sort_unstable();
        assert_eq!(ids, vec![366552, 661748]);
        assert!(r1156880.iter().all(|c| c.seq_via.is_none()));

        // or-fallback: no via-null rows → all rows (both via-not-null).
        let r1257260 = select_conjs(&ctx, 1257260, None).await.unwrap();
        let mut ids: Vec<i32> = r1257260.iter().map(|c| c.id).collect();
        ids.sort_unstable();
        assert_eq!(ids, vec![1239109, 1239126]);
        assert!(r1257260.iter().all(|c| c.seq_via.is_some()));
    }

    /// REPL: `(select-conjs 2028980 :root)` → `NIL`.
    #[tokio::test]
    async fn select_conjs_root_is_empty() {
        let ctx = ctx_from_env().await;
        let result = select_conjs(&ctx, 2028980, Some(&WordConjugations::Root))
            .await
            .unwrap();
        assert!(result.is_empty());
    }

    /// REPL: `(select-conjs 1156880 (list 366552))` → only the requested
    /// id, regardless of the via-null preference (no `or` fallback). The
    /// via-not-null row (705712) is reachable through an explicit id list.
    #[tokio::test]
    async fn select_conjs_explicit_ids() {
        let ctx = ctx_from_env().await;

        let one = select_conjs(&ctx, 1156880, Some(&WordConjugations::Ids(vec![366552])))
            .await
            .unwrap();
        let ids: Vec<i32> = one.iter().map(|c| c.id).collect();
        assert_eq!(ids, vec![366552]);

        // The via-not-null row is selectable by id even though the
        // nil-conj-ids path filters it out.
        let via_row = select_conjs(&ctx, 1156880, Some(&WordConjugations::Ids(vec![705712])))
            .await
            .unwrap();
        assert_eq!(via_row.len(), 1);
        assert_eq!(via_row[0].seq_via, Some(1156890));

        // ids that don't belong to the seq are filtered by the `seq =` clause.
        let none = select_conjs(&ctx, 1156880, Some(&WordConjugations::Ids(vec![1])))
            .await
            .unwrap();
        assert!(none.is_empty());
    }
}

#[cfg(test)]
mod test_select_conjs_and_props {
    use super::*;
    use std::sync::Arc;

    async fn ctx_from_env() -> Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    type FpropRow = (i32, i32, i32, String, Option<bool>, Option<bool>);
    type ConjRow = (i32, i32, i32, Option<i32>, [i32; 2], Vec<FpropRow>);

    fn project(rows: &[(Conjugation, Vec<ConjProp>, [i32; 2])]) -> Vec<ConjRow> {
        rows.iter()
            .map(|(conj, fprops, key)| {
                (
                    conj.id,
                    conj.seq,
                    conj.seq_from,
                    conj.seq_via,
                    *key,
                    fprops
                        .iter()
                        .map(|p| (p.id, p.conj_id, p.conj_type, p.pos.clone(), p.neg, p.fml))
                        .collect(),
                )
            })
            .collect()
    }

    /// REPL: `(select-conjs-and-props 1156880)` → two via-null
    /// conjugations sorted by `(0 val)`. conj 661748 has prop type 13
    /// → `conj-type-order` 10 → key `(0 10)`, sorts ahead of conj
    /// 366552 (prop type 10 → `conj-type-order` 13 → key `(0 13)`),
    /// reordering the `select-conjs` input. Exercises the val swap and
    /// the sort, with nil text (all props kept).
    #[tokio::test]
    async fn via_null_sorted_by_val() {
        let ctx = ctx_from_env().await;
        let rows = select_conjs_and_props(&ctx, 1156880, None, FilterPropsText::None)
            .await
            .unwrap();
        let expected: Vec<ConjRow> = vec![
            (
                661748,
                1156880,
                1156890,
                None,
                [0, 10],
                vec![(676835, 661748, 13, "v1".to_string(), None, None)],
            ),
            (
                366552,
                1156880,
                1156870,
                None,
                [0, 13],
                vec![(
                    374822,
                    366552,
                    10,
                    "v5m".to_string(),
                    Some(false),
                    Some(false),
                )],
            ),
        ];
        assert_eq!(project(&rows), expected);
    }

    /// REPL: `(select-conjs-and-props 1257260)` → no via-null rows, so
    /// `select-conjs` falls back to all rows; both have non-null via →
    /// key first element 1. Sorted `(1 10)` before `(1 13)`. Exercises
    /// the via-flag=1 branch and the or-fallback path.
    #[tokio::test]
    async fn via_not_null_flag_one() {
        let ctx = ctx_from_env().await;
        let rows = select_conjs_and_props(&ctx, 1257260, None, FilterPropsText::None)
            .await
            .unwrap();
        let expected: Vec<ConjRow> = vec![
            (
                1239109,
                1257260,
                1609260,
                Some(10036077),
                [1, 10],
                vec![(1254564, 1239109, 13, "v1".to_string(), None, None)],
            ),
            (
                1239126,
                1257260,
                1609260,
                Some(10036081),
                [1, 13],
                vec![(
                    1254581,
                    1239126,
                    10,
                    "v5s".to_string(),
                    Some(false),
                    Some(false),
                )],
            ),
        ];
        assert_eq!(project(&rows), expected);
    }

    /// REPL fixtures (.103, `ichiran/dict::select-conjs-and-props`),
    /// 2026-05-24. seq 1232500 has one via-null conjugation (159588)
    /// with a passive prop (type 6, pos v1). The key stays `(0 6)` and
    /// `val` stays 6 across every text — `val` reads the *unfiltered*
    /// props — while `fprops` drops the passive prop exactly when
    /// `filter-props` would: text non-nil, not a rareru form. Covers
    /// nil / single rareru / single non-rareru / list with a rareru /
    /// list without a rareru.
    #[tokio::test]
    async fn text_threads_to_filter_props() {
        let ctx = ctx_from_env().await;
        let prop = (
            163127,
            159588,
            6,
            "v1".to_string(),
            Some(false),
            Some(false),
        );
        let kept: Vec<ConjRow> = vec![(159588, 1232500, 2864818, None, [0, 6], vec![prop.clone()])];
        let dropped: Vec<ConjRow> = vec![(159588, 1232500, 2864818, None, [0, 6], vec![])];

        let rareru = ["食べる", "見られる"];
        let no_rareru = ["食べる", "飲む"];
        let cases: &[(FilterPropsText, &Vec<ConjRow>)] = &[
            (FilterPropsText::None, &kept),
            (FilterPropsText::One("見られる"), &kept),
            (FilterPropsText::One("食べる"), &dropped),
            (FilterPropsText::Many(&rareru), &kept),
            (FilterPropsText::Many(&no_rareru), &dropped),
        ];
        for (text, expected) in cases {
            let rows = select_conjs_and_props(&ctx, 1232500, None, *text)
                .await
                .unwrap();
            assert_eq!(&project(&rows), *expected, "text variant mismatch");
        }
    }

    /// REPL: `(select-conjs-and-props 2028980 :root)` → `NIL`
    /// (`select-conjs … :root` returns no conjugations).
    #[tokio::test]
    async fn root_conj_ids_empty() {
        let ctx = ctx_from_env().await;
        let rows = select_conjs_and_props(
            &ctx,
            2028980,
            Some(&WordConjugations::Root),
            FilterPropsText::None,
        )
        .await
        .unwrap();
        assert!(rows.is_empty());
    }
}

#[cfg(test)]
mod test_filter_props {
    use super::*;

    fn prop(conj_id: i32, conj_type: i32, pos: &str) -> ConjProp {
        ConjProp {
            id: conj_id,
            conj_id,
            conj_type,
            pos: pos.to_string(),
            neg: None,
            fml: None,
        }
    }

    fn ids(props: &[&ConjProp]) -> Vec<i32> {
        props.iter().map(|prop| prop.conj_id).collect()
    }

    /// REPL fixtures (.103, `ichiran/dict::filter-props`), 2026-05-24.
    /// `props` = passive v1 (1), passive v5r (2, pos out of set), plain v1
    /// (3, conj-type ≠ 6), passive v1s (4), passive vk (5). Each row drops
    /// the passive v1/v1s/vk props (1,4,5) only when text is non-nil and
    /// not a rareru form. Covers nil, single string (rareru / non-rareru /
    /// empty-but-truthy), and list (with / without a rareru member /
    /// empty-list-is-nil).
    #[test]
    fn filter_props_fixtures() {
        let props = vec![
            prop(1, 6, "v1"),
            prop(2, 6, "v5r"),
            prop(3, 1, "v1"),
            prop(4, 6, "v1s"),
            prop(5, 6, "vk"),
        ];
        let some = ["食べる", "見られる"];
        let none = ["食べる", "飲む"];
        let cases: &[(FilterPropsText, Vec<i32>)] = &[
            (FilterPropsText::None, vec![1, 2, 3, 4, 5]),
            (FilterPropsText::One("食べる"), vec![2, 3]),
            (FilterPropsText::One("食べられる"), vec![1, 2, 3, 4, 5]),
            (FilterPropsText::One(""), vec![2, 3]),
            (FilterPropsText::Many(&none), vec![2, 3]),
            (FilterPropsText::Many(&some), vec![1, 2, 3, 4, 5]),
            (FilterPropsText::Many(&[]), vec![1, 2, 3, 4, 5]),
        ];
        for (text, expected) in cases {
            assert_eq!(ids(&filter_props(&props, *text)), *expected);
        }
        // empty props → empty result
        assert!(filter_props(&[], FilterPropsText::One("食べる")).is_empty());
    }
}

#[cfg(test)]
mod test_print_conj_info {
    use super::*;
    use std::sync::Arc;

    async fn ctx_from_env() -> Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    async fn render(
        ctx: &KaniranContext,
        seq: i32,
        conjugations: Option<&WordConjugations>,
    ) -> String {
        let mut out = String::new();
        print_conj_info(ctx, seq, conjugations, &mut out)
            .await
            .unwrap();
        out
    }

    /// REPL fixtures (.103, `ichiran/dict::print-conj-info` via
    /// `(with-output-to-string (s) (print-conj-info seq :out s))`),
    /// 2026-05-24, `conjugations` = nil. Covers:
    /// - 1156880: two via-null conjugations, one prop each — the
    ///   entry-info-short branch repeated, each prop opens with "[".
    /// - 1184270: one via-null conjugation with two props — the
    ///   `first` toggle ("[" then " ") inside one " ]".
    /// - 1257260: two non-null via conjugations — the " --(via)--"
    ///   branch with a recursive call producing the via entry.
    /// - 10674648: two conjugations sharing via 10327845 — the second
    ///   is dropped by `(member via via-used)`, so only one block prints.
    /// - 1358280: no conjugations (root) → empty output.
    #[tokio::test]
    async fn print_conj_info_fixtures() {
        let ctx = ctx_from_env().await;
        let cases: &[(i32, &str)] = &[
            (
                1156880,
                "\n[ Conjugation: [v1] Continuative (~i)\n  慰める 【なぐさめる】 : to comfort; to console; to amuse ]\n[ Conjugation: [v5m] Imperative Affirmative Plain\n  慰む 【なぐさむ】 : to feel comforted; to be in good spirits; to feel better; to forget one's worries ]",
            ),
            (
                1184270,
                "\n[ Conjugation: [v5aru] Imperative Affirmative Plain\n  Conjugation: [v5aru] Continuative (~i)\n  下さる 【くださる】 : to give; to confer; to bestow ]",
            ),
            (
                1257260,
                "\n[ Conjugation: [v1] Continuative (~i)\n --(via)--\n[ Conjugation: [v5r] Causative Affirmative Plain\n  嫌がる 【いやがる】 : to appear uncomfortable (with); to seem to hate; to express dislike ] ]\n[ Conjugation: [v5s] Imperative Affirmative Plain\n --(via)--\n[ Conjugation: [v5r] Causative (~su) Affirmative Plain\n  嫌がる 【いやがる】 : to appear uncomfortable (with); to seem to hate; to express dislike ] ]",
            ),
            (
                10674648,
                "\n[ Conjugation: [v1] Past (~ta) Affirmative Plain\n --(via)--\n[ Conjugation: [v5s] Potential Affirmative Plain\n  くねらす : to wriggle; to twist (one's body); to writhe ]\n[ Conjugation: [v5r] Causative Affirmative Plain\n  くねる : to bend loosely back and forth; to wriggle; to be crooked ] ]",
            ),
            (1358280, ""),
        ];
        for (seq, expected) in cases {
            assert_eq!(&render(&ctx, *seq, None).await, expected, "seq={seq}");
        }
    }

    /// REPL fixtures (.103, `print-conj-info 1156880` with
    /// `:conjugations`), 2026-05-24. `:root` selects no conjugations
    /// (empty output); an explicit id list narrows to that single
    /// conjugation.
    #[tokio::test]
    async fn print_conj_info_conjugations_arg() {
        let ctx = ctx_from_env().await;
        assert_eq!(
            render(&ctx, 1156880, Some(&WordConjugations::Root)).await,
            "",
            "conjugations=:root"
        );
        assert_eq!(
            render(&ctx, 1156880, Some(&WordConjugations::Ids(vec![366552]))).await,
            "\n[ Conjugation: [v5m] Imperative Affirmative Plain\n  慰む 【なぐさむ】 : to feel comforted; to be in good spirits; to feel better; to forget one's worries ]",
            "conjugations=(366552)"
        );
    }
}
