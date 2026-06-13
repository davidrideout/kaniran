use crate::conn::kani_backend::KaniBackend;
use crate::conn::kani_context::KaniranContext;
use crate::dict::load::conj_rules::get_conj_description;
use serde_json::{Map, Value};
use sqlx::postgres::PgRow;
use sqlx::{FromRow, Row};

/// Port of `ichiran/dict:entry` (`dict.lisp:26`).
///
/// Row representation of one JMdict entry, mapped 1:1 to the
/// `public.entry` Postgres table.
///
/// The DB `content` column (raw JMdict XML, ~80 MB across the corpus)
/// is intentionally not materialized here: nothing at runtime reads
/// it, and the errata UPDATEs that used to round-trip it have been
/// rewritten to skip the column. JMdict loaders write the column on
/// initial DB population for audit/debugging — Postgres keeps it,
/// Rust never lifts it back.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "rkyv", derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize))]
pub struct Entry {
    pub seq: i32,
    pub root_p: bool,
    pub n_kanji: i32,
    pub n_kana: i32,
    pub primary_nokanji: bool,
}

impl<'r> FromRow<'r, PgRow> for Entry {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(Entry {
            seq: row.try_get("seq")?,
            root_p: row.try_get("root_p")?,
            n_kanji: row.try_get("n_kanji")?,
            n_kana: row.try_get("n_kana")?,
            primary_nokanji: row.try_get("primary_nokanji")?,
        })
    }
}

impl Entry {
    /// `get-text` override — `dict.lisp:47-49`:
    ///
    /// ```lisp
    /// (defmethod get-text ((obj entry))
    ///   (text (car (select-dao (if (> (n-kanji obj) 0) 'kanji-text 'kana-text)
    ///                          (:and (:= 'seq (seq obj)) (:= 'ord 0))))))
    /// ```
    ///
    /// Returns the `text` of the entry's headword row at `ord = 0` —
    /// from `kanji_text` when the entry has any kanji writings,
    /// otherwise from `kana_text`. Reached only from locally-typed
    /// callsites (`dict.lisp:67` `entry-digest`, `dict-load.lisp:135`);
    /// `entry` is not a variant of
    /// [`crate::dict::kani_word::KaniWordDispatchEnum`] (see that
    /// module's header) so no polymorphic `get-text` dispatch flows
    /// here.
    ///
    /// Diverges from the upstream lambda list `(obj)` only by:
    /// - taking `&KaniranContext` for the database handle, replacing
    ///   the upstream dynamic `*connection*` per
    ///   [`crate::conn::kani_context`];
    /// - returning [`Option<String>`] rather than a raw string —
    ///   upstream errors on `(text nil)` if the headword row is
    ///   absent (data-integrity violation), which the Rust port
    ///   surfaces as [`None`].
    pub async fn get_text(&self, ctx: &KaniranContext) -> Result<Option<String>, sqlx::Error> {
        if self.n_kanji > 0 {
            ctx.store.headword_kanji_text(self.seq).await
        } else {
            ctx.store.headword_kana_text(self.seq).await
        }
    }

    /// `get-kana` override — `dict.lisp:44-45`:
    ///
    /// ```lisp
    /// (defmethod get-kana ((obj entry))
    ///   (text (car (select-dao 'kana-text (:and (:= 'seq (seq obj)) (:= 'ord 0))))))
    /// ```
    ///
    /// Returns the `text` of the entry's headword `kana_text` row at
    /// `ord = 0`. Reached only from locally-typed callsites
    /// (`entry-digest` at `dict.lisp:67` is the canonical one);
    /// `entry` is not a variant of
    /// [`crate::dict::kani_word::KaniWordDispatchEnum`] (see that module's
    /// header) so no polymorphic `get-kana` dispatch flows here.
    ///
    /// Diverges from the upstream lambda list `(obj)` only by:
    /// - taking `&KaniranContext` for the database handle, replacing
    ///   the upstream dynamic `*connection*` per
    ///   [`crate::conn::kani_context`];
    /// - returning [`Option<String>`] rather than a raw string —
    ///   upstream errors on `(text nil)` if the headword row is
    ///   absent (data-integrity violation), which the Rust port
    ///   surfaces as [`None`].
    pub async fn get_kana(&self, ctx: &KaniranContext) -> Result<Option<String>, sqlx::Error> {
        ctx.store.headword_kana_text(self.seq).await
    }
}

/// Port of `ichiran/dict:recalc-entry-stats` (`dict.lisp:55`).
///
/// Recomputes the `n_kanji` / `n_kana` row-count caches on the `entry`
/// rows whose `seq` is in `entries`, from their current `kanji_text` /
/// `kana_text` children.
pub async fn recalc_entry_stats(ctx: &KaniranContext, entries: &[i32]) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE entry SET \
         n_kanji = (SELECT COUNT(id) FROM kanji_text WHERE kanji_text.seq = entry.seq), \
         n_kana = (SELECT COUNT(id) FROM kana_text WHERE kana_text.seq = entry.seq) \
         WHERE entry.seq = ANY($1)",
    )
    .bind(entries)
    .execute(&ctx.pool)
    .await?;
    Ok(result.rows_affected())
}

/// Port of `ichiran/dict:recalc-entry-stats-all` (`dict.lisp:61`).
///
/// Recomputes the `n_kanji` / `n_kana` row-count caches on every
/// `entry` from its current `kanji_text` / `kana_text` children.
pub async fn recalc_entry_stats_all(ctx: &KaniranContext) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE entry SET \
         n_kanji = (SELECT COUNT(id) FROM kanji_text WHERE kanji_text.seq = entry.seq), \
         n_kana = (SELECT COUNT(id) FROM kana_text WHERE kana_text.seq = entry.seq)",
    )
    .execute(&ctx.pool)
    .await?;
    Ok(result.rows_affected())
}

/// Port of `ichiran/dict:entry-digest` (`dict.lisp:64`).
///
/// Returns `(seq, text, kana)` for an entry.
pub async fn entry_digest(
    ctx: &KaniranContext,
    entry: &Entry,
) -> Result<(i32, Option<String>, Option<String>), sqlx::Error> {
    Ok((
        entry.seq,
        entry.get_text(ctx).await?,
        entry.get_kana(ctx).await?,
    ))
}

/// Port of `ichiran/dict:simple-text` (`dict.lisp:69`).
///
/// Abstract CLOS base for the [`crate::dict::dao::KanjiText`]
/// and [`crate::dict::dao::KanaText`] DAO row classes,
/// holding the two runtime-mutable state slots that are not persisted.
#[derive(Debug, Clone, Default)]
pub struct SimpleText {
    pub conjugations: Option<WordConjugations>,
    pub hintedp: bool,
}

/// Value space of the upstream `conjugations` slot. The Lisp slot can
/// hold `nil` (no annotation), the symbol `:root` (entry kept as
/// itself), or a list of integer conjugation ids tying a derived
/// reading back to the conjugation rows that produced it. The `nil`
/// case is the `None` of `Option<WordConjugations>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WordConjugations {
    Root,
    Ids(Vec<i32>),
}

/// Port of `ichiran/dict:kanji-text` (`dict.lisp:86`).
///
/// Row representation of a kanji writing of a JMdict entry.
#[derive(Debug, Clone)]
pub struct KanjiText {
    pub id: i32,
    pub seq: i32,
    pub text: String,
    pub ord: i32,
    pub common: Option<i32>,
    pub common_tags: String,
    pub conjugate_p: bool,
    pub nokanji: bool,
    pub best_kana: Option<String>,
    pub state: SimpleText,
}

impl<'r> FromRow<'r, PgRow> for KanjiText {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(KanjiText {
            id: row.try_get("id")?,
            seq: row.try_get("seq")?,
            text: row.try_get("text")?,
            ord: row.try_get("ord")?,
            common: row.try_get("common")?,
            common_tags: row.try_get("common_tags")?,
            conjugate_p: row.try_get("conjugate_p")?,
            nokanji: row.try_get("nokanji")?,
            best_kana: row.try_get("best_kana")?,
            state: SimpleText::default(),
        })
    }
}

/// Port of `ichiran/dict:kana-text` (`dict.lisp:128`).
///
/// Row representation of a kana reading of a JMdict entry.
#[derive(Debug, Clone)]
pub struct KanaText {
    pub id: i32,
    pub seq: i32,
    pub text: String,
    pub ord: i32,
    pub common: Option<i32>,
    pub common_tags: String,
    pub conjugate_p: bool,
    pub nokanji: bool,
    pub best_kanji: Option<String>,
    pub state: SimpleText,
}

impl<'r> FromRow<'r, PgRow> for KanaText {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(KanaText {
            id: row.try_get("id")?,
            seq: row.try_get("seq")?,
            text: row.try_get("text")?,
            ord: row.try_get("ord")?,
            common: row.try_get("common")?,
            common_tags: row.try_get("common_tags")?,
            conjugate_p: row.try_get("conjugate_p")?,
            nokanji: row.try_get("nokanji")?,
            best_kanji: row.try_get("best_kanji")?,
            state: SimpleText::default(),
        })
    }
}

/// Port of `ichiran/dict:sense` (`dict.lisp:166`).
///
/// Row of the `public.sense` table — one numbered meaning attached to
/// an entry, ordered within the entry by the `ord` column.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "rkyv", derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize))]
pub struct Sense {
    pub id: i32,
    pub seq: i32,
    pub ord: i32,
}

impl<'r> FromRow<'r, PgRow> for Sense {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(Sense {
            id: row.try_get("id")?,
            seq: row.try_get("seq")?,
            ord: row.try_get("ord")?,
        })
    }
}

/// Port of `ichiran/dict:gloss` (`dict.lisp:178`).
///
/// Row representation of one English gloss attached to a JMdict sense.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "rkyv", derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize))]
pub struct Gloss {
    pub id: i32,
    pub sense_id: i32,
    pub text: String,
    pub ord: i32,
}

impl<'r> FromRow<'r, PgRow> for Gloss {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(Gloss {
            id: row.try_get("id")?,
            sense_id: row.try_get("sense_id")?,
            text: row.try_get("text")?,
            ord: row.try_get("ord")?,
        })
    }
}

/// Port of `ichiran/dict:sense-prop` (`dict.lisp:197`).
///
/// Row of the `public.sense_prop` table — one tagged property on a
/// sense, where `tag` is the property kind (`pos`, `stagk`, `misc`, …)
/// and `text` holds its value.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "rkyv", derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize))]
pub struct SenseProp {
    pub id: i32,
    pub tag: String,
    pub sense_id: i32,
    pub text: String,
    pub ord: i32,
    pub seq: i32,
}

impl<'r> FromRow<'r, PgRow> for SenseProp {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(SenseProp {
            id: row.try_get("id")?,
            tag: row.try_get("tag")?,
            sense_id: row.try_get("sense_id")?,
            text: row.try_get("text")?,
            ord: row.try_get("ord")?,
            seq: row.try_get("seq")?,
        })
    }
}

/// Port of `ichiran/dict:restricted-readings` (`dict.lisp:221`).
///
/// Row representation of one JMdict re_restr / ke_restr restriction,
/// mapped 1:1 to the `public.restricted_readings` Postgres table
/// populated by ichiran's schema. Each row links a kana `reading` to
/// the kanji `text` form within an entry (`seq`) it is allowed to
/// pair with — used to filter readings to the subset valid for a
/// given kanji surface.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "rkyv", derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize))]
pub struct RestrictedReadings {
    pub id: i32,
    pub seq: i32,
    pub reading: String,
    pub text: String,
}

impl<'r> FromRow<'r, PgRow> for RestrictedReadings {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(RestrictedReadings {
            id: row.try_get("id")?,
            seq: row.try_get("seq")?,
            reading: row.try_get("reading")?,
            text: row.try_get("text")?,
        })
    }
}

/// Port of `ichiran/dict:conjugation` (`dict.lisp:238`).
///
/// Row of the `public.conjugation` table — one conjugation link
/// recording that entry `seq` was derived from entry `seq_from`,
/// optionally via an intermediate entry `seq_via`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "rkyv", derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize))]
pub struct Conjugation {
    pub id: i32,
    pub seq: i32,
    pub seq_from: i32,
    pub seq_via: Option<i32>,
}

impl<'r> FromRow<'r, PgRow> for Conjugation {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(Conjugation {
            id: row.try_get("id")?,
            seq: row.try_get("seq")?,
            seq_from: row.try_get("from")?,
            seq_via: row.try_get("via")?,
        })
    }
}

/// Port of `ichiran/dict:conj-prop` (`dict.lisp:262`).
///
/// Row of the `public.conj_prop` table — one tagged property attached
/// to a conjugation, the `(pos, conj-type, neg, fml)` quadruple naming
/// which conjugation form of which part-of-speech the row describes.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "rkyv", derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize))]
pub struct ConjProp {
    pub id: i32,
    pub conj_id: i32,
    pub conj_type: i32,
    pub pos: String,
    pub neg: Option<bool>,
    pub fml: Option<bool>,
}

impl<'r> FromRow<'r, PgRow> for ConjProp {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(ConjProp {
            id: row.try_get("id")?,
            conj_id: row.try_get("conj_id")?,
            conj_type: row.try_get("conj_type")?,
            pos: row.try_get("pos")?,
            neg: row.try_get("neg")?,
            fml: row.try_get("fml")?,
        })
    }
}

/// Port of `ichiran/dict:conj-info-short` (`dict.lisp:277`).
///
/// Formats a [`ConjProp`] as `[<pos>] <description>` followed by an
/// optional ` Affirmative`/` Negative` and ` Plain`/` Formal` selected
/// by the nullable `neg`/`fml` flags (`db-null` → omitted).
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

/// Port of `ichiran/dict:conj-prop-json` (`dict.lisp:285`).
///
/// Builds the JSON object for one [`ConjProp`] — `pos` and `type`
/// always, plus `neg`/`fml` only when the flag is the true state.
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

/// Port of `ichiran/dict:conj-source-reading` (`dict.lisp:309`).
///
/// Row of the `public.conj_source_reading` table — one (text,
/// source-text) pair giving a rendered conjugated form and the
/// dictionary surface form it derives from.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "rkyv", derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize))]
pub struct ConjSourceReading {
    pub id: i32,
    pub conj_id: i32,
    pub text: String,
    pub source_text: String,
}

impl<'r> FromRow<'r, PgRow> for ConjSourceReading {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(ConjSourceReading {
            id: row.try_get("id")?,
            conj_id: row.try_get("conj_id")?,
            text: row.try_get("text")?,
            source_text: row.try_get("source_text")?,
        })
    }
}

#[cfg(test)]
mod tests;
