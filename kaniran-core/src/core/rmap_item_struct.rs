//! Port of `ichiran:rmap-item` (`deromanize.lisp:5`).
//!
//! One romaji-to-kana rule from `data/romaji-map.csv`. `next` is non-empty
//! only for doubled-consonant gemination rows — e.g. `bb` emits `っ` and
//! re-emits `b` so the next pass picks up the second consonant.

#[derive(Debug, Clone)]
pub struct RmapItem {
    pub text: String,
    pub kana: String,
    pub next: Option<String>,
}
