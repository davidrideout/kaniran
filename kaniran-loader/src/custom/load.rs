use super::constants::MUNICIPALITY_TYPES_DESCRIPTION;
use super::entry::insert;
use super::types::{
    CustomEntry, CustomLoader, Municipality, MunicipalityCsv, Ward, WardCsv, XmlLoader,
};
use kaniran_core::conn::kani_context::KaniranContext;
use std::path::PathBuf;

/// Port of `ichiran/custom:get-words` (gf — `dict-custom.lisp:176`).
pub fn get_words(entry: &CustomEntry) -> Vec<String> {
    match entry {
        CustomEntry::Municipality(m) => get_words_municipality(m),
        CustomEntry::Ward(w) => get_words_ward(w),
        CustomEntry::Xml(_) => {
            panic!("get-words: no method for xml-entry (dict-custom.lisp:176)")
        }
    }
}

fn get_words_municipality(entry: &Municipality) -> Vec<String> {
    // dict-custom.lisp:178 (typeword (cdr (assoc (municipality-type entry) *municipality-types-description*)))
    let mut typeword: Option<String> = MUNICIPALITY_TYPES_DESCRIPTION.iter().find_map(|(t, d)| {
        if *t == entry.r#type {
            d.map(|s| s.to_string())
        } else {
            None
        }
    });
    // dict-custom.lisp:179 (name (car (split-sequence #\Space (municipality-definition entry))))
    let name = entry
        .definition
        .split(' ')
        .next()
        .expect("split always yields at least one element")
        .to_string();
    // dict-custom.lisp:180-181 (when (and typeword (alexandria:ends-with #\) typeword)) (setf typeword (subseq typeword 0 (1- (length typeword)))))
    if let Some(tw) = &typeword {
        if tw.ends_with(')') {
            let trimmed = tw[..tw.len() - ')'.len_utf8()].to_string();
            typeword = Some(trimmed);
        }
    }
    // dict-custom.lisp:183 (remove-if 'null (list name typeword (municipality-prefecture entry)))
    let mut out = vec![name];
    if let Some(tw) = typeword {
        out.push(tw);
    }
    if let Some(p) = &entry.prefecture {
        out.push(p.clone());
    }
    out
}

fn get_words_ward(entry: &Ward) -> Vec<String> {
    // dict-custom.lisp:287 (name (car (split-sequence #\Space (ward-definition entry))))
    let name = entry
        .definition
        .split(' ')
        .next()
        .expect("split always yields at least one element")
        .to_string();
    // dict-custom.lisp:288 (list name "Ward" (ward-city entry))
    vec![name, "Ward".to_string(), entry.city.clone()]
}

/// Port of `ichiran/custom:slurp` (gf — `dict-custom.lisp:7`).
pub fn slurp(loader: &mut CustomLoader) -> std::io::Result<usize> {
    match loader {
        // dict-custom.lisp:65 (defmethod slurp ((loader xml-loader)) ...)
        CustomLoader::Xml(x) => x.slurp()?,
        // dict-custom.lisp:88 (defmethod slurp ((loader csv-loader)) ...) + dict-custom.lisp:168 (defmethod slurp :after ((loader municipality-csv)) ...)
        CustomLoader::Municipality(m) => m.slurp()?,
        // dict-custom.lisp:271 (defmethod slurp ((loader ward-csv)) ...)
        CustomLoader::Ward(w) => w.slurp()?,
    }
    // dict-custom.lisp:10-12 (defmethod slurp :around (source) (call-next-method) (length (entries source)))
    Ok(loader.base().entries.len())
}

/// Port of `ichiran/custom:source-path` (`dict-custom.lisp:315`).
pub fn source_path(file: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("sources")
        .join(file)
}

/// Port of `ichiran/custom:get-custom-data` (`dict-custom.lisp:318`).
///
/// Returns the three built-in loaders, each tagged with its keyword
/// (`:extra`, `:municipality`, `:ward`).
/// Tag identifying one of the built-in custom-data loaders.
///
/// ```text
/// CustomDataKey::Extra         // :extra
/// CustomDataKey::Municipality  // :municipality
/// CustomDataKey::Ward          // :ward
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CustomDataKey {
    Extra,
    Municipality,
    Ward,
}

pub fn get_custom_data() -> Vec<(CustomDataKey, CustomLoader)> {
    // dict-custom.lisp:321-324 (`( :extra (xml-loader :source-file ,(source-path "extra.xml"))
    //                             :municipality (municipality-csv :source-file ,(source-path "jichitai.csv"))
    //                             :ward (ward-csv :source-file ,(source-path "gyoseiku.csv")) ))
    vec![
        (
            CustomDataKey::Extra,
            CustomLoader::Xml(XmlLoader::new(source_path("extra.xml"))),
        ),
        (
            CustomDataKey::Municipality,
            CustomLoader::Municipality(MunicipalityCsv::new(source_path("jichitai.csv"))),
        ),
        (
            CustomDataKey::Ward,
            CustomLoader::Ward(WardCsv::new(source_path("gyoseiku.csv"))),
        ),
    ]
}

/// Port of `ichiran/custom:load-custom-data` (`dict-custom.lisp:327`).
///
/// Walks the built-in loaders, optionally filtered by `keys`, and for
/// each runs slurp (file read) then insert (database); `silent_p`
/// suppresses progress prints.
/// Combined I/O + database error for the slurp → insert pipeline.
///
/// ```text
/// LoadCustomDataError::Io(io::Error)        // slurp file-read failure
/// LoadCustomDataError::Database(sqlx::Error) // insert/test/update failure
/// ```
#[derive(Debug, thiserror::Error)]
pub enum LoadCustomDataError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("database: {0}")]
    Database(#[from] sqlx::Error),
}

pub async fn load_custom_data(
    ctx: &KaniranContext,
    keys: &[CustomDataKey],
    silent_p: bool,
) -> Result<(), LoadCustomDataError> {
    // dict-custom.lisp:328-329 (loaders (loop for (key loader) on (get-custom-data) by #'cddr
    //                                       if (or (not keys) (find key keys)) collect loader))
    let mut loaders: Vec<_> = get_custom_data()
        .into_iter()
        .filter(|(key, _)| keys.is_empty() || keys.contains(key))
        .map(|(_, loader)| loader)
        .collect();
    // dict-custom.lisp:331-335 (dolist (loader loaders) ...)
    for loader in loaders.iter_mut() {
        // dict-custom.lisp:332 (unless *silent-p* (format t "Loading ~a~%" (description loader)))
        if !silent_p {
            println!("Loading {}", loader.base().description);
        }
        // dict-custom.lisp:333 (let ((n-entries (slurp loader))) ...)
        let n_entries = slurp(loader)?;
        // dict-custom.lisp:334 (unless *silent-p* (format t "Inserting ~a entries~%" n-entries))
        if !silent_p {
            println!("Inserting {n_entries} entries");
        }
        // dict-custom.lisp:335 (insert loader)
        insert(ctx, loader).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
