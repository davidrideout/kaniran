//! Port of `ichiran:rmap-item` (`deromanize.lisp:5`).
//!
//! In-memory record carrying one row of `data/romaji-map.csv` — a
//! romaji-to-kana rule. `text` is the romaji prefix this rule
//! consumes; `kana` is the kana fragment it emits; `next` is the
//! romaji fragment to prepend back onto the unconsumed input after
//! the rule fires. `next` is non-empty only for doubled-consonant
//! gemination rules — e.g. `bb` consumes the input `bb`, emits `っ`,
//! and re-emits `b` so the next pass picks up the second consonant
//! (CSV row `bb\tっ\tb` at `data/romaji-map.csv`). For single-rule
//! entries CSV column 3 is absent and `next` is `None`. Used by
//! `apply-rmap-item` (`deromanize.lisp:35`), which prepends
//! `(or (rmi-next rmi) "")` to the rest of the romaji string.

#[derive(Debug, Clone)]
pub struct RmapItem {
    pub text: String,
    pub kana: String,
    pub next: Option<String>,
}
