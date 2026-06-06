//! Port of `ichiran/dict:process-word-info` (`dict.lisp:1417`).
//!
//! Post-processes a word-info sequence to fix the 何 (what) reading
//! based on the next word's first kana, picking `"なん"` when every
//! leading kana falls in the dental / voiced / `n` / `r` bracket and
//! `"なに"` otherwise (empty kana strings leave the reading unchanged).

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
        // dict.lisp:1421-1438 — `(unless (listp kn) (setf kn (list kn)))`
        // wraps a non-list `kn` in a singleton; the inner loop then
        // iterates `kn` at one level. `(char kana 0)` errors with a
        // type-error on a non-string element; we mirror that by
        // panicking on a nested `Multi` entry. `None` entries become
        // length-0 and are skipped via `(when (> (length kana) 0) ...)`.
        // Iterate kn at one level. Lisp's `(unless (listp kn) (setf kn (list kn)))`
        // wraps a non-list element into a singleton; equivalent here: a
        // `Single`/`None` slot wraps to a one-element iteration.
        let singleton: Option<WordInfoKana>;
        let kn_iter: &[Option<WordInfoKana>] = match &next.kana {
            Some(WordInfoKana::Multi(items)) => items.as_slice(),
            other => {
                singleton = other.clone();
                std::slice::from_ref(&singleton)
            }
        };
        let mut nani = false;
        let mut nan = false;
        for entry in kn_iter {
            let kana: &str = match entry {
                Some(WordInfoKana::Single(s)) => s.as_str(),
                None => "",
                Some(WordInfoKana::Multi(_)) => {
                    panic!(
                        "process-word-info: nested Multi inside kana list — upstream `(char list 0)` would type-error"
                    );
                }
            };
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
            wi_list[i].kana = Some(WordInfoKana::Single(s.to_string()));
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
            kana: Some(WordInfoKana::Single(kana.to_string())),
            ..Default::default()
        }
    }

    #[test]
    fn nan_branch_voiced_t() {
        let list = process_word_info(vec![wi("何", "なに"), wi("で", "で")]);
        assert_eq!(list[0].kana, Some(WordInfoKana::Single("なん".to_string())));
    }

    #[test]
    fn nani_branch_unvoiced_k() {
        let list = process_word_info(vec![wi("何", "なん"), wi("か", "か")]);
        assert_eq!(list[0].kana, Some(WordInfoKana::Single("なに".to_string())));
    }

    #[test]
    fn nani_branch_vowel() {
        let list = process_word_info(vec![wi("何", "なん"), wi("ある", "ある")]);
        assert_eq!(list[0].kana, Some(WordInfoKana::Single("なに".to_string())));
    }

    #[test]
    fn ni_treated_as_nani() {
        let list = process_word_info(vec![wi("何", "なん"), wi("人", "にん")]);
        assert_eq!(list[0].kana, Some(WordInfoKana::Single("なに".to_string())));
    }

    #[test]
    fn no_next_word_unchanged() {
        let list = process_word_info(vec![wi("何", "なん")]);
        assert_eq!(list[0].kana, Some(WordInfoKana::Single("なん".to_string())));
    }

    #[test]
    fn non_target_text_unchanged() {
        let list = process_word_info(vec![wi("猫", "ねこ"), wi("で", "で")]);
        assert_eq!(list[0].kana, Some(WordInfoKana::Single("ねこ".to_string())));
    }

    #[test]
    fn multi_kana_mixed_picks_nani() {
        let mut next_wi = wi("X", "");
        next_wi.kana = Some(WordInfoKana::Multi(vec![
            Some(WordInfoKana::Single("で".to_string())),
            Some(WordInfoKana::Single("か".to_string())),
        ]));
        let list = process_word_info(vec![wi("何", "なん"), next_wi]);
        assert_eq!(list[0].kana, Some(WordInfoKana::Single("なに".to_string())));
    }

    #[test]
    fn empty_kana_no_change() {
        let mut next_wi = wi("X", "");
        next_wi.kana = Some(WordInfoKana::Multi(Vec::new()));
        let list = process_word_info(vec![wi("何", "なん"), next_wi]);
        assert_eq!(list[0].kana, Some(WordInfoKana::Single("なん".to_string())));
    }
}
