//! Port of `ichiran:kana-representation` (`deromanize.lisp:23`).
//!
//! One branch of the deromanizer's candidate tree.

#[derive(Debug, Clone, Default)]
pub struct KanaRepresentation {
    pub canonical: String,
    pub pattern: String,
    pub rest: String,
    pub branch: i32,
}
