//! Port of `ichiran/dict:process-word-info` (`dict.lisp:1417`).
//!
//! Post-processes a word-info sequence to fix the 何 (what) reading
//! based on the next word's first kana. Walks adjacent pairs; for
//! each pair where the current `text` is `"何"` and a next word
//! exists, scans the next word's kana strings and picks one of
//! `"なに"` / `"なん"` based on which kana classes the leading
//! characters fall into:
//!
//! - All leading kana fall in the dental / voiced / `n` / `r`
//!   bracket → `"なん"`.
//! - All leading kana are outside that bracket → `"なに"`.
//! - A mix of both → `"なに"`.
//! - Empty kana strings only → no change.
//!
//! The `nan` class is the inline keyword set in `dict.lisp:1426-1430`:
//! the `b/p/d/dz/z/t/n/r` rows minus `ni`. Encoded here as
//! [`is_nan_class`].

use super::word_info_class::{WordInfo, WordInfoKana};
use crate::characters::get_char_class::get_char_class;
use crate::characters::kani_kana_class::KanaClass;

pub fn process_word_info(mut wi_list: Vec<WordInfo>) -> Vec<WordInfo> {
    for i in 0..wi_list.len() {
        if wi_list[i].text != "何" {
            continue;
        }
        let Some(next) = wi_list.get(i + 1) else {
            continue;
        };
        let kana_strings: Vec<&str> = match &next.kana {
            WordInfoKana::Single(s) => vec![s.as_str()],
            WordInfoKana::Multi(v) => v.iter().map(|s| s.as_str()).collect(),
        };
        let mut nani = false;
        let mut nan = false;
        for kana in kana_strings {
            let Some(first_char) = kana.chars().next() else {
                continue;
            };
            let fc_class = get_char_class(first_char);
            if matches!(fc_class, Some(c) if is_nan_class(c)) {
                nan = true;
            } else {
                nani = true;
            }
        }
        let nani_kana = match (nan, nani) {
            (true, true) => Some("なに"),
            (true, false) => Some("なん"),
            (false, true) => Some("なに"),
            (false, false) => None,
        };
        if let Some(s) = nani_kana {
            wi_list[i].kana = WordInfoKana::Single(s.to_string());
        }
    }
    wi_list
}

fn is_nan_class(c: KanaClass) -> bool {
    use KanaClass::*;
    matches!(
        c,
        Ba | Bi | Bu | Be | Bo
            | Pa | Pi | Pu | Pe | Po
            | Da | Dji | Dzu | De | Do
            | Za | Ji | Zu | Ze | Zo
            | Ta | Chi | Tsu | Te | To
            | Na | Nu | Ne | No
            | Ra | Ri | Ru | Re | Ro
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::word_info_class::WordInfoType;

    fn wi(text: &str, kana: &str) -> WordInfo {
        WordInfo {
            kind: WordInfoType::Kanji,
            text: text.to_string(),
            kana: WordInfoKana::Single(kana.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn nan_branch_voiced_t() {
        let list = process_word_info(vec![wi("何", "なに"), wi("で", "で")]);
        assert_eq!(list[0].kana, WordInfoKana::Single("なん".to_string()));
    }

    #[test]
    fn nani_branch_unvoiced_k() {
        let list = process_word_info(vec![wi("何", "なん"), wi("か", "か")]);
        assert_eq!(list[0].kana, WordInfoKana::Single("なに".to_string()));
    }

    #[test]
    fn nani_branch_vowel() {
        let list = process_word_info(vec![wi("何", "なん"), wi("ある", "ある")]);
        assert_eq!(list[0].kana, WordInfoKana::Single("なに".to_string()));
    }

    #[test]
    fn ni_treated_as_nani() {
        let list = process_word_info(vec![wi("何", "なん"), wi("人", "にん")]);
        assert_eq!(list[0].kana, WordInfoKana::Single("なに".to_string()));
    }

    #[test]
    fn no_next_word_unchanged() {
        let list = process_word_info(vec![wi("何", "なん")]);
        assert_eq!(list[0].kana, WordInfoKana::Single("なん".to_string()));
    }

    #[test]
    fn non_target_text_unchanged() {
        let list = process_word_info(vec![wi("猫", "ねこ"), wi("で", "で")]);
        assert_eq!(list[0].kana, WordInfoKana::Single("ねこ".to_string()));
    }

    #[test]
    fn multi_kana_mixed_picks_nani() {
        let mut next_wi = wi("X", "");
        next_wi.kana = WordInfoKana::Multi(vec!["で".to_string(), "か".to_string()]);
        let list = process_word_info(vec![wi("何", "なん"), next_wi]);
        assert_eq!(list[0].kana, WordInfoKana::Single("なに".to_string()));
    }

    #[test]
    fn empty_kana_no_change() {
        let mut next_wi = wi("X", "");
        next_wi.kana = WordInfoKana::Multi(vec![]);
        let list = process_word_info(vec![wi("何", "なん"), next_wi]);
        assert_eq!(list[0].kana, WordInfoKana::Single("なん".to_string()));
    }
}
