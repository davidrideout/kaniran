//! Port of `ichiran/characters:hash-from-list` (`characters.lisp:55-61`).
//!
//! Doc-only port per CONVENTIONS §4.8.
//!
//! The Lisp macro defines a `defparameter` whose value is a hash-table
//! built from a flat plist `(key1 value1 key2 value2 ...)`. The macro
//! itself encodes only that loop; the data registered via each
//! call-site is what the codebase actually consumes.
//!
//! Six upstream call-sites:
//!
//! - `*dakuten-hash*`     — `characters.lisp:63`  → ported as
//!   [`super::_star_dakuten_hash_star_`].
//! - `*handakuten-hash*`  — `characters.lisp:70`  → ported as
//!   [`super::_star_handakuten_hash_star_`].
//! - `*undakuten-hash*`   — `characters.lisp:73`  → ported as
//!   [`super::_star_undakuten_hash_star_`].
//! - `*suffix-description*` — `dict-grammar.lisp:108` (`ichiran/dict`) — pending.
//! - `*hepburn-kana-table*`  — `romanize.lisp:81`  (`ichiran/romanize`) — pending.
//! - `*kunrei-siki-kana-table*` — `romanize.lisp:172` (`ichiran/romanize`) — pending.
//!
//! Each call-site lands directly as a Rust `HashMap` constructed in
//! its own `_star_<name>_star_.rs` file (see
//! [`crate::kani::naming`]). There is nothing to translate at the
//! macro level — no fourth consumer of the macro's expansion shape
//! exists in the codebase.
