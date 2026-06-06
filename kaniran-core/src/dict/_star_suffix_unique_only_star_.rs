//! Port of `ichiran/dict:*suffix-unique-only*` (`dict-grammar.lisp:330`).
//!
//! Registry of suffix classes that suppress the current suffix's
//! expansion in `find-word-suffix`, tagged with one of three match
//! behaviors (bare, `:desu`, `:sa`).

#[derive(Debug, Clone, Copy)]
pub enum SuffixUniqueOnly {
    Bare,
    Desu,
    Sa,
}

pub static SUFFIX_UNIQUE_ONLY: &[(&str, SuffixUniqueOnly)] = &[
    ("ii", SuffixUniqueOnly::Bare),
    ("seba", SuffixUniqueOnly::Bare),
    ("meba", SuffixUniqueOnly::Bare),
    ("beba", SuffixUniqueOnly::Bare),
    ("neba", SuffixUniqueOnly::Bare),
    ("geba", SuffixUniqueOnly::Bare),
    ("keba", SuffixUniqueOnly::Bare),
    ("reba", SuffixUniqueOnly::Bare),
    ("teba", SuffixUniqueOnly::Bare),
    ("eba", SuffixUniqueOnly::Bare),
    ("dewanai", SuffixUniqueOnly::Bare),
    ("nai-n", SuffixUniqueOnly::Bare),
    ("gai", SuffixUniqueOnly::Bare),
    ("nikui", SuffixUniqueOnly::Bare),
    ("mo", SuffixUniqueOnly::Bare),
    ("desu", SuffixUniqueOnly::Desu),
    ("ra", SuffixUniqueOnly::Bare),
    ("sa", SuffixUniqueOnly::Sa),
];
