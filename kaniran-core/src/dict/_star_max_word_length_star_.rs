//! Port of `ichiran/dict:*max-word-length*` (`dict.lisp:486`).
//!
//! Upper bound on the character length of a word
//! [`super::find_word::find_word`] will look up in the JMdict text
//! tables. Anything longer is treated as nonexistent without hitting
//! the database. The upstream comment notes the kana_text and
//! kanji_text columns capped at 37 and 27 characters at the time the
//! constant was written; 50 leaves headroom while still
//! short-circuiting unrealistically long substrings the segmenter
//! generates as candidates.

pub const MAX_WORD_LENGTH: usize = 50;
