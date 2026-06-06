//! Port of `ichiran/kanji:get-reading-alternatives` (`kanji.lisp:218`).
//!
//! Expands a single `(reading, type)` kanjidic2 reading into its variant
//! readings. The base entry is always emitted; a trailing-mora geminated
//! (`っ`) entry is added when `type` is `"ja_on"` and the last character
//! is one of つ/く/き/ち; with `rendaku` set, every entry above is also
//! duplicated dakuten- and handakuten-voiced.

use crate::characters::geminate::geminate;
use crate::characters::rendaku::{rendaku as voice_rendaku, Voicing};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadingTag {
    Plain,
    Rendaku,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadingAlternative {
    pub reading: String,
    pub r#type: String,
    pub tag: ReadingTag,
    pub gem: Option<String>,
}

pub fn get_reading_alternatives(
    reading: &str,
    r#type: &str,
    rendaku: bool,
) -> Vec<ReadingAlternative> {
    let chars: Vec<char> = reading.chars().collect();
    let mut lst: Vec<ReadingAlternative> = vec![ReadingAlternative {
        reading: reading.to_string(),
        r#type: r#type.to_string(),
        tag: ReadingTag::Plain,
        gem: None,
    }];

    if chars.len() > 1 && r#type == "ja_on" {
        let last = chars[chars.len() - 1];
        if matches!(last, 'つ' | 'く' | 'き' | 'ち') {
            let mut copy = reading.to_string();
            geminate(&mut copy);
            lst.push(ReadingAlternative {
                reading: copy,
                r#type: r#type.to_string(),
                tag: ReadingTag::Plain,
                gem: Some(last.to_string()),
            });
        }
    }

    if rendaku {
        let snapshot = lst.clone();
        for entry in &snapshot {
            let mut rd = entry.reading.clone();
            voice_rendaku(&mut rd, Voicing::Dakuten);
            lst.push(ReadingAlternative {
                reading: rd,
                r#type: r#type.to_string(),
                tag: ReadingTag::Rendaku,
                gem: entry.gem.clone(),
            });
            let mut rd_h = entry.reading.clone();
            voice_rendaku(&mut rd_h, Voicing::Handakuten);
            lst.push(ReadingAlternative {
                reading: rd_h,
                r#type: r#type.to_string(),
                tag: ReadingTag::Rendaku,
                gem: entry.gem.clone(),
            });
        }
    }

    lst
}
