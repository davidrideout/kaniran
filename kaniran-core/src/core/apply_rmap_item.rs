//! Port of `ichiran:apply-rmap-item` (`deromanize.lisp:37`).
//!
//! Builds the [`KanaRepresentation`] for applying one romaji rule
//! `rmi` to input `s`: the rule's kana is the canonical kana and the
//! base pattern, with a trailing `う?` appended when the consumed
//! romaji could be a long vowel; `rest` is the rule's `next` fragment
//! (or empty) prepended to `s` past the consumed prefix. `branch`
//! takes the struct default (0).

use super::kana_representation_struct::KanaRepresentation;
use super::possible_long_vowel_p::possible_long_vowel_p;
use super::rmap_item_struct::RmapItem;

pub fn apply_rmap_item(s: &str, rmi: &RmapItem) -> KanaRepresentation {
    let kana = &rmi.kana;
    KanaRepresentation {
        canonical: kana.clone(),
        pattern: if possible_long_vowel_p(&rmi.text).is_some() {
            format!("{kana}う?")
        } else {
            kana.clone()
        },
        rest: format!(
            "{}{}",
            rmi.next.as_deref().unwrap_or(""),
            s.chars().skip(rmi.text.chars().count()).collect::<String>()
        ),
        branch: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rmi(text: &str, kana: &str, next: Option<&str>) -> RmapItem {
        RmapItem {
            text: text.to_string(),
            kana: kana.to_string(),
            next: next.map(str::to_string),
        }
    }

    #[test]
    fn apply_rmap_item_fixtures() {
        // REPL fixtures (.103, ichiran::apply-rmap-item), 2026-05-26.
        // (s, rmi, expected canonical/pattern/rest). Rows cover the
        // long-vowel branch (う? appended), the plain branch, and the
        // doubled-consonant `next` re-emission.
        let cases: &[(&str, RmapItem, (&str, &str, &str))] = &[
            // long-vowel branch: text ends in o/u -> pattern gets う?
            ("konnichiwa", rmi("ko", "こ", None), ("こ", "こう?", "nnichiwa")),
            ("ohayou", rmi("o", "お", None), ("お", "おう?", "hayou")),
            ("toukyou", rmi("to", "と", None), ("と", "とう?", "ukyou")),
            // plain branch: text ends in other char
            ("katana", rmi("ka", "か", None), ("か", "か", "tana")),
            ("shinkansen", rmi("shi", "し", None), ("し", "し", "nkansen")),
            ("nagoya", rmi("na", "な", None), ("な", "な", "goya")),
            // gemination: next re-emitted before the unconsumed tail. Inputs
            // are the remaining strings romaji-next passes mid-word when
            // deromanizing 結婚(kekkon)/一杯(ippai)/学校(gakkou).
            ("kkon", rmi("kk", "っ", Some("k")), ("っ", "っ", "kon")),
            ("ppai", rmi("pp", "っ", Some("p")), ("っ", "っ", "pai")),
            ("kkou", rmi("kk", "っ", Some("k")), ("っ", "っ", "kou")),
        ];
        for (s, item, (canonical, pattern, rest)) in cases {
            let got = apply_rmap_item(s, item);
            assert_eq!(got.canonical, *canonical, "canonical, s={s:?} rmi={item:?}");
            assert_eq!(got.pattern, *pattern, "pattern, s={s:?} rmi={item:?}");
            assert_eq!(got.rest, *rest, "rest, s={s:?} rmi={item:?}");
            assert_eq!(got.branch, 0, "branch, s={s:?} rmi={item:?}");
        }
    }
}
