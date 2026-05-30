//! Port of the dict-counters.lisp lookup + numeric-formatting layer —
//! `find-counter` and its three helper accessors (`get-counter-ids`,
//! `get-counter-stags`, `get-counter-readings`) plus `nokanji` /
//! `ord` / `ordinal-str` / `get-digit`. Used by callers materializing
//! a [`super::classes::CounterText`] from a digit string.


use crate::conn::kani_context::KaniranContext;
use crate::dict::counters::cache::{EXTRA_COUNTER_IDS, SKIP_COUNTER_IDS, counter_cache};
use crate::dict::counters::classes::{Counter, CounterSource};
use crate::dict::counters::dispatchers::verify;
use crate::dict::dao::KanaText;
use crate::dict::kani::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use crate::dict::dao::KanjiText;
use sqlx::Row;
use std::collections::{HashMap, HashSet};

// ----- was find_counter.rs -----

pub fn find_counter(
    ctx: &KaniranContext,
    number: &str,
    counter: &str,
    unique: Option<bool>,
) -> Vec<Counter> {
    // dict-counters.lisp:273 — `&key (unique t)`. `None` here means
    // "caller didn't supply :UNIQUE", which Lisp resolves to `t`.
    let unique = unique.unwrap_or(true);
    let Some(args_list) = counter_cache(ctx).get(counter) else {
        return Vec::new();
    };
    let mut out = Vec::with_capacity(args_list.len());
    for args in args_list {
        match Counter::new(args, number) {
            Ok(c) if verify(&c, unique) => out.push(c),
            _ => {}
        }
    }
    out
}


// ----- was get_counter_ids.rs -----

pub async fn get_counter_ids(ctx: &KaniranContext) -> Result<Vec<i32>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT DISTINCT seq FROM sense_prop WHERE tag = 'pos' AND text = 'ctr'",
    )
    .fetch_all(&ctx.pool)
    .await?;
    let mut seqs: Vec<i32> = rows.into_iter().map(|r| r.get::<i32, _>("seq")).collect();
    seqs.sort();
    Ok(seqs)
}


// ----- was get_counter_stags.rs -----

pub type CounterStags = (HashMap<i32, Vec<String>>, HashMap<i32, Vec<String>>);

pub async fn get_counter_stags(
    ctx: &KaniranContext,
    seqs: &[i32],
) -> Result<CounterStags, sqlx::Error> {
    let mut stagks: HashMap<i32, Vec<String>> = HashMap::new();
    let mut stagrs: HashMap<i32, Vec<String>> = HashMap::new();

    let sql = "SELECT sp.seq, sp.text \
               FROM sense_prop sp, sense_prop sp1 \
               WHERE sp.seq = sp1.seq \
                 AND sp.sense_id = sp1.sense_id \
                 AND sp.tag = $1 \
                 AND sp1.tag = 'pos' \
                 AND sp1.text = 'ctr' \
                 AND sp.seq = ANY($2)";

    for row in sqlx::query(sql)
        .bind("stagk")
        .bind(seqs)
        .fetch_all(&ctx.pool)
        .await?
    {
        let seq: i32 = row.get("seq");
        let text: String = row.get("text");
        stagks.entry(seq).or_default().push(text);
    }

    for row in sqlx::query(sql)
        .bind("stagr")
        .bind(seqs)
        .fetch_all(&ctx.pool)
        .await?
    {
        let seq: i32 = row.get("seq");
        let text: String = row.get("text");
        stagrs.entry(seq).or_default().push(text);
    }

    Ok((stagks, stagrs))
}


// ----- was get_counter_readings.rs -----

pub type CounterReadings = HashMap<i32, (Vec<KanjiText>, Vec<KanaText>)>;

pub async fn get_counter_readings(ctx: &KaniranContext) -> Result<CounterReadings, sqlx::Error> {
    let mut counter_ids: Vec<i32> = get_counter_ids(ctx).await?;
    counter_ids.extend(EXTRA_COUNTER_IDS.iter().copied());
    let skip: HashSet<i32> = SKIP_COUNTER_IDS.iter().copied().collect();
    counter_ids.retain(|id| !skip.contains(id));

    let stags = get_counter_stags(ctx, &counter_ids).await?;

    let kanji_readings: Vec<KanjiText> = sqlx::query_as::<_, KanjiText>(
        "SELECT * FROM kanji_text WHERE seq = ANY($1)",
    )
    .bind(&counter_ids)
    .fetch_all(&ctx.pool)
    .await?;

    let kana_readings: Vec<KanaText> = sqlx::query_as::<_, KanaText>(
        "SELECT * FROM kana_text WHERE seq = ANY($1)",
    )
    .bind(&counter_ids)
    .fetch_all(&ctx.pool)
    .await?;

    let mut hash: CounterReadings = HashMap::new();

    for r in kanji_readings {
        let stagks = stags.0.get(&r.seq);
        let admit = match stagks {
            None => true,
            Some(list) => list.iter().any(|t| t == &r.text),
        };
        if admit {
            hash.entry(r.seq).or_insert_with(|| (Vec::new(), Vec::new())).0.push(r);
        }
    }

    for r in kana_readings {
        let stagrs = stags.1.get(&r.seq);
        let admit = match stagrs {
            None => true,
            Some(list) => list.iter().any(|t| t == &r.text),
        };
        if admit {
            hash.entry(r.seq).or_insert_with(|| (Vec::new(), Vec::new())).1.push(r);
        }
    }

    for (_, (kanji, kana)) in hash.iter_mut() {
        kanji.sort_by_key(|r| r.ord);
        kana.sort_by_key(|r| r.ord);
    }

    Ok(hash)
}


// ----- was nokanji.rs -----

pub fn nokanji(obj: &KaniWordDispatchEnum) -> Option<bool> {
    match obj {
        KaniWordDispatchEnum::Kanji(k) => Some(k.nokanji),
        KaniWordDispatchEnum::Kana(k) => Some(k.nokanji),
        KaniWordDispatchEnum::Proxy(p) => Some(nokanji_simple(&p.source)),
        KaniWordDispatchEnum::Counter(c) => Some(nokanji_counter(c)),
        KaniWordDispatchEnum::Compound(_) => None,
    }
}

fn nokanji_simple(obj: &KaniSimpleTextDispatchEnum) -> bool {
    match obj {
        KaniSimpleTextDispatchEnum::Kanji(k) => k.nokanji,
        KaniSimpleTextDispatchEnum::Kana(k) => k.nokanji,
        KaniSimpleTextDispatchEnum::Proxy(p) => nokanji_simple(&p.source),
    }
}

fn nokanji_counter(c: &Counter) -> bool {
    // dict-counters.lisp:89-90 — `(and (source obj) (nokanji (source obj)))`
    match c.base().source.as_ref() {
        None => false,
        Some(CounterSource::Kanji(k)) => k.nokanji,
        Some(CounterSource::Kana(k)) => k.nokanji,
    }
}

#[cfg(test)]
mod test_nokanji {
    use super::*;
    use crate::dict::counters::classes::{Common, Counter, CounterSource, CounterText};
    use crate::dict::dao::KanaText;
    use crate::dict::dao::KanjiText;
    use crate::dict::text_classes::ProxyText;
    use crate::dict::text_classes::SimpleText;

    fn kanji_with(nokanji: bool) -> KanjiText {
        KanjiText {
            id: 0, seq: 0, text: String::new(), ord: 0,
            common: None, common_tags: String::new(), conjugate_p: true,
            nokanji, best_kana: None, state: SimpleText::default(),
        }
    }

    fn kana_with(nokanji: bool) -> KanaText {
        KanaText {
            id: 0, seq: 0, text: String::new(), ord: 0,
            common: None, common_tags: String::new(), conjugate_p: true,
            nokanji, best_kanji: None, state: SimpleText::default(),
        }
    }

    fn counter_with_source(source: Option<CounterSource>) -> Counter {
        Counter::Base(CounterText {
            text: String::new(), kana: String::new(),
            number_text: "0".into(), number: 0,
            source, ordinalp: false, suffix: None,
            accepts_suffixes: Vec::new(), suffix_descriptions: Vec::new(),
            digit_opts: Vec::new(), common: Common::Inherit,
            allowed: Vec::new(), foreign: false,
        })
    }

    #[test]
    fn proxy_recurses_through_source_chain() {
        let leaf = KaniSimpleTextDispatchEnum::Kana(kana_with(true));
        let inner = KaniSimpleTextDispatchEnum::Proxy(ProxyText {
            text: String::new(), kana: String::new(),
            source: Box::new(leaf), state: SimpleText::default(),
        });
        let outer = ProxyText {
            text: String::new(), kana: String::new(),
            source: Box::new(inner), state: SimpleText::default(),
        };
        assert_eq!(
            nokanji(&KaniWordDispatchEnum::Proxy(outer)),
            Some(true),
        );
    }

    #[test]
    fn counter_without_source_is_false() {
        let c = counter_with_source(None);
        assert_eq!(nokanji(&KaniWordDispatchEnum::Counter(c)), Some(false));
    }

    #[test]
    fn counter_with_kana_source_propagates_flag() {
        let c = counter_with_source(Some(CounterSource::Kana(kana_with(true))));
        assert_eq!(nokanji(&KaniWordDispatchEnum::Counter(c)), Some(true));
    }

    #[test]
    fn counter_with_kanji_source_propagates_flag() {
        let c = counter_with_source(Some(CounterSource::Kanji(kanji_with(false))));
        assert_eq!(nokanji(&KaniWordDispatchEnum::Counter(c)), Some(false));
    }
}


// ----- was ord.rs -----

fn ord_simple(obj: &KaniSimpleTextDispatchEnum) -> i32 {
    match obj {
        KaniSimpleTextDispatchEnum::Kanji(k) => k.ord,
        KaniSimpleTextDispatchEnum::Kana(k) => k.ord,
        KaniSimpleTextDispatchEnum::Proxy(p) => ord_simple(&p.source),
    }
}

pub fn ord(obj: &KaniWordDispatchEnum) -> i32 {
    match obj {
        KaniWordDispatchEnum::Kanji(k) => k.ord,
        KaniWordDispatchEnum::Kana(k) => k.ord,
        KaniWordDispatchEnum::Proxy(p) => ord_simple(&p.source),
        KaniWordDispatchEnum::Compound(c) => ord(&c.primary),
        KaniWordDispatchEnum::Counter(c) => match &c.base().source {
            None => 0,
            Some(CounterSource::Kanji(k)) => k.ord,
            Some(CounterSource::Kana(k)) => k.ord,
        },
    }
}


// ----- was ordinal_str.rs -----
pub fn ordinal_str(n: i64) -> String {
    let digit = n.rem_euclid(10);
    let teen = (11..=19).contains(&n.rem_euclid(100));
    let suffix = if teen {
        "th"
    } else {
        match digit {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        }
    };
    format!("{}{}", n, suffix)
}

#[cfg(test)]
mod test_ordinal_str {
    //! Unit tests cover the three teen-band edges plus a sample of
    //! each digit-suffix case. Bulk behavioural coverage lives in
    //! `corpus/extracted_counter_2026_05_08/dict/ordinal_str.parquet`
    //! (134 rows) replayed by `audit_fixtures`.
    use super::*;

    #[test]
    fn small_digits_select_st_nd_rd_th() {
        assert_eq!(ordinal_str(1), "1st");
        assert_eq!(ordinal_str(2), "2nd");
        assert_eq!(ordinal_str(3), "3rd");
        assert_eq!(ordinal_str(4), "4th");
        assert_eq!(ordinal_str(7), "7th");
        assert_eq!(ordinal_str(0), "0th");
    }

    #[test]
    fn teens_force_th() {
        // (mod n 100) ∈ 11..=19 → "th" regardless of last digit.
        assert_eq!(ordinal_str(11), "11th");
        assert_eq!(ordinal_str(12), "12th");
        assert_eq!(ordinal_str(13), "13th");
        assert_eq!(ordinal_str(19), "19th");
        assert_eq!(ordinal_str(111), "111th");
        assert_eq!(ordinal_str(212), "212th");
    }

    #[test]
    fn non_teen_twos_threes_etc_use_digit_suffix() {
        assert_eq!(ordinal_str(21), "21st");
        assert_eq!(ordinal_str(22), "22nd");
        assert_eq!(ordinal_str(23), "23rd");
        assert_eq!(ordinal_str(101), "101st");
        assert_eq!(ordinal_str(122), "122nd");
        assert_eq!(ordinal_str(1000), "1000th");
    }

    #[test]
    fn negative_uses_floor_mod() {
        // Lisp (mod -1 10) = 9, so digit "9" → "th". Format prints "-1".
        assert_eq!(ordinal_str(-1), "-1th");
        // (mod -21 10) = 9; (mod -21 100) = 79 (not in 11..=19) → "th".
        assert_eq!(ordinal_str(-21), "-21th");
    }
}


// ----- was get_digit.rs -----
pub fn get_digit(n: i64) -> Option<i64> {
    let digit = n % 10;
    if digit != 0 {
        return Some(digit);
    }
    for &(p, pn) in &[
        (10_i64, 100_i64),
        (100, 1_000),
        (1_000, 10_000),
        (10_000, 100_000_000),
    ] {
        if n % pn != 0 {
            return Some(p);
        }
    }
    None
}
