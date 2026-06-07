/// Rust-only sidecar within the `ichiran/dict` port module.
/// Has no Lisp counterpart.
///
/// `KaniHintKind` is the closed two-variant enumeration of the
/// keyword tags `(:space :mod)` used by the kana-hint system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KaniHintKind {
    Space,
    Mod,
}
