//! Port of `ichiran/kanji:*kanjidic-path*` (`settings.lisp:16`).
//!
//! Filesystem path to the kanjidic2.xml dump consumed by
//! `load-kanjidic`. The upstream `defvar` at `kanji.lisp:3` introduces
//! the symbol with a placeholder (`#P"e:/dump/kanjidic2.xml"`) and
//! `settings.lisp:16` rebinds it per-deployment via `defparameter`.
//! The Rust port encodes the introspected value captured on the
//! reference host; downstream consumers that need a different path
//! shadow this constant or read from configuration before invoking
//! the loader.

pub static KANJIDIC_PATH: &str = "/home/david/storage/ichiran/dump/kanjidic2.xml";
