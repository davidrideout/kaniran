//! Reading retrieval: per-character lookup with caching, plus
//! geminate/rendaku expansion into the variant set the matcher
//! consumes. From `kanji.lisp:199-238`.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use crate::characters::voicing::{geminate, rendaku as voice_rendaku, Voicing};
use crate::conn::kani_context::KaniranContext;

/// Single typed shape for the heterogeneous reading lists that flow
/// through [`get_normal_readings`], [`super::matching::make_rmap`],
/// and [`super::matching::match_readings_star`]. Upstream is a 2-/3-/
/// 4-element cons list: `(reading type)`, `(reading type tag)`, or
/// `(reading type tag gem)`. `tag = None` flags the bare 2-elt form;
/// `tag = Some(_)` covers explicit-NIL and explicit-`:rendaku`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KanjiReading {
    pub reading: String,
    pub r#type: String,
    pub tag: Option<ReadingTag>,
    pub gem: Option<String>,
}

impl From<ReadingAlternative> for KanjiReading {
    fn from(a: ReadingAlternative) -> Self {
        Self {
            reading: a.reading,
            r#type: a.r#type,
            tag: Some(a.tag),
            gem: a.gem,
        }
    }
}

/// `*reading-cache*` (`kanji.lisp:199`). Per-key lazy cache for
/// [`get_readings_cache`]; owned by `KaniranContext`. Last-write-wins
/// on concurrent misses, mirroring the Lisp
/// `(setf (gethash key *reading-cache*) result)` semantics.
pub type ReadingCache = Mutex<HashMap<(String, Vec<String>), Vec<(String, String)>>>;

pub fn new_reading_cache() -> ReadingCache {
    Mutex::new(HashMap::new())
}

pub fn reading_cache(ctx: &KaniranContext) -> &ReadingCache {
    &ctx.reading_cache
}

/// `get-readings-cache` (`kanji.lisp:201`). Readings of `text` whose
/// `type` is not in `typeset`, cached on `(text, typeset)`. Empty
/// `typeset` returns no rows — upstream's `r.type NOT IN (NULL)`
/// expands to UNKNOWN for every row. The Rust port short-circuits
/// the SQL and still records the empty entry.
pub async fn get_readings_cache(
    ctx: &KaniranContext,
    text: &str,
    typeset: &[String],
) -> Result<Vec<(String, String)>, sqlx::Error> {
    let key = (text.to_string(), typeset.to_vec());
    {
        let cache = ctx.reading_cache.lock().unwrap();
        if let Some(val) = cache.get(&key) {
            return Ok(val.clone());
        }
    }
    let result: Vec<(String, String)> = if typeset.is_empty() {
        Vec::new()
    } else {
        // kanji.lisp:206 ((:select 'r.text 'r.type :from (:as 'kanji 'k) ...))
        sqlx::query_as::<_, (String, String)>(
            "SELECT r.text, r.type FROM kanji k \
             INNER JOIN reading r ON r.kanji_id = k.id \
             WHERE k.text = $1 AND r.type <> ALL($2)",
        )
        .bind(text)
        .bind(typeset)
        .fetch_all(&ctx.pool)
        .await?
    };
    {
        let mut cache = ctx.reading_cache.lock().unwrap();
        cache.insert(key, result.clone());
    }
    Ok(result)
}

/// `get-readings` (`kanji.lisp:213`). Kanjidic2 readings of `char`,
/// defaulting to everything except `ja_na`. With `names = true` the
/// typeset is empty and the cached query yields `[]`, matching the
/// upstream REPL capture `(get-readings #\日 :names t) ⇒ NIL`.
pub async fn get_readings(
    ctx: &KaniranContext,
    char: char,
    names: bool,
) -> Result<Vec<(String, String)>, sqlx::Error> {
    let str: String = char.into();
    let typeset: Vec<String> = if names {
        Vec::new()
    } else {
        vec!["ja_na".to_string()]
    };
    get_readings_cache(ctx, &str, &typeset).await
}

/// `:rendaku` keyword on [`get_reading_alternatives`] outputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadingTag {
    Plain,
    Rendaku,
}

/// One row in the candidate set produced by [`get_reading_alternatives`].
/// Upstream emits a 3-/4-element cons list `(rd type tag gem?)` where
/// `tag` is `nil` (base/geminate) or `:rendaku`; the Rust port names
/// the positions so consumers don't destructure positionally.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadingAlternative {
    pub reading: String,
    pub r#type: String,
    pub tag: ReadingTag,
    pub gem: Option<String>,
}

/// `get-reading-alternatives` (`kanji.lisp:218`). Expand `(reading,
/// type)` into the variant set: always the base entry; a geminated
/// (`っ`) entry when `type = "ja_on"` and the last character is
/// つ/く/き/ち; and with `rendaku`, every prior entry duplicated as
/// dakuten + handakuten voicings.
///
/// Geminate/rendaku take `&mut String`; `:fresh t` semantics at every
/// callsite are reproduced by cloning before the call.
pub fn get_reading_alternatives(
    reading: &str,
    r#type: &str,
    rendaku: bool,
) -> Vec<ReadingAlternative> {
    let chars: Vec<char> = reading.chars().collect();
    let mut lst: Vec<ReadingAlternative> = vec![ReadingAlternative {
        reading: reading.to_string(),
        r#type: r#type.to_string(),
        tag: ReadingTag::Plain,
        gem: None,
    }];

    if chars.len() > 1 && r#type == "ja_on" {
        let last = chars[chars.len() - 1];
        if matches!(last, 'つ' | 'く' | 'き' | 'ち') {
            let mut copy = reading.to_string();
            geminate(&mut copy);
            lst.push(ReadingAlternative {
                reading: copy,
                r#type: r#type.to_string(),
                tag: ReadingTag::Plain,
                gem: Some(last.to_string()),
            });
        }
    }

    if rendaku {
        let snapshot = lst.clone();
        for entry in &snapshot {
            let mut rd = entry.reading.clone();
            voice_rendaku(&mut rd, Voicing::Dakuten);
            lst.push(ReadingAlternative {
                reading: rd,
                r#type: r#type.to_string(),
                tag: ReadingTag::Rendaku,
                gem: entry.gem.clone(),
            });
            let mut rd_h = entry.reading.clone();
            voice_rendaku(&mut rd_h, Voicing::Handakuten);
            lst.push(ReadingAlternative {
                reading: rd_h,
                r#type: r#type.to_string(),
                tag: ReadingTag::Rendaku,
                gem: entry.gem.clone(),
            });
        }
    }

    lst
}

/// `get-normal-readings` (`kanji.lisp:231`). Kun/on readings of `char`
/// (filtering `ja_na`), each expanded via [`get_reading_alternatives`],
/// then deduplicated by reading text keeping the first occurrence in
/// original order.
///
/// Upstream's `:from-end t` in `remove-duplicates` empirically keeps
/// the first occurrence in original-order (REPL-verified): the rendaku-
/// induced "び" of 日 is shadowed by the original `("び" "ja_kun" NIL)`
/// entry.
pub async fn get_normal_readings(
    ctx: &KaniranContext,
    char: char,
    rendaku: bool,
) -> Result<Vec<KanjiReading>, sqlx::Error> {
    let str: String = char.into();
    let typeset = vec!["ja_na".to_string()];
    let readings = get_readings_cache(ctx, &str, &typeset).await?;

    let mut main_readings: Vec<KanjiReading> = Vec::new();
    let mut alt_readings: Vec<KanjiReading> = Vec::new();
    for (reading, r#type) in &readings {
        let alternatives = get_reading_alternatives(reading, r#type, rendaku);
        // kanji.lisp:235 (loop ... for (main . rest) = ...)
        let mut iter = alternatives.into_iter();
        if let Some(main) = iter.next() {
            main_readings.push(main.into());
        }
        for alt in iter {
            alt_readings.push(alt.into());
        }
    }

    let mut combined: Vec<KanjiReading> = main_readings;
    combined.extend(alt_readings);

    // kanji.lisp:239 (remove-duplicates ... :test 'equal :key 'car :from-end t)
    let mut seen: HashSet<String> = HashSet::new();
    let mut deduped: Vec<KanjiReading> = Vec::with_capacity(combined.len());
    for entry in combined {
        if seen.insert(entry.reading.clone()) {
            deduped.push(entry);
        }
    }
    Ok(deduped)
}
