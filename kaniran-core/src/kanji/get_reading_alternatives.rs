//! Port of `ichiran/kanji:get-reading-alternatives` (`kanji.lisp:218`).
//!
//! Expands a single `(reading, type)` kanjidic2 reading into the set of
//! variant readings considered by the kanji-to-kana matcher. The base
//! entry is always emitted; an additional entry with the trailing mora
//! geminated (`っ`) is emitted when `type` is `"ja_on"` and the last
//! character is one of つ/く/き/ち. With the `rendaku` flag set, every
//! entry above is then duplicated twice — once dakuten-voiced, once
//! handakuten-voiced — and appended after the originals.
//!
//! Diverges from the upstream lambda list `(reading type &key rendaku)`
//! by:
//!
//! - taking `rendaku` as a plain `bool` per CONVENTIONS §4.4. The
//!   keyword is binary (absent ↔ nil ↔ off, `t` ↔ on) and the only
//!   in-tree caller (`get-normal-readings` `kanji.lisp:235`) threads its
//!   own `rendaku` parameter through unchanged;
//! - returning `Vec<ReadingAlternative>` rather than a heterogeneous
//!   3-or-4 element cons list. Each upstream entry is `(rd type tag
//!   gem?)` where `tag` is `nil` (base/geminate) or `:rendaku`, and
//!   `gem` is absent on bare bases and a single-char string on
//!   geminated/rendaku-of-geminated entries; the `tag` field is the
//!   §4.3 enum for the closed `nil | :rendaku` shape, and the struct
//!   names the four positions so consumers don't destructure
//!   positionally.
//!
//! Geminate/rendaku take `&mut String` per their own `:fresh nil` port
//! (CONVENTIONS §4.4); the upstream `:fresh t` semantic at every
//! callsite here is reproduced by cloning before the call.

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
