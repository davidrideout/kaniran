//! Port of `ichiran/custom:load-custom-data` (`dict-custom.lisp:327`).
//!
//! Walks the built-in loaders, optionally filtered by `keys`, and for
//! each runs slurp (file read) then insert (database); `silent_p`
//! suppresses progress prints.

use crate::conn::kani_context::KaniranContext;

use super::get_custom_data::{get_custom_data, CustomDataKey};
use super::insert::insert;
use super::slurp::slurp;

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
