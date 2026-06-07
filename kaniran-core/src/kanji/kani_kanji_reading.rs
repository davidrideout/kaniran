//! Rust-only sidecar — single typed shape for a kanji reading variant
//! (`reading`, `type`, optional `tag`, optional `gem`), unifying the
//! heterogeneous 2- to 4-element reading lists upstream emits.

use super::readings::{ReadingAlternative, ReadingTag};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KanjiReading {
    pub reading: String,
    pub r#type: String,
    pub tag: Option<ReadingTag>,
    pub gem: Option<String>,
}

impl From<ReadingAlternative> for KanjiReading {
    fn from(a: ReadingAlternative) -> Self {
        Self {
            reading: a.reading,
            r#type: a.r#type,
            tag: Some(a.tag),
            gem: a.gem,
        }
    }
}
