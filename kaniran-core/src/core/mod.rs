//! Port of the bare `ichiran` package — romanization and
//! deromanization. Upstream is `romanize.lisp` + `deromanize.lisp`;
//! the Rust port splits them into five modules mirroring the
//! upstream's internal section order.

pub mod deromanize;
pub mod methods;
pub mod romanize;
pub mod rules;
pub mod tables;
