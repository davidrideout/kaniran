use super::geo::normalize_geo;
use super::load::get_words;
use super::types::{CustomEntry, CustomLoader, Municipality, Ward};
use super::xml::as_xml;
use crate::conn::kani_context::KaniranContext;
use crate::dict::add_new_sense::add_new_sense;
use crate::dict::load_entry::{load_entry, LoadEntryIfExists, LoadEntrySeq};
use crate::dict::match_glosses::{match_glosses, MatchValue};
use crate::dict::next_seq::next_seq;
use fancy_regex::Regex;

/// Port of `ichiran/custom:insert` (gf — `dict-custom.lisp:14`).
pub async fn insert(ctx: &KaniranContext, loader: &CustomLoader) -> Result<(), sqlx::Error> {
    match loader {
        // dict-custom.lisp:74 (defmethod insert ((loader xml-loader)) ...)
        CustomLoader::Xml(x) => x.insert(ctx).await,
        _ => insert_default(ctx, loader).await,
    }
}

async fn insert_default(ctx: &KaniranContext, loader: &CustomLoader) -> Result<(), sqlx::Error> {
    // dict-custom.lisp:39-51 (defmethod insert (source) (with-connection *connection* (loop with cur-seq = (ichiran/dict::next-seq) for entry in (entries source) do (multiple-value-bind (ok seq) (test-entry source entry) (when ok (cond ((consp seq) (apply 'update-entry-gloss source entry seq)) (seq (update-entry source entry seq)) (t (insert-entry source entry cur-seq) (incf cur-seq))))))))
    let mut cur_seq = next_seq(ctx).await?;
    for entry in &loader.base().entries {
        match test_entry(ctx, loader, entry).await? {
            TestEntryResult::UpdateGloss(seq, gloss) => {
                update_entry_gloss(ctx, loader, entry, seq, &gloss).await?;
            }
            TestEntryResult::Update(seq) => {
                update_entry(ctx, loader, entry, seq).await?;
            }
            TestEntryResult::Insert => {
                insert_entry(ctx, loader, entry, cur_seq).await?;
                cur_seq += 1;
            }
            TestEntryResult::Skip => {}
        }
    }
    Ok(())
}

/// Port of `ichiran/custom:insert-entry` (gf — `dict-custom.lisp:30`).
pub async fn insert_entry(
    ctx: &KaniranContext,
    _source: &CustomLoader,
    entry: &CustomEntry,
    seq: i32,
) -> Result<(), sqlx::Error> {
    // dict-custom.lisp:252 (ichiran/dict::load-entry (as-xml entry) :seq seq)
    // dict-custom.lisp:308 (ichiran/dict::load-entry (as-xml entry) :seq seq)
    let content = as_xml(entry);
    load_entry(
        ctx,
        &content,
        LoadEntryIfExists::None,
        None,
        LoadEntrySeq::Int(seq),
        false,
    )
    .await?;
    Ok(())
}

/// Port of `ichiran/custom:process-entry` (gf — `dict-custom.lisp:19`).
pub fn process_entry(loader: &CustomLoader, row: &[String]) -> Vec<CustomEntry> {
    match loader {
        // dict-custom.lisp:142 (defmethod process-entry ((loader municipality-csv) row) ...)
        CustomLoader::Municipality(m) => m
            .process_entry(row)
            .into_iter()
            .map(CustomEntry::Municipality)
            .collect(),
        // dict-custom.lisp:21 (:method (source entry) (list entry))
        _ => panic!("process-entry: default method received non-CustomEntry row"),
    }
}

/// Port of `ichiran/custom:test-entry` (gf — `dict-custom.lisp:23`).
///
/// Decides whether an entry should be inserted, updated, gloss-updated,
/// or skipped by matching its words against existing dictionary glosses.
/// Resolved disposition for an entry under [`match_glosses`].
///
/// ```text
/// TestEntryResult::Insert                       // (values t nil)
/// TestEntryResult::Update(1213170)              // (values t seq)
/// TestEntryResult::UpdateGloss(1213170, "X")    // (values t (list seq match))
/// TestEntryResult::Skip                         // (values nil seq)
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestEntryResult {
    Insert,
    Update(i32),
    UpdateGloss(i32, String),
    Skip,
}

pub async fn test_entry(
    ctx: &KaniranContext,
    _source: &CustomLoader,
    entry: &CustomEntry,
) -> Result<TestEntryResult, sqlx::Error> {
    match entry {
        // dict-custom.lisp:185 (defmethod test-entry (loader (entry municipality)) ...)
        CustomEntry::Municipality(m) => test_entry_municipality(ctx, m).await,
        // dict-custom.lisp:290 (defmethod test-entry (loader (entry ward)) ...)
        CustomEntry::Ward(w) => test_entry_ward(ctx, w).await,
        // dict-custom.lisp:28 (:method (source entry) t)
        CustomEntry::Xml(_) => Ok(TestEntryResult::Insert),
    }
}

async fn test_entry_municipality(
    ctx: &KaniranContext,
    entry: &Municipality,
) -> Result<TestEntryResult, sqlx::Error> {
    // dict-custom.lisp:187 (words (get-words entry))
    let words = get_words(&CustomEntry::Municipality(entry.clone()));
    let words_refs: Vec<&str> = words.iter().map(String::as_str).collect();
    // dict-custom.lisp:193-214 (:update-gloss (case (municipality-type entry) (#\市 ...) (#\県 ...)))
    let update_gloss: Option<Regex> = match entry.r#type {
        '市' => Some(build_city_update_gloss(&words, entry)),
        '県' => Some(build_pref_update_gloss(&words)),
        _ => None,
    };
    let normalize_fn: &dyn Fn(&str) -> String = &normalize_geo;
    // dict-custom.lisp:188-214 (ichiran/dict:match-glosses (municipality-text entry) (municipality-reading entry) words :normalize 'normalize-geo :update-gloss ...)
    let result = match_glosses(
        ctx,
        &entry.text,
        Some(&entry.reading),
        &words_refs,
        Some(normalize_fn),
        update_gloss.as_ref(),
    )
    .await?;
    Ok(map_match_glosses_to_disposition(result))
}

async fn test_entry_ward(
    ctx: &KaniranContext,
    entry: &Ward,
) -> Result<TestEntryResult, sqlx::Error> {
    // dict-custom.lisp:292 (words (get-words entry))
    let words = get_words(&CustomEntry::Ward(entry.clone()));
    let words_refs: Vec<&str> = words.iter().map(String::as_str).collect();
    let normalize_fn: &dyn Fn(&str) -> String = &normalize_geo;
    // dict-custom.lisp:293-297 (ichiran/dict:match-glosses (ward-text entry) (ward-reading entry) words :normalize 'normalize-geo)
    let result = match_glosses(
        ctx,
        &entry.text,
        Some(&entry.reading),
        &words_refs,
        Some(normalize_fn),
        None,
    )
    .await?;
    // dict-custom.lisp:298-300 (cond ((not seq) (values t nil)) (match-p (values nil seq)) (t (values t seq)))
    Ok(match result {
        None => TestEntryResult::Insert,
        Some((MatchValue::Seq(_), true)) => TestEntryResult::Skip,
        Some((MatchValue::Seq(seq), false)) => TestEntryResult::Update(seq),
        Some((MatchValue::SeqAndGloss(_, _), _)) => {
            unreachable!("ward test-entry: SeqAndGloss requires :update-gloss")
        }
    })
}

fn map_match_glosses_to_disposition(result: Option<(MatchValue, bool)>) -> TestEntryResult {
    // dict-custom.lisp:215-218 (cond ((not seq) (values t nil)) ((consp seq) (values t seq)) (match-p (values nil seq)) (t (values t seq)))
    match result {
        None => TestEntryResult::Insert,
        Some((MatchValue::SeqAndGloss(seq, gloss), _)) => TestEntryResult::UpdateGloss(seq, gloss),
        Some((MatchValue::Seq(_), true)) => TestEntryResult::Skip,
        Some((MatchValue::Seq(seq), false)) => TestEntryResult::Update(seq),
    }
}

fn build_city_update_gloss(words: &[String], entry: &Municipality) -> Regex {
    // dict-custom.lisp:195-205 `(:sequence :case-insensitive-p :start-anchor ,(car words) " (city" (:alternation :void (:sequence " in " ,(car (split-sequence #\Space (municipality-prefecture entry))))) ")" :end-anchor)
    let name = fancy_regex::escape(&words[0]);
    let pref_first = entry
        .prefecture
        .as_deref()
        .expect("city test-entry: prefecture must be set")
        .split(' ')
        .next()
        .expect("split always yields at least one element");
    let pref_first_escaped = fancy_regex::escape(pref_first);
    let pattern = format!("(?i)^{name} \\(city(?:| in {pref_first_escaped})\\)$",);
    Regex::new(&pattern).expect("city update-gloss regex compiles")
}

fn build_pref_update_gloss(words: &[String]) -> Regex {
    // dict-custom.lisp:207-213 `(:sequence :case-insensitive-p :start-anchor ,(car words) " (" (:alternation :void "city, ") "prefecture)" :end-anchor)
    let name = fancy_regex::escape(&words[0]);
    let pattern = format!("(?i)^{name} \\((?:|city, )prefecture\\)$");
    Regex::new(&pattern).expect("pref update-gloss regex compiles")
}

/// Port of `ichiran/custom:update-entry` (gf — `dict-custom.lisp:33`).
pub async fn update_entry(
    ctx: &KaniranContext,
    _source: &CustomLoader,
    entry: &CustomEntry,
    seq: i32,
) -> Result<(), sqlx::Error> {
    // dict-custom.lisp:257 (ichiran/dict::add-new-sense seq '("n") (list (municipality-definition entry)))
    // dict-custom.lisp:313 (ichiran/dict::add-new-sense seq '("n") (list (ward-definition entry)))
    let definition = match entry {
        CustomEntry::Municipality(m) => m.definition.clone(),
        CustomEntry::Ward(w) => w.definition.clone(),
        CustomEntry::Xml(_) => {
            panic!("update-entry: no upstream method for xml-entry (dict-custom.lisp:33)")
        }
    };
    add_new_sense(ctx, seq, &["n".to_string()], &[definition]).await?;
    Ok(())
}

/// Port of `ichiran/custom:update-entry-gloss` (gf — `dict-custom.lisp:36`).
pub async fn update_entry_gloss(
    ctx: &KaniranContext,
    _source: &CustomLoader,
    entry: &CustomEntry,
    seq: i32,
    gloss: &str,
) -> Result<(), sqlx::Error> {
    // dict-custom.lisp:260 (new-gloss (municipality-definition entry))
    let new_gloss = match entry {
        CustomEntry::Municipality(m) => m.definition.clone(),
        CustomEntry::Ward(_) | CustomEntry::Xml(_) => {
            panic!(
                "update-entry-gloss: no upstream method for non-municipality entry (dict-custom.lisp:36)"
            )
        }
    };
    // dict-custom.lisp:263-264 (postmodern:query (:update 'gloss :set 'text new-gloss :from 'sense :where (:and (:= 'gloss.sense-id 'sense.id) (:= 'sense.seq seq) (:= 'gloss.text gloss))))
    sqlx::query(
        "UPDATE gloss SET text = $1 FROM sense \
         WHERE gloss.sense_id = sense.id AND sense.seq = $2 AND gloss.text = $3",
    )
    .bind(&new_gloss)
    .bind(seq)
    .bind(gloss)
    .execute(&ctx.pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests;
