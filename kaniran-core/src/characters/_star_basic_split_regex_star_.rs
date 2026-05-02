//! Port of `ichiran/characters:*basic-split-regex*`
//! (`characters.lisp:131`).
//!
//! Composite pattern that recognizes one "word" token in
//! Japanese-mixed-with-digits text. Built upstream by `format`-stitching
//! the simpler `*digit-regex*`, `*decimal-point-regex*`, `*word-regex*`,
//! and `*num-word-regex*` globals; here the final 124-char value is
//! stored directly. Compilation lives in `*char-scanners*` and is
//! deferred to its own port.
//!
//! The negative lookbehind `(?<![.,]|[0-9０-９〇])` is what required
//! `fancy-regex` instead of the stdlib `regex` crate.

pub static BASIC_SPLIT_REGEX: &str = "((?:(?<![.,]|[0-9０-９〇])[0-9０-９〇]+|[々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー〇])[0-9０-９〇々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー]*[々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー〇]|[々ヶ〆一-龯ァ-ヺヽヾぁ-ゔゝゞー〇])";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_under_fancy_regex() {
        fancy_regex::Regex::new(BASIC_SPLIT_REGEX).expect("regex must compile");
    }
}
