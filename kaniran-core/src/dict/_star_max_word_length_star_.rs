//! Port of `ichiran/dict:*max-word-length*` (`dict.lisp:486`).
//!
//! Upper bound (50 chars) on the length of a word looked up in the
//! JMdict text tables; anything longer is treated as nonexistent
//! without hitting the database.

pub const MAX_WORD_LENGTH: usize = 50;
