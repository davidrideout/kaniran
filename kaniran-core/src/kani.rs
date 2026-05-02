//! `kani::` — kaniran-specific infrastructure that is *not* a port of
//! any ichiran symbol.
//!
//! Everything under this module is original kaniran code: fixture
//! replay, name translation, and other glue that supports the port but
//! is not part of the ported behavior itself. Ported Lisp functions
//! live in sibling modules (`characters::`, `dict::`, etc.) named
//! after their original Lisp packages.

pub mod fixture;
pub mod naming;
