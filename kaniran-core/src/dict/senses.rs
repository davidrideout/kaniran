//! Port of the dict.lisp senses + readings-str layer —
//! get-senses[-raw/-str/-json], match-kana-kanji,
//! match-sense-restrictions, split-pos, short-sense-str,
//! reading-str[/star/-seq], entry-info-{short,long}, get-glosses,
//! match-glosses, is-rareru.

use crate::characters::text_utils::join;
use crate::conn::kani_context::KaniranContext;
use crate::dict::best_text::{get_kanji, word_info_reading_str};
use crate::dict::counters::dispatchers::text;
use crate::dict::counters::find_counter::nokanji;
use crate::dict::dao::{KanaText, KanjiText};
use crate::dict::find_word::get_candidates;
use crate::dict::kani::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use crate::dict::word_info::{word_type, WordInfo, WordType};
use serde_json::{Map, Value};
use sqlx::Row;
use std::fmt::Write;
use std::future::Future;

pub type SenseEntry = (String, String, Vec<(String, Vec<String>)>);

pub async fn get_senses(ctx: &KaniranContext, seq: i32) -> Result<Vec<SenseEntry>, sqlx::Error> {
    let raw = get_senses_raw(ctx, seq).await?;
    let mut out: Vec<SenseEntry> = Vec::with_capacity(raw.len());
    for sense in raw {
        let pos_str = {
            let pos: &[String] = sense
                .props
                .iter()
                .find(|(tag, _)| tag == "pos")
                .map(|(_, vals)| vals.as_slice())
                .unwrap_or(&[]);
            format!("[{}]", pos.join(","))
        };
        out.push((pos_str, sense.gloss, sense.props));
    }
    Ok(out)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSense {
    pub ord: i32,
    pub gloss: String,
    pub props: Vec<(String, Vec<String>)>,
}

const TAGS: &[&str] = &["pos", "s_inf", "stagk", "stagr", "field"];

pub async fn get_senses_raw(ctx: &KaniranContext, seq: i32) -> Result<Vec<RawSense>, sqlx::Error> {
    let gloss_rows = sqlx::query(
        "SELECT sense.ord AS ord, \
                string_agg(gloss.text, '; ' ORDER BY gloss.ord) AS gloss \
         FROM sense LEFT JOIN gloss ON gloss.sense_id = sense.id \
         WHERE sense.seq = $1 \
         GROUP BY sense.id \
         ORDER BY sense.ord",
    )
    .bind(seq)
    .fetch_all(&ctx.pool)
    .await?;

    let mut sense_list: Vec<RawSense> = Vec::with_capacity(gloss_rows.len());
    for row in gloss_rows {
        let ord: i32 = row.get("ord");
        let gloss: Option<String> = row.get("gloss");
        sense_list.push(RawSense {
            ord,
            gloss: gloss.unwrap_or_default(),
            props: Vec::new(),
        });
    }

    let prop_rows = sqlx::query(
        "SELECT sense.ord AS ord, sense_prop.tag AS tag, sense_prop.text AS text \
         FROM sense, sense_prop \
         WHERE sense.seq = $1 \
           AND sense_prop.sense_id = sense.id \
           AND sense_prop.tag = ANY($2) \
         ORDER BY sense.ord, sense_prop.tag, sense_prop.ord",
    )
    .bind(seq)
    .bind(TAGS)
    .fetch_all(&ctx.pool)
    .await?;

    let mut cur_sord: Option<i32> = None;
    let mut cur_tag: Option<String> = None;
    let mut cur_idx: Option<usize> = None;
    let mut bag: Vec<String> = Vec::new();

    for row in prop_rows {
        let sord: i32 = row.get("ord");
        let tag: String = row.get("tag");
        let text: String = row.get("text");

        let changed = cur_sord != Some(sord) || cur_tag.as_deref() != Some(tag.as_str());
        if changed {
            // dict.lisp:1479 (in-loop transition) — emit prior bag in
            // upstream insertion order (Lisp `(reverse bag)` flips
            // `push`-prepended order; Rust `Vec::push` is already in
            // insertion order so no reverse is applied).
            if let Some(idx) = cur_idx {
                let prev_tag = cur_tag.take().unwrap_or_default();
                let prev_bag = std::mem::take(&mut bag);
                sense_list[idx].props.insert(0, (prev_tag, prev_bag));
            }
            cur_sord = Some(sord);
            cur_tag = Some(tag);
            bag.clear();
            cur_idx = sense_list.iter().position(|s| s.ord == sord);
        }
        bag.push(text);
    }
    // dict.lisp:1483 (finally clause) — upstream emits `(cons curtag
    // bag)` without `reverse`, leaving the final group's texts in
    // reverse insertion order. The Rust `Vec::push` produced
    // insertion order, so reverse here to mirror the asymmetry.
    if let Some(idx) = cur_idx {
        let prev_tag = cur_tag.take().unwrap_or_default();
        bag.reverse();
        sense_list[idx].props.insert(0, (prev_tag, bag));
    }

    Ok(sense_list)
}

pub async fn get_senses_str(ctx: &KaniranContext, seq: i32) -> Result<String, sqlx::Error> {
    let senses = get_senses(ctx, seq).await?;
    let mut out = String::new();
    let mut rpos: &str = "";
    for (i, (pos, gloss, props)) in senses.iter().enumerate() {
        // dict.lisp:1499 (loop for rpos = pos then …) — first iter seeds rpos,
        // later iters keep the prior rpos when the current pos is "[]".
        if i == 0 {
            rpos = pos.as_str();
        } else {
            if pos != "[]" {
                rpos = pos.as_str();
            }
            out.push('\n');
        }
        let inf = props
            .iter()
            .find(|(tag, _)| tag == "s_inf")
            .map(|(_, vals)| join("; ", vals));
        let field = props
            .iter()
            .find(|(tag, _)| tag == "field")
            .map(|(_, vals)| join(",", vals));
        write!(out, "{}. {} ", i + 1, rpos).unwrap();
        if let Some(f) = &field {
            write!(out, "{{{}}} ", f).unwrap();
        }
        if let Some(s) = &inf {
            write!(out, "《{}》 ", s).unwrap();
        }
        out.push_str(gloss);
    }
    Ok(out)
}

pub async fn get_senses_json<Fut>(
    ctx: &KaniranContext,
    seq: i32,
    pos_list: &[String],
    reading: Option<KaniWordDispatchEnum>,
    reading_getter: Option<Fut>,
) -> Result<Vec<Value>, sqlx::Error>
where
    Fut: Future<Output = Result<Option<KaniWordDispatchEnum>, sqlx::Error>>,
{
    let has_reading_getter = reading_getter.is_some();
    let mut reading_getter = reading_getter;
    let mut reading = reading;
    let mut readp = false;
    let mut rpos = String::new();
    let mut lpos: Vec<String> = Vec::new();
    let mut first = true;
    let mut out: Vec<Value> = Vec::new();

    for (pos, gloss, props) in get_senses(ctx, seq).await? {
        let emptypos = pos == "[]";
        // for rpos / lpos = … then (if emptypos … …): first iteration uses
        // the raw value, later iterations keep the prior on an empty pos.
        if first || !emptypos {
            rpos = pos.clone();
            lpos = split_pos(&pos).into_iter().map(str::to_owned).collect();
            first = false;
        }
        let rinf = props
            .iter()
            .find(|(tag, _)| tag == "s_inf")
            .map(|(_, inf)| join("; ", inf));
        let rfield = props
            .iter()
            .find(|(tag, _)| tag == "field")
            .map(|(_, field)| format!("{{{}}}", field.join(",")));

        // (or (not pos-list) (intersection lpos pos-list :test 'equal))
        let cond1 = pos_list.is_empty() || lpos.iter().any(|lp| pos_list.iter().any(|q| q == lp));
        let collect_this = if !cond1 {
            false
        } else if !(has_reading_getter || reading.is_some()) {
            // (not (or reading-getter reading))
            true
        } else if !props
            .iter()
            .any(|(tag, _)| tag == "stagk" || tag == "stagr")
        {
            // (not (or (assoc "stagk" props) (assoc "stagr" props)))
            true
        } else {
            // (let ((rr (or reading (and (not readp) (setf readp t reading (funcall reading-getter)))))) …)
            if reading.is_none() && !readp {
                readp = true;
                reading = match reading_getter.take() {
                    Some(fut) => fut.await?,
                    None => None,
                };
            }
            // (if rr (match-sense-restrictions seq props rr) t)
            match &reading {
                Some(rr) => match_sense_restrictions(ctx, seq, &props, rr)
                    .await?
                    .is_some(),
                None => true,
            }
        };

        if collect_this {
            let mut js = Map::new();
            js.insert("pos".to_owned(), Value::String(rpos.clone()));
            js.insert("gloss".to_owned(), Value::String(gloss));
            if let Some(rfield) = rfield {
                js.insert("field".to_owned(), Value::String(rfield));
            }
            if let Some(rinf) = rinf {
                js.insert("info".to_owned(), Value::String(rinf));
            }
            out.push(Value::Object(js));
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchKanaKanjiResult {
    /// Lisp `t` — the kana reading carries no restricted-reading
    /// constraint, so any kanji pairs.
    Yes,
    /// Lisp `(find (text kanji-reading) restr :test 'equal)` — the
    /// matched kanji surface.
    Found(String),
}

pub fn match_kana_kanji(
    kana_reading: &KaniWordDispatchEnum,
    kanji_reading: &KaniWordDispatchEnum,
    restricted: &[(String, String)],
) -> Option<MatchKanaKanjiResult> {
    // dict.lisp:1508 ((nokanji kana-reading) nil) — `nokanji` has no
    // method for compound-text upstream (no-applicable-method); the
    // dispatcher returns None there and `.expect` surfaces that error.
    if nokanji(kana_reading).expect("nokanji: no method for compound-text") {
        return None;
    }
    // dict.lisp:1509 (kana-text (text kana-reading))
    let kana_text = text(kana_reading);
    let kana_text = kana_text.as_ref();
    // dict.lisp:1510 (restr (loop for (rt kt) in restricted when (equal kana-text rt) collect kt))
    let restr: Vec<&str> = restricted
        .iter()
        .filter(|(rt, _kt)| rt.as_str() == kana_text)
        .map(|(_rt, kt)| kt.as_str())
        .collect();
    // dict.lisp:1511-1513 (if restr (find (text kanji-reading) restr :test 'equal) t)
    if !restr.is_empty() {
        let kanji_text = text(kanji_reading);
        if restr.iter().any(|kt| *kt == kanji_text.as_ref()) {
            Some(MatchKanaKanjiResult::Found(kanji_text.into_owned()))
        } else {
            None
        }
    } else {
        Some(MatchKanaKanjiResult::Yes)
    }
}

pub async fn match_sense_restrictions(
    ctx: &KaniranContext,
    seq: i32,
    props: &[(String, Vec<String>)],
    reading: &KaniWordDispatchEnum,
) -> Result<Option<MatchKanaKanjiResult>, sqlx::Error> {
    // dict.lisp:1516-1517 (stagk/stagr (cdr (assoc … props :test 'equal)))
    let stagk: &[String] = props
        .iter()
        .find(|(tag, _)| tag == "stagk")
        .map_or(&[], |(_, texts)| texts.as_slice());
    let stagr: &[String] = props
        .iter()
        .find(|(tag, _)| tag == "stagr")
        .map_or(&[], |(_, texts)| texts.as_slice());
    // dict.lisp:1518 (wtype (word-type reading))
    let wtype = word_type(reading);

    // dict.lisp:1519 ((and (not stagk) (not stagr)) t)
    if stagk.is_empty() && stagr.is_empty() {
        return Ok(Some(MatchKanaKanjiResult::Yes));
    }
    // dict.lisp:1520-1521 ((or (member (text reading) stagk) (member (text reading) stagr)) t)
    let reading_text = text(reading);
    let reading_text = reading_text.as_ref();
    if stagk.iter().any(|t| t.as_str() == reading_text)
        || stagr.iter().any(|t| t.as_str() == reading_text)
    {
        return Ok(Some(MatchKanaKanjiResult::Yes));
    }
    // dict.lisp:1522 ((and (not stagr) (eql wtype :kanji)) nil)
    if stagr.is_empty() && wtype == WordType::Kanji {
        return Ok(None);
    }
    // dict.lisp:1523 ((and (not stagk) (eql wtype :kana)) nil)
    if stagk.is_empty() && wtype == WordType::Kana {
        return Ok(None);
    }
    // dict.lisp:1524 (restricted (query (:select 'reading 'text :from 'restricted-readings :where (:= 'seq seq))))
    let restricted: Vec<(String, String)> =
        sqlx::query_as("SELECT reading, text FROM restricted_readings WHERE seq = $1")
            .bind(seq)
            .fetch_all(&ctx.pool)
            .await?;
    // dict.lisp:1525-1532 (case wtype …)
    match wtype {
        WordType::Kanji => {
            // dict.lisp:1527 (rkana (select-dao 'kana-text (:and (:= 'seq seq) (:in 'text (:set stagr)))))
            let rkana: Vec<KanaText> =
                sqlx::query_as("SELECT * FROM kana_text WHERE seq = $1 AND text = ANY($2)")
                    .bind(seq)
                    .bind(stagr)
                    .fetch_all(&ctx.pool)
                    .await?;
            // dict.lisp:1528 (some (lambda (rk) (match-kana-kanji rk reading restricted)) rkana)
            Ok(rkana.into_iter().find_map(|rk| {
                match_kana_kanji(&KaniWordDispatchEnum::Kana(rk), reading, &restricted)
            }))
        }
        WordType::Kana => {
            // dict.lisp:1530 (rkanji (select-dao 'kanji-text (:and (:= 'seq seq) (:in 'text (:set stagk)))))
            let rkanji: Vec<KanjiText> =
                sqlx::query_as("SELECT * FROM kanji_text WHERE seq = $1 AND text = ANY($2)")
                    .bind(seq)
                    .bind(stagk)
                    .fetch_all(&ctx.pool)
                    .await?;
            // dict.lisp:1531 (some (lambda (rk) (match-kana-kanji reading rk restricted)) rkanji)
            Ok(rkanji.into_iter().find_map(|rk| {
                match_kana_kanji(reading, &KaniWordDispatchEnum::Kanji(rk), &restricted)
            }))
        }
        // (case wtype …) has no :gap clause → nil
        WordType::Gap => Ok(None),
    }
}

pub fn split_pos(pos_str: &str) -> Vec<&str> {
    // :start 1 :end (1- (length pos-str)) — code-point offsets.
    let char_count = pos_str.chars().count();
    let start = pos_str
        .char_indices()
        .nth(1)
        .map_or(pos_str.len(), |(byte, _)| byte);
    let end = pos_str
        .char_indices()
        .nth(char_count - 1)
        .map_or(pos_str.len(), |(byte, _)| byte);
    pos_str[start..end].split(',').collect()
}

pub async fn short_sense_str(
    ctx: &KaniranContext,
    seq: i32,
    with_pos: Option<&str>,
) -> Result<Option<String>, sqlx::Error> {
    // dict.lisp:1562 — `,@(if with-pos …)` splices the sense-prop join
    // only when with-pos is supplied.
    let single: Option<Option<String>> = match with_pos {
        Some(with_pos) => {
            sqlx::query_scalar(
                "SELECT (SELECT string_agg(gloss.text, '; ' ORDER BY gloss.ord) \
                         FROM gloss WHERE gloss.sense_id = sense.id) \
                 FROM sense \
                 INNER JOIN sense_prop AS pos \
                   ON (pos.sense_id = sense.id AND pos.tag = 'pos' AND pos.text = $2) \
                 WHERE sense.seq = $1 \
                 GROUP BY sense.id \
                 ORDER BY sense.ord \
                 LIMIT 1",
            )
            .bind(seq)
            .bind(with_pos)
            .fetch_optional(&ctx.pool)
            .await?
        }
        None => {
            sqlx::query_scalar(
                "SELECT (SELECT string_agg(gloss.text, '; ' ORDER BY gloss.ord) \
                         FROM gloss WHERE gloss.sense_id = sense.id) \
                 FROM sense \
                 WHERE sense.seq = $1 \
                 GROUP BY sense.id \
                 ORDER BY sense.ord \
                 LIMIT 1",
            )
            .bind(seq)
            .fetch_optional(&ctx.pool)
            .await?
        }
    };
    Ok(single.flatten())
}

impl WordInfo {
    // dict.lisp:1739-1743 (defmethod reading-str ((word-info word-info)) / ((word-info list)))
    pub fn reading_str(&self) -> Option<String> {
        word_info_reading_str(self)
    }
}

impl KaniSimpleTextDispatchEnum {
    // dict.lisp:1589-1590 (defmethod reading-str ((obj simple-text)))
    pub async fn reading_str(&self, ctx: &KaniranContext) -> Result<Option<String>, sqlx::Error> {
        let kanji = get_kanji(ctx, &self.to_word()).await?;
        let kana = self.get_kana(ctx).await?;
        Ok(reading_str_star_(kanji.as_deref(), kana.as_deref()))
    }
}

pub async fn reading_str_seq(
    ctx: &KaniranContext,
    seq: i32,
) -> Result<Option<String>, sqlx::Error> {
    let kanji_text: Option<String> =
        sqlx::query_scalar("SELECT text FROM kanji_text WHERE seq = $1 AND ord = 0")
            .bind(seq)
            .fetch_optional(&ctx.pool)
            .await?;
    let kana_text: Option<String> =
        sqlx::query_scalar("SELECT text FROM kana_text WHERE seq = $1 AND ord = 0")
            .bind(seq)
            .fetch_optional(&ctx.pool)
            .await?;
    Ok(reading_str_star_(
        kanji_text.as_deref(),
        kana_text.as_deref(),
    ))
}

pub fn reading_str_star_(kanji: Option<&str>, kana: Option<&str>) -> Option<String> {
    match kanji {
        // ~a of nil prints "NIL"
        Some(kanji) => Some(format!("{} 【{}】", kanji, kana.unwrap_or("NIL"))),
        None => kana.map(str::to_string),
    }
}

pub async fn entry_info_short(
    ctx: &KaniranContext,
    seq: i32,
    with_pos: Option<&str>,
) -> Result<String, sqlx::Error> {
    let sense_str = short_sense_str(ctx, seq, with_pos).await?;
    // ~a of a nil reading prints "NIL"
    let mut s = format!(
        "{} : ",
        reading_str_seq(ctx, seq).await?.as_deref().unwrap_or("NIL")
    );
    if let Some(sense_str) = sense_str {
        s.push_str(&sense_str);
    }
    Ok(s)
}

pub async fn entry_info_long(ctx: &KaniranContext, seq: i32) -> Result<String, sqlx::Error> {
    let mut out = seq.to_string();
    // dict.lisp:1601 (~@[ ~a~%~]) — " <reading>\n" only when reading-str-seq
    // is non-nil; nil prints nothing (unlike entry-info-short's bare ~a).
    if let Some(reading) = reading_str_seq(ctx, seq).await? {
        writeln!(out, " {}", reading).unwrap();
    }
    out.push_str(&get_senses_str(ctx, seq).await?);
    Ok(out)
}

pub async fn get_glosses(
    ctx: &KaniranContext,
    seqs: &[i32],
) -> Result<Vec<(i32, Vec<String>)>, sqlx::Error> {
    let glosses: Vec<(i32, String)> = sqlx::query_as(
        "SELECT sense.seq, gloss.text FROM gloss, sense \
         WHERE sense.seq = ANY($1) AND gloss.sense_id = sense.id \
         ORDER BY sense.seq",
    )
    .bind(seqs)
    .fetch_all(&ctx.pool)
    .await?;

    let mut al: Vec<(i32, Vec<String>)> = Vec::new();
    for (seq, text) in glosses {
        // dict.lisp:1896-1899 — `if (eql (caar al) seq) do (push text (cdar al))`
        match al.last_mut() {
            Some((s, inner)) if *s == seq => inner.insert(0, text),
            _ => al.push((seq, vec![text])),
        }
    }
    Ok(al)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchValue {
    /// Upstream returned `seq` alone — either the all-words-match arm
    /// (`found_p = true`) or the no-match fallback arm using
    /// `(car candidates)` (`found_p = false`).
    Seq(i32),
    /// Upstream returned `(list seq match)` — the `update-gloss`
    /// regex arm. `match` is the original (un-normalized) gloss
    /// string.
    SeqAndGloss(i32, String),
}

pub async fn match_glosses(
    ctx: &KaniranContext,
    text: &str,
    reading: Option<&str>,
    words: &[&str],
    normalize: Option<&dyn Fn(&str) -> String>,
    update_gloss: Option<&fancy_regex::Regex>,
) -> Result<Option<(MatchValue, bool)>, sqlx::Error> {
    let candidates = get_candidates(ctx, text, reading).await?;
    if candidates.is_empty() {
        return Ok(None);
    }
    // dict.lisp:1923 — `(nwords (mapcar normalize words))`
    let nwords: Vec<String> = words
        .iter()
        .map(|w| match normalize {
            Some(f) => f(w),
            None => (*w).to_string(),
        })
        .collect();

    // dict.lisp:1925 — `(loop for (seq . glosses) in (get-glosses candidates) ...)`
    let groups = get_glosses(ctx, &candidates).await?;
    for (seq, glosses) in groups {
        // dict.lisp:1926 — `(loop for gloss in (nreverse glosses) ...)`
        let dbo_glosses: Vec<String> = glosses.into_iter().rev().collect();
        let mut inner: Option<MatchValue> = None;
        for gloss in &dbo_glosses {
            // dict.lisp:1927 — `for ngloss = (funcall normalize gloss)`
            let ngloss = match normalize {
                Some(f) => f(gloss),
                None => gloss.clone(),
            };
            // dict.lisp:1928-1929 —
            // `when (and update-gloss (ppcre:scan update-gloss ngloss))
            //    do (return gloss)`
            if let Some(rg) = update_gloss {
                if rg
                    .is_match(&ngloss)
                    .expect("match-glosses: fancy_regex runtime error")
                {
                    inner = Some(MatchValue::SeqAndGloss(seq, gloss.clone()));
                    break;
                }
            }
            // dict.lisp:1930 —
            // `thereis (loop for word in nwords always (search word ngloss))`
            if nwords.iter().all(|w| ngloss.contains(w.as_str())) {
                inner = Some(MatchValue::Seq(seq));
                break;
            }
        }
        if let Some(m) = inner {
            return Ok(Some((m, true)));
        }
    }
    // dict.lisp:1934 — `(values (car candidates) nil)`
    Ok(Some((MatchValue::Seq(candidates[0]), false)))
}

pub fn is_rareru(text: &str) -> bool {
    text.ends_with("られる")
        || text.ends_with("られます")
        || text.ends_with("られない")
        || text.ends_with("られません")
}

#[cfg(test)]
mod get_senses_tests {
    //! All expected values pinned against .103 REPL runs of
    //! `(ichiran/dict::get-senses <seq>)`. Run with `--test-threads=1`.
    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    #[tokio::test]
    async fn unknown_seq_returns_empty() {
        // REPL: (get-senses 999999) => NIL
        let ctx = ctx_from_env().await;
        let result = get_senses(&ctx, 999999).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn simple_single_sense() {
        // REPL: (get-senses 1582710)
        // => (("[n]" "Japan" (("pos" "n"))))
        let ctx = ctx_from_env().await;
        let result = get_senses(&ctx, 1582710).await.unwrap();
        assert_eq!(
            result,
            vec![(
                "[n]".to_string(),
                "Japan".to_string(),
                vec![("pos".to_string(), vec!["n".to_string()])],
            )]
        );
    }

    #[tokio::test]
    async fn multi_value_pos() {
        // REPL: (get-senses 1577900)
        // => (("[adj-no,n]" "eternity" (("pos" "adj-no" "n"))))
        let ctx = ctx_from_env().await;
        let result = get_senses(&ctx, 1577900).await.unwrap();
        assert_eq!(
            result,
            vec![(
                "[adj-no,n]".to_string(),
                "eternity".to_string(),
                vec![(
                    "pos".to_string(),
                    vec!["adj-no".to_string(), "n".to_string()]
                )],
            )]
        );
    }

    #[tokio::test]
    async fn field_tag_preserved_in_props() {
        // REPL: (get-senses 1001390)
        // => (("[n]"
        //       "oden; dish of various ingredients, e.g. egg, daikon,
        //        potato, chikuwa, konnyaku stewed in soy-flavored dashi"
        //       (("pos" "n") ("field" "food"))))
        let ctx = ctx_from_env().await;
        let result = get_senses(&ctx, 1001390).await.unwrap();
        assert_eq!(
            result,
            vec![(
                "[n]".to_string(),
                "oden; dish of various ingredients, e.g. egg, daikon, potato, chikuwa, konnyaku stewed in soy-flavored dashi".to_string(),
                vec![
                    ("pos".to_string(), vec!["n".to_string()]),
                    ("field".to_string(), vec!["food".to_string()]),
                ],
            )]
        );
    }

    #[tokio::test]
    async fn second_sense_no_pos_yields_empty_brackets() {
        // REPL: (get-senses 1447690)
        // => (("[n]" "Tokyo" (("pos" "n")))
        //     ("[]" "Tokyo Metropolis" NIL))
        let ctx = ctx_from_env().await;
        let result = get_senses(&ctx, 1447690).await.unwrap();
        assert_eq!(
            result,
            vec![
                (
                    "[n]".to_string(),
                    "Tokyo".to_string(),
                    vec![("pos".to_string(), vec!["n".to_string()])],
                ),
                ("[]".to_string(), "Tokyo Metropolis".to_string(), Vec::new(),),
            ]
        );
    }
}

#[cfg(test)]
mod get_senses_raw_tests {
    //! All expected values pinned against .103 REPL runs of
    //! `(get-senses-raw <seq>)`. Test threads must be 1 —
    //! `cargo test --test-threads=1` per the project's DB-test
    //! convention.
    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    #[tokio::test]
    async fn unknown_seq_returns_empty() {
        // REPL: (get-senses-raw 999999) => NIL
        let ctx = ctx_from_env().await;
        let result = get_senses_raw(&ctx, 999999).await.unwrap();
        assert_eq!(result, Vec::<RawSense>::new());
    }

    #[tokio::test]
    async fn simple_single_sense() {
        // REPL: (get-senses-raw 1582710)
        // => ((:ORD 0 :GLOSS "Japan" :PROPS (("pos" "n"))))
        let ctx = ctx_from_env().await;
        let result = get_senses_raw(&ctx, 1582710).await.unwrap();
        assert_eq!(
            result,
            vec![RawSense {
                ord: 0,
                gloss: "Japan".to_string(),
                props: vec![("pos".to_string(), vec!["n".to_string()])],
            }]
        );
    }

    #[tokio::test]
    async fn multi_value_pos_single_sense() {
        // REPL: (get-senses-raw 1577900)
        // => ((:ORD 0 :GLOSS "eternity" :PROPS (("pos" "adj-no" "n"))))
        let ctx = ctx_from_env().await;
        let result = get_senses_raw(&ctx, 1577900).await.unwrap();
        assert_eq!(
            result,
            vec![RawSense {
                ord: 0,
                gloss: "eternity".to_string(),
                props: vec![(
                    "pos".to_string(),
                    vec!["adj-no".to_string(), "n".to_string()],
                )],
            }]
        );
    }

    #[tokio::test]
    async fn field_tag_present() {
        // REPL: (get-senses-raw 1001390)
        // => ((:ORD 0 :GLOSS "oden; dish of various ingredients, e.g.
        //      egg, daikon, potato, chikuwa, konnyaku stewed in
        //      soy-flavored dashi"
        //     :PROPS (("pos" "n") ("field" "food"))))
        let ctx = ctx_from_env().await;
        let result = get_senses_raw(&ctx, 1001390).await.unwrap();
        assert_eq!(
            result,
            vec![RawSense {
                ord: 0,
                gloss: "oden; dish of various ingredients, e.g. egg, daikon, potato, chikuwa, konnyaku stewed in soy-flavored dashi".to_string(),
                props: vec![
                    ("pos".to_string(), vec!["n".to_string()]),
                    ("field".to_string(), vec!["food".to_string()]),
                ],
            }]
        );
    }

    #[tokio::test]
    async fn stagk_tag_and_multiple_pos() {
        // REPL: (get-senses-raw 1000300)
        // => ((:ORD 0 :GLOSS "to treat; to handle; to deal with"
        //      :PROPS (("stagk" "遇う") ("pos" "v5u" "vt")))
        //     (:ORD 1 :GLOSS "to arrange; to decorate (with); to adorn
        //      (with); to dress (with); to garnish (with)"
        //      :PROPS (("pos" "vt" "v5u"))))
        let ctx = ctx_from_env().await;
        let result = get_senses_raw(&ctx, 1000300).await.unwrap();
        assert_eq!(
            result,
            vec![
                RawSense {
                    ord: 0,
                    gloss: "to treat; to handle; to deal with".to_string(),
                    props: vec![
                        ("stagk".to_string(), vec!["遇う".to_string()]),
                        (
                            "pos".to_string(),
                            vec!["v5u".to_string(), "vt".to_string()],
                        ),
                    ],
                },
                RawSense {
                    ord: 1,
                    gloss: "to arrange; to decorate (with); to adorn (with); to dress (with); to garnish (with)".to_string(),
                    props: vec![(
                        "pos".to_string(),
                        vec!["vt".to_string(), "v5u".to_string()],
                    )],
                },
            ]
        );
    }

    #[tokio::test]
    async fn final_group_bag_not_reversed_asymmetry() {
        // REPL: (get-senses-raw 1011960)
        // Pins the upstream asymmetry: sense 1's `stagr` is
        // ("ボタボタ" "ぼたぼた") — the in-loop `(reverse bag)`
        // path; sense 2's `stagr` is ("ぼたぼた" "ボタボタ") — the
        // `finally` path without reverse. Same two sense_prop.ord
        // 0/1 rows in both senses.
        let ctx = ctx_from_env().await;
        let result = get_senses_raw(&ctx, 1011960).await.unwrap();
        assert_eq!(
            result,
            vec![
                RawSense {
                    ord: 0,
                    gloss: "dripping; trickling; drop by drop; in drops".to_string(),
                    props: vec![(
                        "pos".to_string(),
                        vec!["adv".to_string(), "adv-to".to_string(), "vs".to_string()],
                    )],
                },
                RawSense {
                    ord: 1,
                    gloss: "wet and heavy (snow, clay, etc.)".to_string(),
                    props: vec![
                        (
                            "stagr".to_string(),
                            vec!["ボタボタ".to_string(), "ぼたぼた".to_string()],
                        ),
                        (
                            "pos".to_string(),
                            vec!["adv".to_string(), "adv-to".to_string(), "vs".to_string()],
                        ),
                    ],
                },
                RawSense {
                    ord: 2,
                    gloss: "(moving) slowly".to_string(),
                    props: vec![
                        (
                            "stagr".to_string(),
                            vec!["ぼたぼた".to_string(), "ボタボタ".to_string()],
                        ),
                        (
                            "pos".to_string(),
                            vec!["adv".to_string(), "adv-to".to_string()],
                        ),
                    ],
                },
            ]
        );
    }

    #[tokio::test]
    async fn sense_with_no_props_yields_empty_props() {
        // REPL: (get-senses-raw 1447690)
        // => ((:ORD 0 :GLOSS "Tokyo" :PROPS (("pos" "n")))
        //     (:ORD 1 :GLOSS "Tokyo Metropolis" :PROPS NIL))
        let ctx = ctx_from_env().await;
        let result = get_senses_raw(&ctx, 1447690).await.unwrap();
        assert_eq!(
            result,
            vec![
                RawSense {
                    ord: 0,
                    gloss: "Tokyo".to_string(),
                    props: vec![("pos".to_string(), vec!["n".to_string()])],
                },
                RawSense {
                    ord: 1,
                    gloss: "Tokyo Metropolis".to_string(),
                    props: Vec::new(),
                },
            ]
        );
    }
}

#[cfg(test)]
mod get_senses_str_tests {
    //! All expected values pinned against .103 REPL runs of
    //! `(ichiran/dict::get-senses-str <seq>)`. Run with `--test-threads=1`.
    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    #[tokio::test]
    async fn unknown_seq_yields_empty_string() {
        // REPL: (get-senses-str 999999) => ""
        let ctx = ctx_from_env().await;
        let result = get_senses_str(&ctx, 999999).await.unwrap();
        assert_eq!(result, "");
    }

    #[tokio::test]
    async fn simple_single_sense() {
        // REPL: (get-senses-str 1582710) => "1. [n] Japan"
        let ctx = ctx_from_env().await;
        let result = get_senses_str(&ctx, 1582710).await.unwrap();
        assert_eq!(result, "1. [n] Japan");
    }

    #[tokio::test]
    async fn multi_value_pos() {
        // REPL: (get-senses-str 1577900) => "1. [adj-no,n] eternity"
        let ctx = ctx_from_env().await;
        let result = get_senses_str(&ctx, 1577900).await.unwrap();
        assert_eq!(result, "1. [adj-no,n] eternity");
    }

    #[tokio::test]
    async fn field_braced_before_gloss() {
        // REPL: (get-senses-str 1001390) =>
        //   "1. [n] {food} oden; dish of various ingredients, e.g. egg,
        //    daikon, potato, chikuwa, konnyaku stewed in soy-flavored dashi"
        let ctx = ctx_from_env().await;
        let result = get_senses_str(&ctx, 1001390).await.unwrap();
        assert_eq!(
            result,
            "1. [n] {food} oden; dish of various ingredients, e.g. egg, daikon, potato, chikuwa, konnyaku stewed in soy-flavored dashi"
        );
    }

    #[tokio::test]
    async fn multi_field_joined_by_comma() {
        // REPL: (get-senses-str 1014100) => "1. [n] {physics,chem} isotope"
        let ctx = ctx_from_env().await;
        let result = get_senses_str(&ctx, 1014100).await.unwrap();
        assert_eq!(result, "1. [n] {physics,chem} isotope");
    }

    #[tokio::test]
    async fn s_inf_in_double_angle_brackets() {
        // REPL: (get-senses-str 900000) =>
        //   "1. [suf] 《after the -masu stem of a verb》 to seem to want to (do something)"
        let ctx = ctx_from_env().await;
        let result = get_senses_str(&ctx, 900000).await.unwrap();
        assert_eq!(
            result,
            "1. [suf] 《after the -masu stem of a verb》 to seem to want to (do something)"
        );
    }

    #[tokio::test]
    async fn field_and_s_inf_both_present() {
        // REPL: (get-senses-str 1005660) =>
        //   "1. [n] {food} 《from the sound of the dish being prepared》 shabu-shabu; …"
        let ctx = ctx_from_env().await;
        let result = get_senses_str(&ctx, 1005660).await.unwrap();
        assert_eq!(
            result,
            "1. [n] {food} 《from the sound of the dish being prepared》 shabu-shabu; hot pot dish where thinly sliced meat is boiled quickly and then dipped in sauce"
        );
    }

    #[tokio::test]
    async fn multi_sense_separated_by_newline_no_trailing() {
        // REPL: (get-senses-str 1447690) =>
        //   "1. [n] Tokyo\n2. [n] Tokyo Metropolis"
        // sense 2 has pos "[]" → rpos inherits "[n]" from sense 1.
        let ctx = ctx_from_env().await;
        let result = get_senses_str(&ctx, 1447690).await.unwrap();
        assert_eq!(result, "1. [n] Tokyo\n2. [n] Tokyo Metropolis");
    }

    #[tokio::test]
    async fn three_senses_mixed_props() {
        // REPL: (get-senses-str 1011960) =>
        //   "1. [adv,adv-to,vs] dripping; trickling; drop by drop; in drops
        //    2. [adv,adv-to,vs] wet and heavy (snow, clay, etc.)
        //    3. [adv,adv-to] (moving) slowly"
        let ctx = ctx_from_env().await;
        let result = get_senses_str(&ctx, 1011960).await.unwrap();
        assert_eq!(
            result,
            "1. [adv,adv-to,vs] dripping; trickling; drop by drop; in drops\n2. [adv,adv-to,vs] wet and heavy (snow, clay, etc.)\n3. [adv,adv-to] (moving) slowly"
        );
    }

    #[tokio::test]
    async fn five_senses_with_s_inf_subset() {
        // REPL: (get-senses-str 1000090) — pinned 5-sense output.
        let ctx = ctx_from_env().await;
        let result = get_senses_str(&ctx, 1000090).await.unwrap();
        assert_eq!(
            result,
            "1. [n] 《sometimes used for zero》 circle\n2. [n] 《when marking a test, homework, etc.》 \"correct\"; \"good\"\n3. [unc] 《placeholder used to censor individual characters or indicate a space to be filled in》 *; _\n4. [n] period; full stop\n5. [n] handakuten (diacritic)"
        );
    }
}

#[cfg(test)]
mod get_senses_json_tests {
    use super::*;
    use crate::dict::dao::{KanaText, KanjiText};
    use std::future::Ready;
    use std::sync::Arc;

    type GetterFut = Ready<Result<Option<KaniWordDispatchEnum>, sqlx::Error>>;

    async fn ctx_from_env() -> Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    fn json(values: &[Value]) -> String {
        serde_json::to_string(values).unwrap()
    }

    async fn kanji_reading(ctx: &KaniranContext, seq: i32, text: &str) -> KaniWordDispatchEnum {
        let row: KanjiText =
            sqlx::query_as("SELECT * FROM kanji_text WHERE seq = $1 AND text = $2")
                .bind(seq)
                .bind(text)
                .fetch_one(&ctx.pool)
                .await
                .unwrap();
        KaniWordDispatchEnum::Kanji(row)
    }

    async fn kana_reading(ctx: &KaniranContext, seq: i32, text: &str) -> KaniWordDispatchEnum {
        let row: KanaText = sqlx::query_as("SELECT * FROM kana_text WHERE seq = $1 AND text = $2")
            .bind(seq)
            .bind(text)
            .fetch_one(&ctx.pool)
            .await
            .unwrap();
        KaniWordDispatchEnum::Kana(row)
    }

    /// REPL fixtures (.103, `(jsown:to-json (get-senses-json …))`),
    /// 2026-05-24. No reading/getter, no pos-list: every sense is
    /// collected. Covers `field` ({food}), multi-pos `[adj-no,n]`, the
    /// `[]`-second-sense `rpos` carry-forward (1447690 → both `[n]`), and
    /// the `s_inf` → `info` path with non-ASCII text (serde emits raw
    /// UTF-8, not jsown's `\u` escapes).
    #[tokio::test]
    async fn plain_collect_all() {
        let ctx = ctx_from_env().await;
        let cases: &[(i32, &str)] = &[
            (
                1001390,
                r#"[{"pos":"[n]","gloss":"oden; dish of various ingredients, e.g. egg, daikon, potato, chikuwa, konnyaku stewed in soy-flavored dashi","field":"{food}"}]"#,
            ),
            (1577900, r#"[{"pos":"[adj-no,n]","gloss":"eternity"}]"#),
            (
                1447690,
                r#"[{"pos":"[n]","gloss":"Tokyo"},{"pos":"[n]","gloss":"Tokyo Metropolis"}]"#,
            ),
            (
                1000230,
                r#"[{"pos":"[exp]","gloss":"useless; no good; hopeless","info":"commonly used with i-adjective inflections, e.g. あかんかった, あかんくない"},{"pos":"[exp]","gloss":"cannot; must not; not allowed"}]"#,
            ),
            (
                1000320,
                r#"[{"pos":"[pn]","gloss":"there; over there; that place; yonder; you-know-where","info":"place physically distant from both speaker and listener"},{"pos":"[n]","gloss":"genitals; private parts; nether regions"},{"pos":"[n]","gloss":"that far; that much; that point","info":"something psychologically distant from both speaker and listener"}]"#,
            ),
        ];
        for (seq, expected) in cases {
            let result = get_senses_json(&ctx, *seq, &[], None, None::<GetterFut>)
                .await
                .unwrap();
            assert_eq!(json(&result), *expected, "seq={seq}");
        }
    }

    /// REPL fixtures (.103), 2026-05-24. `pos-list` filter against the
    /// carried-forward `lpos`. 1577900 keeps/drops on `n`/`xxx`; 1447690
    /// `n` keeps both senses (the `[]` sense inherits `lpos=["n"]`);
    /// 1000320 `n` drops the leading `pn` sense; 1199330 `ctr` keeps only
    /// the counter sense (mirrors the `:pos-list '("ctr")` call site).
    #[tokio::test]
    async fn pos_list_filter() {
        let ctx = ctx_from_env().await;
        struct Case {
            seq: i32,
            pos: Vec<String>,
            expected: &'static str,
        }
        let cases = [
            Case {
                seq: 1577900,
                pos: vec!["n".to_owned()],
                expected: r#"[{"pos":"[adj-no,n]","gloss":"eternity"}]"#,
            },
            Case {
                seq: 1577900,
                pos: vec!["xxx".to_owned()],
                expected: "[]",
            },
            Case {
                seq: 1447690,
                pos: vec!["n".to_owned()],
                expected: r#"[{"pos":"[n]","gloss":"Tokyo"},{"pos":"[n]","gloss":"Tokyo Metropolis"}]"#,
            },
            Case {
                seq: 1000320,
                pos: vec!["n".to_owned()],
                expected: r#"[{"pos":"[n]","gloss":"genitals; private parts; nether regions"},{"pos":"[n]","gloss":"that far; that much; that point","info":"something psychologically distant from both speaker and listener"}]"#,
            },
            Case {
                seq: 1199330,
                pos: vec!["ctr".to_owned()],
                expected: r#"[{"pos":"[ctr]","gloss":"counter for occurrences"}]"#,
            },
        ];
        for case in &cases {
            let result = get_senses_json(&ctx, case.seq, &case.pos, None, None::<GetterFut>)
                .await
                .unwrap();
            assert_eq!(
                json(&result),
                case.expected,
                "seq={} pos={:?}",
                case.seq,
                case.pos
            );
        }
    }

    /// REPL fixtures (.103), 2026-05-24. `:reading` restriction path
    /// (1339160 sense 1 has stagk 出し / stagr ダシ; 1115120 sense 1 has
    /// stagk 風太郎). A `stagk`/`stagr` member passes (出し / ダシ); a
    /// non-matching kanji is filtered (出汁 → nil; プー太郎 → nil); the
    /// restricted-reading `Found` path passes (ぷうたろう → 風太郎).
    #[tokio::test]
    async fn reading_restriction() {
        let ctx = ctx_from_env().await;
        let both_dashi = r#"[{"pos":"[n]","gloss":"dashi; Japanese soup stock made from fish and kelp","field":"{food}"},{"pos":"[n]","gloss":"pretext; excuse; pretense (pretence); dupe; front man"}]"#;
        let one_dashi = r#"[{"pos":"[n]","gloss":"dashi; Japanese soup stock made from fish and kelp","field":"{food}"}]"#;
        let one_taro = r#"[{"pos":"[n]","gloss":"unemployed person; vagabond; floater; vagrant"}]"#;
        let both_taro = r#"[{"pos":"[n]","gloss":"unemployed person; vagabond; floater; vagrant"},{"pos":"[n]","gloss":"day labourer (esp. on the docks)"}]"#;

        // 出汁 (kanji): sense 1 filtered (only ダシ nokanji matches → nil)
        let reading = kanji_reading(&ctx, 1339160, "出汁").await;
        let result = get_senses_json(&ctx, 1339160, &[], Some(reading), None::<GetterFut>)
            .await
            .unwrap();
        assert_eq!(json(&result), one_dashi, "1339160 出汁");
        // 出し (kanji): member of stagk → both pass
        let reading = kanji_reading(&ctx, 1339160, "出し").await;
        let result = get_senses_json(&ctx, 1339160, &[], Some(reading), None::<GetterFut>)
            .await
            .unwrap();
        assert_eq!(json(&result), both_dashi, "1339160 出し");
        // ダシ (kana): member of stagr → both pass
        let reading = kana_reading(&ctx, 1339160, "ダシ").await;
        let result = get_senses_json(&ctx, 1339160, &[], Some(reading), None::<GetterFut>)
            .await
            .unwrap();
        assert_eq!(json(&result), both_dashi, "1339160 ダシ");
        // プー太郎 (kanji): sense 1 filtered
        let reading = kanji_reading(&ctx, 1115120, "プー太郎").await;
        let result = get_senses_json(&ctx, 1115120, &[], Some(reading), None::<GetterFut>)
            .await
            .unwrap();
        assert_eq!(json(&result), one_taro, "1115120 プー太郎");
        // ぷうたろう (kana): match-kana-kanji Found(風太郎) → sense 1 passes
        let reading = kana_reading(&ctx, 1115120, "ぷうたろう").await;
        let result = get_senses_json(&ctx, 1115120, &[], Some(reading), None::<GetterFut>)
            .await
            .unwrap();
        assert_eq!(json(&result), both_taro, "1115120 ぷうたろう");
    }

    /// REPL fixtures (.103), 2026-05-24. `:reading-getter` lazy thunk.
    /// A getter yielding 出汁 filters sense 1 exactly like the eager
    /// `:reading` form; a getter yielding `nil` leaves the restricted
    /// sense in (the `(if rr … t)` fallthrough). 1011960 carries two
    /// stag-restricted senses (1 and 2): the nil getter fires once at
    /// sense 1 then sense 2 takes the `readp`-already-true / `reading`-nil
    /// path; the ぼたぼた getter fires once then sense 2 reuses the memoized
    /// `reading` (the `(or reading …)` short-circuit) — both keep all three.
    #[tokio::test]
    async fn reading_getter_path() {
        let ctx = ctx_from_env().await;
        let one_dashi = r#"[{"pos":"[n]","gloss":"dashi; Japanese soup stock made from fish and kelp","field":"{food}"}]"#;
        let both_dashi = r#"[{"pos":"[n]","gloss":"dashi; Japanese soup stock made from fish and kelp","field":"{food}"},{"pos":"[n]","gloss":"pretext; excuse; pretense (pretence); dupe; front man"}]"#;
        let all_bota = r#"[{"pos":"[adv,adv-to,vs]","gloss":"dripping; trickling; drop by drop; in drops"},{"pos":"[adv,adv-to,vs]","gloss":"wet and heavy (snow, clay, etc.)"},{"pos":"[adv,adv-to]","gloss":"(moving) slowly"}]"#;

        // getter → 出汁: sense 1 filtered
        let reading = kanji_reading(&ctx, 1339160, "出汁").await;
        let getter = std::future::ready(Ok(Some(reading)));
        let result = get_senses_json(&ctx, 1339160, &[], None, Some(getter))
            .await
            .unwrap();
        assert_eq!(json(&result), one_dashi, "getter 出汁");

        // getter → nil: restricted sense passes
        let getter = std::future::ready(Ok(None));
        let result = get_senses_json(&ctx, 1339160, &[], None, Some(getter))
            .await
            .unwrap();
        assert_eq!(json(&result), both_dashi, "getter nil");

        // two stag senses, nil getter: readp-true path on sense 2
        let getter = std::future::ready(Ok(None));
        let result = get_senses_json(&ctx, 1011960, &[], None, Some(getter))
            .await
            .unwrap();
        assert_eq!(json(&result), all_bota, "getter nil, two stag senses");

        // two stag senses, ぼたぼた getter (member of both): memoized reading reused
        let reading = kana_reading(&ctx, 1011960, "ぼたぼた").await;
        let getter = std::future::ready(Ok(Some(reading)));
        let result = get_senses_json(&ctx, 1011960, &[], None, Some(getter))
            .await
            .unwrap();
        assert_eq!(json(&result), all_bota, "getter ぼたぼた, two stag senses");
    }
}

#[cfg(test)]
mod match_kana_kanji_tests {
    use super::*;
    use crate::dict::dao::{KanaText, KanjiText};
    use crate::dict::text_classes::SimpleText;

    fn kana(text: &str, nokanji: bool) -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kana(KanaText {
            id: 0,
            seq: 0,
            text: text.into(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: true,
            nokanji,
            best_kanji: None,
            state: SimpleText::default(),
        })
    }

    fn kanji(text: &str) -> KaniWordDispatchEnum {
        KaniWordDispatchEnum::Kanji(KanjiText {
            id: 0,
            seq: 0,
            text: text.into(),
            ord: 0,
            common: None,
            common_tags: String::new(),
            conjugate_p: true,
            nokanji: false,
            best_kana: None,
            state: SimpleText::default(),
        })
    }

    fn restr(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(r, t)| (r.to_string(), t.to_string()))
            .collect()
    }

    /// REPL fixtures (.103, `ichiran/dict::match-kana-kanji`), 2026-05-24.
    /// Readings are seq-1339160 forms: だし (nokanji nil), ダシ
    /// (nokanji t), 出し / 出汁 kanji.
    #[test]
    fn match_kana_kanji_fixtures() {
        let dashi = kana("だし", false);
        let dashi_kata = kana("ダシ", true);
        let dashi_k = kanji("出し");
        let dashi_kanji = kanji("出汁");

        let cases: &[(
            &KaniWordDispatchEnum,
            &KaniWordDispatchEnum,
            Vec<(String, String)>,
            Option<MatchKanaKanjiResult>,
        )] = &[
            // restricted empty → t
            (
                &dashi,
                &dashi_k,
                restr(&[]),
                Some(MatchKanaKanjiResult::Yes),
            ),
            // restr has だし→出し, kanji is 出し → found
            (
                &dashi,
                &dashi_k,
                restr(&[("だし", "出し")]),
                Some(MatchKanaKanjiResult::Found("出し".into())),
            ),
            // restr has だし→出汁, kanji is 出し → not found
            (&dashi, &dashi_k, restr(&[("だし", "出汁")]), None),
            // restr keyed on ダシ only → filters to empty → t
            (
                &dashi,
                &dashi_k,
                restr(&[("ダシ", "出し")]),
                Some(MatchKanaKanjiResult::Yes),
            ),
            // two rows for だし; kanji 出汁 matches the 2nd → found
            (
                &dashi,
                &dashi_kanji,
                restr(&[("だし", "出し"), ("だし", "出汁")]),
                Some(MatchKanaKanjiResult::Found("出汁".into())),
            ),
            // two rows but kanji 出汁 absent among the だし-keyed ones → not found
            (
                &dashi,
                &dashi_kanji,
                restr(&[("だし", "出し"), ("ダシ", "出汁")]),
                None,
            ),
            // nokanji kana reading → nil regardless of restricted
            (&dashi_kata, &dashi_k, restr(&[("ダシ", "出し")]), None),
            (&dashi_kata, &dashi_k, restr(&[]), None),
        ];
        for (kana_reading, kanji_reading, restricted, expected) in cases {
            assert_eq!(
                &match_kana_kanji(kana_reading, kanji_reading, restricted),
                expected,
                "kana={:?} kanji={:?} restricted={:?}",
                text(kana_reading),
                text(kanji_reading),
                restricted,
            );
        }
    }
}

#[cfg(test)]
mod match_sense_restrictions_tests {
    use super::*;
    use crate::dict::senses::get_senses_raw;
    use std::sync::Arc;

    async fn ctx_from_env() -> Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    async fn props_of(ctx: &KaniranContext, seq: i32, ord: i32) -> Vec<(String, Vec<String>)> {
        get_senses_raw(ctx, seq)
            .await
            .unwrap()
            .into_iter()
            .find(|s| s.ord == ord)
            .expect("sense ord present")
            .props
    }

    async fn reading_of(
        ctx: &KaniranContext,
        seq: i32,
        text: &str,
        kanji: bool,
    ) -> KaniWordDispatchEnum {
        if kanji {
            let row: KanjiText =
                sqlx::query_as("SELECT * FROM kanji_text WHERE seq = $1 AND text = $2")
                    .bind(seq)
                    .bind(text)
                    .fetch_one(&ctx.pool)
                    .await
                    .unwrap();
            KaniWordDispatchEnum::Kanji(row)
        } else {
            let row: KanaText =
                sqlx::query_as("SELECT * FROM kana_text WHERE seq = $1 AND text = $2")
                    .bind(seq)
                    .bind(text)
                    .fetch_one(&ctx.pool)
                    .await
                    .unwrap();
            KaniWordDispatchEnum::Kana(row)
        }
    }

    /// REPL fixtures (.103, `ichiran/dict::match-sense-restrictions`), 2026-05-24.
    /// Covers every cond clause: no-restriction (1339160 s0), direct
    /// member (出し / ダシ / 遇う / ボタボタ / 風太郎), `:kanji`-only nil
    /// (配う), `:kana`-only nil (ポタポタ), the `:kanji` select-dao branch
    /// (出端→t, 出汁→nil via the nokanji ダシ row), and the `:kana`
    /// select-dao branch returning t (だし / あしらう) and the matched
    /// kanji string (ぷうたろう / ふうたろう → "風太郎"; プーたろう → nil;
    /// プータロー nokanji → nil).
    #[tokio::test]
    async fn match_sense_restrictions_fixtures() {
        let ctx = ctx_from_env().await;

        struct Case {
            seq: i32,
            ord: i32,
            text: &'static str,
            kanji: bool,
            expected: Option<MatchKanaKanjiResult>,
        }
        let yes = || Some(MatchKanaKanjiResult::Yes);
        let found = |s: &str| Some(MatchKanaKanjiResult::Found(s.to_string()));
        let cases = [
            // no stags → t (every reading)
            Case {
                seq: 1339160,
                ord: 0,
                text: "出し",
                kanji: true,
                expected: yes(),
            },
            Case {
                seq: 1339160,
                ord: 0,
                text: "だし",
                kanji: false,
                expected: yes(),
            },
            // both stags (stagk 出し, stagr ダシ)
            Case {
                seq: 1339160,
                ord: 1,
                text: "出し",
                kanji: true,
                expected: yes(),
            }, // member stagk
            Case {
                seq: 1339160,
                ord: 1,
                text: "ダシ",
                kanji: false,
                expected: yes(),
            }, // member stagr
            Case {
                seq: 1339160,
                ord: 1,
                text: "出汁",
                kanji: true,
                expected: None,
            }, // :kanji branch, only ダシ(nokanji) matches → nil
            Case {
                seq: 1339160,
                ord: 1,
                text: "だし",
                kanji: false,
                expected: yes(),
            }, // :kana branch → t
            // only stagk (遇う)
            Case {
                seq: 1000300,
                ord: 0,
                text: "遇う",
                kanji: true,
                expected: yes(),
            }, // member stagk
            Case {
                seq: 1000300,
                ord: 0,
                text: "配う",
                kanji: true,
                expected: None,
            }, // :kanji-only ∧ kanji → nil
            Case {
                seq: 1000300,
                ord: 0,
                text: "あしらう",
                kanji: false,
                expected: yes(),
            }, // :kana branch → t
            // only stagr (ボタボタ / ぼたぼた)
            Case {
                seq: 1011960,
                ord: 1,
                text: "ボタボタ",
                kanji: false,
                expected: yes(),
            }, // member stagr
            Case {
                seq: 1011960,
                ord: 1,
                text: "ポタポタ",
                kanji: false,
                expected: None,
            }, // :kana-only ∧ kana → nil
            // both stags, :kanji branch returning t (1580140 stagr でばな non-nokanji)
            Case {
                seq: 1580140,
                ord: 0,
                text: "出端",
                kanji: true,
                expected: yes(),
            },
            // only stagk (風太郎) with restricted_readings rows → Found(string)
            Case {
                seq: 1115120,
                ord: 1,
                text: "風太郎",
                kanji: true,
                expected: yes(),
            }, // member stagk
            Case {
                seq: 1115120,
                ord: 1,
                text: "プー太郎",
                kanji: true,
                expected: None,
            }, // :kanji-only ∧ kanji → nil
            Case {
                seq: 1115120,
                ord: 1,
                text: "プータロー",
                kanji: false,
                expected: None,
            }, // nokanji kana → match-kana-kanji nil
            Case {
                seq: 1115120,
                ord: 1,
                text: "ぷうたろう",
                kanji: false,
                expected: found("風太郎"),
            },
            Case {
                seq: 1115120,
                ord: 1,
                text: "ふうたろう",
                kanji: false,
                expected: found("風太郎"),
            },
            Case {
                seq: 1115120,
                ord: 1,
                text: "プーたろう",
                kanji: false,
                expected: None,
            }, // restr keyed プー太郎, 風太郎 absent → nil
        ];

        for case in &cases {
            let props = props_of(&ctx, case.seq, case.ord).await;
            let reading = reading_of(&ctx, case.seq, case.text, case.kanji).await;
            let actual = match_sense_restrictions(&ctx, case.seq, &props, &reading)
                .await
                .unwrap();
            assert_eq!(
                actual, case.expected,
                "seq={} ord={} text={}",
                case.seq, case.ord, case.text,
            );
        }
    }
}

#[cfg(test)]
mod split_pos_tests {
    use super::*;

    /// REPL fixtures (.103, `ichiran/dict::split-pos`), 2026-05-24.
    #[test]
    fn split_pos_fixtures() {
        let cases: &[(&str, Vec<&str>)] = &[
            ("[n,adj-no]", vec!["n", "adj-no"]),
            ("[]", vec![""]),
            ("[n]", vec!["n"]),
            ("[adv,adv-to,vs]", vec!["adv", "adv-to", "vs"]),
            ("[ctr]", vec!["ctr"]),
            ("[v5u,vt]", vec!["v5u", "vt"]),
        ];
        for (pos_str, expected) in cases {
            assert_eq!(&split_pos(pos_str), expected, "pos_str={pos_str:?}");
        }
    }
}

#[cfg(test)]
mod short_sense_str_tests {
    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    /// REPL fixtures (.103, `ichiran/dict::short-sense-str`), 2026-05-24.
    /// Covers no with-pos (first sense's glosses), with-pos matching a
    /// sense's pos, with-pos with no matching sense (→ nil), and an
    /// unknown seq (→ nil), across a noun entry (1582710 日本) and a
    /// verb entry (1358280 食べる, pos v1).
    #[tokio::test]
    async fn short_sense_str_fixtures() {
        let ctx = ctx_from_env().await;
        let cases: &[(i32, Option<&str>, Option<&str>)] = &[
            (1582710, None, Some("Japan")),
            (1358280, None, Some("to eat")),
            (1358280, Some("v1"), Some("to eat")),
            (1358280, Some("n"), None),
            (1582710, Some("v1"), None),
            (1582710, Some("n"), Some("Japan")),
            (999999, None, None),
            (999999, Some("v1"), None),
        ];
        for (seq, with_pos, expected) in cases {
            assert_eq!(
                short_sense_str(&ctx, *seq, *with_pos)
                    .await
                    .unwrap()
                    .as_deref(),
                *expected,
                "seq={seq} with_pos={with_pos:?}"
            );
        }
    }
}

#[cfg(test)]
mod reading_str_tests {
    use super::*;
    use crate::dict::find_word::{find_word, FindWordRows};

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    async fn first_simple_text(ctx: &KaniranContext, word: &str) -> KaniSimpleTextDispatchEnum {
        match find_word(ctx, word, false).await.unwrap() {
            FindWordRows::Kanji(rows) => KaniSimpleTextDispatchEnum::Kanji(
                rows.into_iter()
                    .next()
                    .expect("at least one kanji-text row"),
            ),
            FindWordRows::Kana(rows) => KaniSimpleTextDispatchEnum::Kana(
                rows.into_iter().next().expect("at least one kana-text row"),
            ),
        }
    }

    /// REPL fixtures (.103, `(reading-str (car (find-word W)))`), 2026-05-24.
    /// Covers the simple-text method: kanji-text (kanji+kana), kana-text with
    /// a kanji form (get-kanji via best-kanji-conj → 猫), and kana-text without
    /// one (get-kanji nil → bare kana).
    #[tokio::test]
    async fn reading_str_simple_text_fixtures() {
        let ctx = ctx_from_env().await;
        let cases: &[(&str, &str)] = &[
            ("日本", "日本 【にほん】"),
            ("ねこ", "猫 【ねこ】"),
            ("ぴちぴち", "ぴちぴち"),
            ("食べる", "食べる 【たべる】"),
            ("東京", "東京 【とうきょう】"),
        ];
        for (word, expected) in cases {
            let obj = first_simple_text(&ctx, word).await;
            assert_eq!(
                obj.reading_str(&ctx).await.unwrap().as_deref(),
                Some(*expected),
                "word={word}"
            );
        }
    }

    /// REPL fixtures (.103, `(reading-str (car (simple-segment S)))`), 2026-05-24.
    /// Covers the word-info method on real segmented (kanji-type) word-infos.
    #[tokio::test]
    async fn reading_str_word_info_fixtures() {
        let ctx = ctx_from_env().await;
        let cases: &[(&str, &str)] = &[
            ("東京", "東京 【とうきょう】"),
            ("日本語", "日本語 【にほんご】"),
        ];
        for (sentence, expected) in cases {
            let word_infos = crate::dict::segment::simple_segment(&ctx, sentence, None)
                .await
                .unwrap();
            assert_eq!(
                word_infos[0].reading_str().as_deref(),
                Some(*expected),
                "sentence={sentence}"
            );
        }
    }
}

#[cfg(test)]
mod reading_str_seq_tests {
    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    /// REPL fixtures (.103, `ichiran/dict::reading-str-seq`), 2026-05-24.
    /// Covers a kanji+kana entry, a conjugating kanji+kana verb, two
    /// kana-only entries (no kanji-text row → kanji nil → bare kana),
    /// and an unknown seq (both column queries empty → nil).
    #[tokio::test]
    async fn reading_str_seq_fixtures() {
        let ctx = ctx_from_env().await;
        let cases: &[(i32, Option<&str>)] = &[
            (1582710, Some("日本 【にほん】")),
            (1358280, Some("食べる 【たべる】")),
            (1010890, Some("ぴちぴち")),
            (1056070, Some("サイバネーション")),
            (999999, None),
        ];
        for (seq, expected) in cases {
            assert_eq!(
                reading_str_seq(&ctx, *seq).await.unwrap().as_deref(),
                *expected,
                "seq={seq}"
            );
        }
    }
}

#[cfg(test)]
mod reading_str_star_tests {
    use super::*;

    /// REPL fixtures (.103, `ichiran/dict::reading-str*`), 2026-05-24.
    /// Covers kanji+kana, kana-only (kanji nil → bare kana), kanji with
    /// nil kana (`~a` of nil → "NIL"), and both nil → nil.
    #[test]
    fn reading_str_star_fixtures() {
        let cases: &[(Option<&str>, Option<&str>, Option<&str>)] = &[
            (Some("日本"), Some("にほん"), Some("日本 【にほん】")),
            (None, Some("ねこ"), Some("ねこ")),
            (Some("猫"), None, Some("猫 【NIL】")),
            (None, None, None),
        ];
        for (kanji, kana, expected) in cases {
            assert_eq!(
                reading_str_star_(*kanji, *kana).as_deref(),
                *expected,
                "kanji={kanji:?} kana={kana:?}"
            );
        }
    }
}

#[cfg(test)]
mod entry_info_short_tests {
    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    /// REPL fixtures (.103, `ichiran/dict::entry-info-short`), 2026-05-24.
    /// Covers a noun (1582710 日本) and a verb (1358280 食べる) with no
    /// with-pos, with-pos matching the entry's pos, and with-pos that
    /// matches no sense (→ sense-str nil → trailing nothing after " : "),
    /// plus an unknown seq (reading nil → "NIL").
    #[tokio::test]
    async fn entry_info_short_fixtures() {
        let ctx = ctx_from_env().await;
        let cases: &[(i32, Option<&str>, &str)] = &[
            (1582710, None, "日本 【にほん】 : Japan"),
            (1358280, None, "食べる 【たべる】 : to eat"),
            (1358280, Some("v1"), "食べる 【たべる】 : to eat"),
            (1358280, Some("n"), "食べる 【たべる】 : "),
            (1582710, Some("v1"), "日本 【にほん】 : "),
            (999999, None, "NIL : "),
            (999999, Some("v1"), "NIL : "),
        ];
        for (seq, with_pos, expected) in cases {
            assert_eq!(
                entry_info_short(&ctx, *seq, *with_pos).await.unwrap(),
                *expected,
                "seq={seq} with_pos={with_pos:?}"
            );
        }
    }
}

#[cfg(test)]
mod entry_info_long_tests {
    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    /// REPL fixtures (.103, `ichiran/dict::entry-info-long`), 2026-05-25.
    /// Covers a single-sense noun (reading with 【kanji】), a verb,
    /// a kana-only entry (reading-str* returns bare kana, no 【】),
    /// a multi-sense entry exercising the `《s_inf》` annotation and the
    /// `get-senses-raw` last-tag-group reversal (sense 4 pos), and an
    /// unknown seq (reading nil + empty senses → just the seq, the
    /// `~@[` nil branch).
    #[tokio::test]
    async fn entry_info_long_fixtures() {
        let ctx = ctx_from_env().await;
        let cases: &[(i32, &str)] = &[
            (1562520, "1562520 賄賂 【わいろ】\n1. [n] bribe; sweetener; douceur"),
            (1573390, "1573390 躊躇う 【ためらう】\n1. [vi,v5u] to hesitate; to waver"),
            (1087690, "1087690 ドーナツ\n1. [n] doughnut; donut"),
            (
                1010900,
                "1010900 ぴったり\n1. [adv,adv-to,vs] 《ぴったし is colloquial》 tightly; closely\n2. [adv,adv-to,vs] exactly; precisely\n3. [adv,adv-to,vs] suddenly (stopping)\n4. [adj-na,vs,adv-to,adv] perfectly (suited); ideally",
            ),
            (999999, "999999"),
        ];
        for (seq, expected) in cases {
            assert_eq!(
                &entry_info_long(&ctx, *seq).await.unwrap(),
                expected,
                "seq={seq}"
            );
        }
    }
}

#[cfg(test)]
mod get_glosses_tests {
    //! Unit tests against the real .103 PG via `KaniranContext::from_env()`.
    //! REPL probe pinned 2026-05-22 using `(get-glosses ...)` after
    //! `(ichiran/conn:with-db nil ...)`. Verifies the upstream
    //! reverse-physical-row inner ordering, multi-seq outer ordering,
    //! and empty / no-rows edge cases.
    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    /// REPL: `(get-glosses '(1372640 1577100))` → outer order is
    /// ascending sense.seq; inner order is reverse physical-row.
    #[tokio::test]
    async fn multi_seq_grouping_and_inner_reversal() {
        let ctx = ctx_from_env().await;
        let out = get_glosses(&ctx, &[1372640, 1577100]).await.unwrap();
        assert_eq!(
            out,
            vec![
                (
                    1372640,
                    vec!["execution".to_string(), "accomplishment".to_string()]
                ),
                (
                    1577100,
                    vec![
                        "oh (certainly not)".to_string(),
                        "why (it's nothing)".to_string(),
                        "oh, no (it's fine)".to_string(),
                        "come on!".to_string(),
                        "hey!".to_string(),
                        "huh?".to_string(),
                        "what?".to_string(),
                        "(not) in the slightest".to_string(),
                        "(not) at all".to_string(),
                        "dick".to_string(),
                        "(one's) thing".to_string(),
                        "penis".to_string(),
                        "what's-her-name".to_string(),
                        "what's-his-name".to_string(),
                        "whachamacallit".to_string(),
                        "whatsit".to_string(),
                        "that thing".to_string(),
                        "you-know-what".to_string(),
                        "what".to_string(),
                    ],
                ),
            ],
        );
    }

    /// REPL: `(get-glosses nil)` → `NIL`.
    #[tokio::test]
    async fn empty_seqs_returns_empty() {
        let ctx = ctx_from_env().await;
        let out = get_glosses(&ctx, &[]).await.unwrap();
        assert!(out.is_empty());
    }

    /// REPL: `(get-glosses '(9999999))` → single-row JMdict header
    /// gloss attached to the placeholder seq.
    #[tokio::test]
    async fn unknown_seq_returns_header_row() {
        let ctx = ctx_from_env().await;
        let out = get_glosses(&ctx, &[9999999]).await.unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].0, 9999999);
        assert_eq!(out[0].1.len(), 1);
        assert!(out[0].1[0].starts_with("Japanese-Multilingual Dictionary Project"));
    }
}

#[cfg(test)]
mod match_glosses_tests {
    //! Unit tests against the real .103 PG via `KaniranContext::from_env()`.
    //! REPL probe pinned 2026-05-22 against `(match-glosses ...)` after
    //! `(ichiran/conn:with-db nil ...)`. Coverage:
    //!   `("Chinese" "character")` → words-match arm `(Seq, true)`.
    //!   `("chinese" "character")` no-normalize → fallback `(Seq, false)`.
    //!   `("chinese" "character")` + `string-downcase` → words-match `(Seq, true)`.
    //!   `("zzzz")` + update-gloss `^(?i)chinese character` → `(SeqAndGloss, true)`.
    //!   `("zzzz")` + update-gloss that misses + words that miss → fallback `(Seq, false)`.
    //!   bogus reading → `None`.
    //!   empty words → matches the first DB-order gloss (`all` on empty is vacuously true).
    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    /// REPL: `(match-glosses "漢字" "かんじ" '("Chinese" "character"))`
    /// → `matched=1213170 found-p=T`.
    #[tokio::test]
    async fn words_match_returns_seq_true() {
        let ctx = ctx_from_env().await;
        let out = match_glosses(
            &ctx,
            "漢字",
            Some("かんじ"),
            &["Chinese", "character"],
            None,
            None,
        )
        .await
        .unwrap();
        assert_eq!(out, Some((MatchValue::Seq(1213170), true)));
    }

    /// REPL: `(match-glosses "漢字" "かんじ" '("zzzzz"))`
    /// → `matched=1213170 found-p=NIL` — fallback to first candidate.
    #[tokio::test]
    async fn no_word_match_fallback_to_first_candidate() {
        let ctx = ctx_from_env().await;
        let out = match_glosses(&ctx, "漢字", Some("かんじ"), &["zzzzz"], None, None)
            .await
            .unwrap();
        assert_eq!(out, Some((MatchValue::Seq(1213170), false)));
    }

    /// REPL: `(match-glosses "漢字" "ZZZZZ" '("x"))`
    /// → `matched=NIL found-p=NIL` — no candidates from `(text, reading)`.
    #[tokio::test]
    async fn empty_candidates_returns_none() {
        let ctx = ctx_from_env().await;
        let out = match_glosses(&ctx, "漢字", Some("ZZZZZ"), &["x"], None, None)
            .await
            .unwrap();
        assert_eq!(out, None);
    }

    /// REPL: `(match-glosses "漢字" "かんじ" '("zzzz")
    ///         :update-gloss '(:sequence :case-insensitive-p :start-anchor "chinese character"))`
    /// → `matched=(1213170 "Chinese character") found-p=T`.
    #[tokio::test]
    async fn update_gloss_match_returns_seq_and_gloss() {
        let ctx = ctx_from_env().await;
        let rg = fancy_regex::Regex::new("(?i)^chinese character").unwrap();
        let out = match_glosses(&ctx, "漢字", Some("かんじ"), &["zzzz"], None, Some(&rg))
            .await
            .unwrap();
        assert_eq!(
            out,
            Some((
                MatchValue::SeqAndGloss(1213170, "Chinese character".to_string()),
                true
            )),
        );
    }

    /// REPL: `(match-glosses "漢字" "かんじ" '("zzzz")
    ///         :update-gloss '(:sequence "XXXX"))`
    /// → `matched=1213170 found-p=NIL` — neither update-gloss nor words match.
    #[tokio::test]
    async fn update_gloss_miss_and_words_miss_fallback() {
        let ctx = ctx_from_env().await;
        let rg = fancy_regex::Regex::new("XXXX").unwrap();
        let out = match_glosses(&ctx, "漢字", Some("かんじ"), &["zzzz"], None, Some(&rg))
            .await
            .unwrap();
        assert_eq!(out, Some((MatchValue::Seq(1213170), false)));
    }

    /// REPL: `(match-glosses "漢字" "かんじ" '("chinese" "character") :normalize 'string-downcase)`
    /// → `matched=1213170 found-p=T` — words become matchable after normalize.
    #[tokio::test]
    async fn normalize_string_downcase_enables_match() {
        let ctx = ctx_from_env().await;
        let downcase: &dyn Fn(&str) -> String = &|s: &str| s.to_lowercase();
        let out = match_glosses(
            &ctx,
            "漢字",
            Some("かんじ"),
            &["chinese", "character"],
            Some(downcase),
            None,
        )
        .await
        .unwrap();
        assert_eq!(out, Some((MatchValue::Seq(1213170), true)));
    }

    /// REPL: same as above but no `:normalize` — case-sensitive
    /// `search` doesn't find lowercase `chinese` in `Chinese character`.
    /// → `matched=1213170 found-p=NIL`.
    #[tokio::test]
    async fn no_normalize_lowercase_falls_back() {
        let ctx = ctx_from_env().await;
        let out = match_glosses(
            &ctx,
            "漢字",
            Some("かんじ"),
            &["chinese", "character"],
            None,
            None,
        )
        .await
        .unwrap();
        assert_eq!(out, Some((MatchValue::Seq(1213170), false)));
    }

    /// REPL: `(match-glosses "漢字" "かんじ" '())`
    /// → `matched=1213170 found-p=T` — empty words is vacuously
    /// satisfied by the first DB-order gloss.
    #[tokio::test]
    async fn empty_words_matches_first_gloss() {
        let ctx = ctx_from_env().await;
        let out = match_glosses(&ctx, "漢字", Some("かんじ"), &[], None, None)
            .await
            .unwrap();
        assert_eq!(out, Some((MatchValue::Seq(1213170), true)));
    }
}

#[cfg(test)]
mod is_rareru_tests {
    use super::*;

    /// REPL fixtures (.103, `ichiran/dict::is-rareru`), 2026-05-24.
    /// Covers each of the four suffixes, a non-rareru form, the empty
    /// string, a rareru substring not at the end (`られるよ` → false), and
    /// a kana-only suffix.
    #[test]
    fn is_rareru_fixtures() {
        let cases: &[(&str, bool)] = &[
            ("食べられる", true),
            ("食べられます", true),
            ("食べられない", true),
            ("食べられません", true),
            ("食べる", false),
            ("", false),
            ("られるよ", false),
            ("られる", true),
        ];
        for (text, expected) in cases {
            assert_eq!(is_rareru(text), *expected, "text={text:?}");
        }
    }
}
