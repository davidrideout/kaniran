//! Port of `ichiran/kanji:*kanjidic-path*` (`kanji.lisp:3`).
//!
//! Filesystem path to the kanjidic2.xml dump consumed by
//! `load-kanjidic`. Upstream introduces the symbol via `defvar` with a
//! placeholder and rebinds it per-deployment in a local settings file.
//! The port keeps the upstream placeholder; consumers that need a real
//! path shadow this constant or read it from configuration before
//! invoking the loader.

pub static KANJIDIC_PATH: &str = "e:/dump/kanjidic2.xml";
