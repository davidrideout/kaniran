//! Port of the `ichiran/numbers` Lisp package. Upstream `numbers.lisp`
//! is one short file; the Rust port splits it into three modules
//! mirroring the upstream's internal section order.

pub mod kana_form;
pub mod kanji_form;
pub mod num_class;
