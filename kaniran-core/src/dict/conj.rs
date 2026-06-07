use crate::conn::kani_context::KaniranContext;
use crate::dict::dao::{conj_info_short, conj_prop_json, ConjProp, Conjugation, WordConjugations};
use crate::dict::kani_word::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use crate::dict::load::conjugate::lex_compare;
use crate::dict::readings::{find_words_seqs, get_original_text_once};
use crate::dict::senses::get_senses_json;
use crate::dict::word_info_str::{entry_info_short, reading_str_seq};
use serde_json::{Map, Value};
use sqlx::{PgPool, Row};
use std::cmp::Ordering;
use std::collections::HashSet;

/// Port of `ichiran/dict:*no-conj-data*` (`dict.lisp:329`).
///
/// Set of seqs for JMdict entries that have no rows in the
/// `conjugation` table.
/// `_cache` suffix because the bare name `no_conj_data` is reserved
/// for the predicate function port at `dict.lisp:339`
/// ([`crate::dict::conj::no_conj_data`]).
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

/// Port of `ichiran/dict:conj-data` (`dict.lisp:327`).
///
/// In-memory record carrying one (entry, conjugation-from, optional
/// via-entry, conj-prop, source-text-pairs) tuple describing a single
/// conjugation candidate.
#[derive(Debug, Clone)]
pub struct ConjData {
    pub seq: Option<i32>,
    pub from: Option<i32>,
    pub via: Option<i32>,
    pub prop: Option<ConjProp>,
    pub src_map: Vec<(String, String)>,
}

/// Port of `ichiran/dict:no-conj-data` (`dict.lisp:339`).
///
/// True when `seq` has no rows in the `conjugation` table.
pub fn no_conj_data(ctx: &KaniranContext, seq: i32) -> bool {
    ctx.no_conj_data.contains(&seq)
}

/// Transliteration of `ichiran/dict:get-conj-data` (`dict.lisp:340`).
///
/// Walks the `conjugation` table for one `seq`, joins each row to its
/// `conj_prop` rows and `conj_source_reading` rows, and packs the
/// result as a [`ConjData`] per `(conjugation, conj-prop)` pair. When
/// `texts` is non-empty, conjugations whose `conj_source_reading.text`
/// doesn't intersect that set are dropped.
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

/// Port of `ichiran/dict:select-conjs` (`dict.lisp:1603`).
///
/// Selects the conjugation rows for `seq`: restricted to `conj_ids`
/// when given, or the root (null-via) rows otherwise.
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

/// Port of `ichiran/dict:conj-type-order` (`dict.lisp:1612`).
///
/// Swaps conj-type 10 and 13 (Continuative and Imperative) so the
/// former sorts first; all other types pass through unchanged.
pub fn conj_type_order(conj_type: i32) -> i32 {
    match conj_type {
        10 => 13,
        13 => 10,
        _ => conj_type,
    }
}

/// Port of `ichiran/dict:is-rareru` (`dict.lisp:1619`).
///
/// True when `text` ends with one of the passive/potential forms
/// られる / られます / られない / られません.
pub fn is_rareru(text: &str) -> bool {
    text.ends_with("られる")
        || text.ends_with("られます")
        || text.ends_with("られない")
        || text.ends_with("られません")
}

/// Port of `ichiran/dict:filter-props` (`dict.lisp:1627`).
///
/// Removes Passive (conj-type 6) props on `v1`/`v1s`/`vk` words whose
/// text isn't a れる form. `text` may be nil, one string, or a list.
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

/// Port of `ichiran/dict:select-conjs-and-props` (`dict.lisp:1638`).
///
/// Looks up the conjugations for `seq`, pairs each with its filtered
/// conjugation properties, and sorts by (root-vs-via, conj-type-order).
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

/// Port of `ichiran/dict:print-conj-info` (`dict.lisp:1648`).
///
/// Writes a human-readable conjugation chain for `seq` into `out`,
/// recursing through each `via` source entry it hasn't already printed.
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

/// Port of `ichiran/dict:conj-info-json*` (`dict.lisp:1664`).
///
/// Builds the per-conjugation JSON objects (prop list, reading, gloss,
/// and any `via` chain) for an entry's conjugation data, recursing into
/// [`crate::dict::conj::conj_info_json`] for `via`-linked sources.
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

/// Port of `ichiran/dict:conj-info-json` (`dict.lisp:1697`).
///
/// Wraps [`crate::dict::conj::conj_info_json_star_`], keeping
/// only the `readok`-true entries and falling back to the full list
/// when none pass.
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

/// Port of `ichiran/dict:simplify-reading-list` (`dict.lisp:1704`).
///
/// Collapse a list of readings into one display string per distinct
/// de-spaced reading, marking the word boundaries the spaces encoded.
/// For each reading the spaces are stripped and the boundary positions
/// recorded; readings sharing the same de-spaced text are merged. A
/// boundary every merged reading agrees on becomes a space; a boundary
/// only some readings carried becomes a `·` (`#\MIDDLE_DOT`, U+00B7).
/// Mirrors `(remove-duplicates positions)`. Order is unobservable here
/// (callers either sort the result or use it only for `count`), so this
/// keeps first occurrences.
fn remove_duplicates(positions: &[usize]) -> Vec<usize> {
    let mut out: Vec<usize> = Vec::new();
    for &position in positions {
        if !out.contains(&position) {
            out.push(position);
        }
    }
    out
}

pub fn simplify_reading_list(reading_list: &[String]) -> Vec<String> {
    // assoc entries are `(can cnt . spos)`: de-spaced text, count of
    // readings that mapped to it, and the accumulated boundary positions
    // (each reading's positions concatenated, cross-reading duplicates
    // kept so `count` below can measure agreement). Kept in encounter
    // order (Lisp builds the reverse and `nreverse`s; see the push arm).
    let mut assoc: Vec<(String, i32, Vec<usize>)> = Vec::new();
    for reading in reading_list {
        let mut can = String::new();
        let mut spos: Vec<usize> = Vec::new();
        let mut off: usize = 0;
        for (i, char) in reading.chars().enumerate() {
            if char == ' ' {
                spos.push(i - off);
                off += 1;
            } else {
                can.push(char);
            }
        }
        let spos = remove_duplicates(&spos);
        match assoc.iter_mut().find(|(entry_can, _, _)| *entry_can == can) {
            // dict.lisp:1713 — (setf (cddr a) (nconc spos (cddr a))) (incf (cadr a)).
            Some(entry) => {
                entry.2.extend(spos);
                entry.1 += 1;
            }
            // dict.lisp:1714 — (push (list* can 1 spos) assoc). Lisp pushes
            // onto the front and `nreverse`s at the end; appending here builds
            // the same encounter order directly.
            None => assoc.push((can, 1, spos)),
        }
    }
    let mut out: Vec<String> = Vec::with_capacity(assoc.len());
    for (can, cnt, spos) in &assoc {
        let mut sspos = remove_duplicates(spos);
        sspos.sort_unstable();
        let mut sspos = sspos.into_iter().peekable();
        let mut s = String::new();
        for (i, char) in can.chars().enumerate() {
            if sspos.peek() == Some(&i) {
                let count = spos.iter().filter(|&&position| position == i).count() as i32;
                s.push(if count == *cnt { ' ' } else { '\u{00B7}' });
                sspos.next();
            }
            s.push(char);
        }
        out.push(s);
    }
    out
}

/// Port of `ichiran/dict:conj-data-from` (`dict.lisp:325`).
///
/// Returns a [`ConjData`]'s `from` field — the seq id of the source
/// entry the conjugation derives from.
pub fn conj_data_from(cd: &ConjData) -> Option<i32> {
    cd.from
}

/// Port of `ichiran/dict:conj-data-prop` (`dict.lisp:325`).
///
/// Returns a [`crate::dict::conj::ConjData`]'s `prop` field — its
/// `conj-prop` row.
pub fn conj_data_prop(cd: &ConjData) -> Option<ConjProp> {
    cd.prop.clone()
}

/// Port of `ichiran/dict:make-conj-data` (`dict.lisp:325`).
///
/// Constructor for the `conj-data` struct (slots seq, from, via, prop,
/// src-map).
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

#[cfg(test)]
mod tests;
