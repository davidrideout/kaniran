//! Rust-only sidecar: closed enum for the counter suffix tags the
//! Lisp uses inline as `:kan`, `:kango`, `:chuu`.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SuffixKind {
    Kan,
    Kango,
    Chuu,
}
