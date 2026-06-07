//! Port of `ichiran/kanji:make-rmap` (`kanji.lisp:273`).
//!
//! Walks `str` once and produces one [`RmapEntry`] per character:
//! `RmapEntry::NonKanji(c)` for non-kanji passthroughs, or
//! `RmapEntry::Readings(...)` listing the candidate readings for
//! each kanji-ish character. The candidate list comes from
//! [`super::get_normal_readings`] for normal kanji; from a
//! per-character literal for the abbreviation marks ヶ and 〆; and
//! propagates the previous kanji's readings (with `rendaku=t`) for
//! the iteration mark 々.

use super::get_normal_readings::get_normal_readings;
use super::get_reading_alternatives::ReadingTag;
use super::kani_kanji_reading::KanjiReading;
use crate::characters::constants::KANJI_REGEX;
use crate::conn::kani_context::KaniranContext;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RmapEntry {
    NonKanji(char),
    Readings(Vec<KanjiReading>),
}

pub async fn make_rmap(
    ctx: &KaniranContext,
    str: &str,
) -> Result<Vec<RmapEntry>, sqlx::Error> {
    let chars: Vec<char> = str.chars().collect();
    let mut out: Vec<RmapEntry> = Vec::with_capacity(chars.len());
    let mut prev_kanji: Option<char> = None;

    for (start, &c) in chars.iter().enumerate() {
        if is_kanji(c) {
            let entry = match c {
                '々' => {
                    // kanji.lisp:279-281 (eql char #\々) — propagate prev with rendaku
                    let readings = match prev_kanji {
                        Some(prev) => get_normal_readings(ctx, prev, true).await?,
                        None => Vec::new(),
                    };
                    prev_kanji = None;
                    RmapEntry::Readings(readings)
                }
                'ヶ' => {
                    // kanji.lisp:282-284 (eql char #\ヶ)
                    prev_kanji = None;
                    RmapEntry::Readings(vec![
                        KanjiReading {
                            reading: "か".to_string(),
                            r#type: "ja_on".to_string(),
                            tag: None,
                            gem: None,
                        },
                        KanjiReading {
                            reading: "が".to_string(),
                            r#type: "abbr".to_string(),
                            tag: None,
                            gem: None,
                        },
                    ])
                }
                '〆' => {
                    // kanji.lisp:285-287 (eql char #\〆) — prev-kanji becomes 締
                    prev_kanji = Some('締');
                    RmapEntry::Readings(vec![
                        KanjiReading {
                            reading: "しめ".to_string(),
                            r#type: "ja_kun".to_string(),
                            tag: None,
                            gem: None,
                        },
                        KanjiReading {
                            reading: "じめ".to_string(),
                            r#type: "ja_kun".to_string(),
                            tag: Some(ReadingTag::Rendaku),
                            gem: None,
                        },
                    ])
                }
                _ => {
                    // kanji.lisp:288-289 (t (setf prev-kanji char) ...)
                    prev_kanji = Some(c);
                    let rendaku = start > 0;
                    RmapEntry::Readings(get_normal_readings(ctx, c, rendaku).await?)
                }
            };
            out.push(entry);
        } else {
            // kanji.lisp:290 (else collect char)
            out.push(RmapEntry::NonKanji(c));
        }
    }
    Ok(out)
}

/// `*kanji-regex*` matches a single character against
/// `[々ヶ〆一-龯]`. The regex engine is overkill for one-codepoint
/// membership; a direct char comparison is equivalent.
fn is_kanji(c: char) -> bool {
    debug_assert_eq!(KANJI_REGEX, "[々ヶ〆一-龯]");
    matches!(c, '々' | 'ヶ' | '〆' | '\u{4e00}'..='\u{9faf}')
}
