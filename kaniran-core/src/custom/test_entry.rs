//! Port of `ichiran/custom:test-entry` (gf — `dict-custom.lisp:23`).
//!
//! The 2-value `(values ok seq)` upstream return collapses into the
//! 4-variant [`TestEntryResult`] per CONVENTIONS §4.3.

use fancy_regex::Regex;

use crate::conn::kani_context::KaniranContext;
use crate::dict::match_glosses::{match_glosses, MatchValue};

use super::custom_source_class::{CustomEntry, CustomLoader};
use super::get_words::get_words;
use super::municipality_struct::Municipality;
use super::normalize_geo::normalize_geo;
use super::ward_struct::Ward;

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

fn map_match_glosses_to_disposition(
    result: Option<(MatchValue, bool)>,
) -> TestEntryResult {
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
    let pattern = format!(
        "(?i)^{name} \\(city(?:| in {pref_first_escaped})\\)$",
    );
    Regex::new(&pattern).expect("city update-gloss regex compiles")
}

fn build_pref_update_gloss(words: &[String]) -> Regex {
    // dict-custom.lisp:207-213 `(:sequence :case-insensitive-p :start-anchor ,(car words) " (" (:alternation :void "city, ") "prefecture)" :end-anchor)
    let name = fancy_regex::escape(&words[0]);
    let pattern = format!("(?i)^{name} \\((?:|city, )prefecture\\)$");
    Regex::new(&pattern).expect("pref update-gloss regex compiles")
}

#[cfg(test)]
mod tests {
    //! REPL fixtures (.103, ichiran/custom::test-entry against live
    //! Postgres), 2026-05-31.
    use super::*;
    use crate::custom::municipality_csv_class::MunicipalityCsv;
    use crate::custom::ward_csv_class::WardCsv;
    use std::path::PathBuf;
    use std::sync::Arc;

    async fn ctx_from_env() -> Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    /// REPL: (test-entry loader m) with text/reading 東京/とうきょう
    /// and definition "Tokyo Metropolis" → ok=NIL seq=1447690.
    #[tokio::test]
    async fn test_entry_municipality_skip_path() {
        let ctx = ctx_from_env().await;
        let loader =
            CustomLoader::Municipality(MunicipalityCsv::new(PathBuf::from("/tmp/x.csv")));
        let entry = CustomEntry::Municipality(Municipality {
            text: "東京".to_string(),
            reading: "とうきょう".to_string(),
            definition: "Tokyo Metropolis".to_string(),
            r#type: '都',
            prefecture: None,
        });
        let got = test_entry(&ctx, &loader, &entry).await.unwrap();
        assert_eq!(got, TestEntryResult::Skip);
    }

    /// REPL: bogus text/reading → no candidates → ok=T seq=NIL.
    #[tokio::test]
    async fn test_entry_municipality_insert_path() {
        let ctx = ctx_from_env().await;
        let loader =
            CustomLoader::Municipality(MunicipalityCsv::new(PathBuf::from("/tmp/x.csv")));
        let entry = CustomEntry::Municipality(Municipality {
            text: "ZZZ".to_string(),
            reading: "うあぱ".to_string(),
            definition: "ZZZ (city), Foo".to_string(),
            r#type: '市',
            prefecture: Some("Foo".to_string()),
        });
        let got = test_entry(&ctx, &loader, &entry).await.unwrap();
        assert_eq!(got, TestEntryResult::Insert);
    }

    /// REPL: 漢字/かんじ + bogus definition → ok=T seq=1213170.
    #[tokio::test]
    async fn test_entry_municipality_update_path() {
        let ctx = ctx_from_env().await;
        let loader =
            CustomLoader::Municipality(MunicipalityCsv::new(PathBuf::from("/tmp/x.csv")));
        let entry = CustomEntry::Municipality(Municipality {
            text: "漢字".to_string(),
            reading: "かんじ".to_string(),
            definition: "FAKE definition with xxx".to_string(),
            r#type: '市',
            prefecture: Some("FakePref Prefecture".to_string()),
        });
        let got = test_entry(&ctx, &loader, &entry).await.unwrap();
        assert_eq!(got, TestEntryResult::Update(1213170));
    }

    /// REPL: 中央区/ちゅうおうく + "Chuo Ward, Sapporo" → ok=NIL seq=12296020.
    #[tokio::test]
    async fn test_entry_ward_skip_path() {
        let ctx = ctx_from_env().await;
        let loader = CustomLoader::Ward(WardCsv::new(PathBuf::from("/tmp/x.csv")));
        let entry = CustomEntry::Ward(Ward {
            text: "中央区".to_string(),
            reading: "ちゅうおうく".to_string(),
            definition: "Chuo Ward, Sapporo".to_string(),
            city: "Sapporo".to_string(),
        });
        let got = test_entry(&ctx, &loader, &entry).await.unwrap();
        assert_eq!(got, TestEntryResult::Skip);
    }

    #[test]
    fn city_update_gloss_regex_shape() {
        let words = vec![
            "Yokohama".to_string(),
            "(city".to_string(),
            "Kanagawa Prefecture".to_string(),
        ];
        let entry = Municipality {
            text: "横浜".to_string(),
            reading: "よこはま".to_string(),
            definition: "Yokohama (city), Kanagawa Prefecture".to_string(),
            r#type: '市',
            prefecture: Some("Kanagawa Prefecture".to_string()),
        };
        let rg = build_city_update_gloss(&words, &entry);
        assert!(rg.is_match("Yokohama (city)").unwrap());
        assert!(rg.is_match("YOKOHAMA (CITY)").unwrap());
        assert!(rg.is_match("Yokohama (city in Kanagawa)").unwrap());
        assert!(!rg.is_match("Yokohama (city), Kanagawa Prefecture").unwrap());
        assert!(!rg.is_match("Yokohama (city in Tokyo)").unwrap());
    }

    #[test]
    fn pref_update_gloss_regex_shape() {
        let words = vec!["Kanagawa".to_string(), "Prefecture".to_string()];
        let rg = build_pref_update_gloss(&words);
        assert!(rg.is_match("Kanagawa (prefecture)").unwrap());
        assert!(rg.is_match("Kanagawa (city, prefecture)").unwrap());
        assert!(rg.is_match("KANAGAWA (PREFECTURE)").unwrap());
        assert!(!rg.is_match("Tokyo (prefecture)").unwrap());
        assert!(!rg.is_match("Kanagawa Prefecture").unwrap());
    }
}
