//! Port of `ichiran/dict:*suffix-next-end*` (`dict.lisp:1050`).
//!
//! Caller-scoped current character end-position used as the lookup key
//! into `*suffix-map-temp*`. Signed: the `find-word-suffix` recursion
//! subtracts the suffix length and can go negative, and a negative key
//! simply misses the map.
