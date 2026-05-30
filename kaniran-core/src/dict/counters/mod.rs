//! Port of `ichiran/dict-counters.lisp` — the counter-text class
//! hierarchy plus the `*counter-cache*` population pipeline and the
//! `find-counter` lookup that materializes per-query counter
//! instances.
//!
//! Six sections mirror the upstream's internal structure:
//!
//! - [`classes`] — the central `counter-text` class with its slot
//!   options and shared method bodies.
//! - [`subclasses`] — `number-text` + the 9 counter subclasses
//!   (`counter-age`, `counter-days-{kun,on}`, `counter-halfhour`,
//!   `counter-hifumi`, `counter-months`, `counter-people`,
//!   `counter-tsu`, `counter-wari`) that override `get-kana` /
//!   `value-string` / `verify`.
//! - [`dispatchers`] — the word-polymorphism generic functions whose
//!   counter-text method bodies make them dict-counters-resident:
//!   `common`, `word-conjugations`, `seq`, `text`, `source`, `verify`,
//!   `value-string`.
//! - [`special`] — the `*special-counters*` registry of 91
//!   `def-special-counter` callsites.
//! - [`cache`] — the `*counter-cache*` populator (driven by `*special-
//!   counters*` + 5 sidecar globals) and the `counter-join` rendering
//!   helper.
//! - [`find_counter`] — `find-counter` lookup + the 3 accessor
//!   helpers + `nokanji` / `ord` / `ordinal-str` / `get-digit`.

pub mod cache;
pub mod classes;
pub mod dispatchers;
pub mod find_counter;
pub mod special;
pub mod subclasses;
