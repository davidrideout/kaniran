//! Port of `ichiran/dict:conjugation-rule` (`dict-load.lisp:262`).
//!
//! In-memory record carrying one row of `conjo.csv` — the conjugation
//! rules table mapping a (part-of-speech, conjugation-id, neg, fml,
//! onum) key to the stem index and three okurigana / euphonic
//! fragments (`okuri`, `euphr`, `euphk`) used to assemble the
//! conjugated form.

#[derive(Debug, Clone)]
pub struct ConjugationRule {
    pub pos: i32,
    pub conj: i32,
    pub neg: bool,
    pub fml: bool,
    pub onum: i32,
    pub stem: i32,
    pub okuri: String,
    pub euphr: String,
    pub euphk: String,
}
