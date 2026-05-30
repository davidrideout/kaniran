//! Port of `ichiran/dict:find-sticky-positions` (`dict.lisp:990`).
//!
//! Positions where a word can neither start nor end: after a sokuon
//! when the following character is a kana mora, and at any modifier
//! or iteration character unless it sits at the end and would extend
//! the preceding mora's vowel (long vowel mark, or `+a/+i/+u/+e/+o`
//! agreeing with the prior kana's vowel).
//!
//! Returned offsets are **character** positions per CONVENTIONS §4.5.

use crate::characters::kana_class_tables::{
    ITERATION_CHARACTERS, KANA_CHARACTERS, MODIFIER_CHARACTERS,
};
use crate::characters::kana_class_tables::get_char_class;
use crate::characters::kani_kana_class::KanaClass;
use crate::characters::long_vowel_modifier_p::long_vowel_modifier_p;

pub fn find_sticky_positions(str: &str) -> Vec<usize> {
    let chars: Vec<char> = str.chars().collect();
    let str_len = chars.len();
    let mut out = Vec::new();

    for pos in 0..str_len {
        let ch = chars[pos];
        let char_class = get_char_class(ch);

        if char_class == Some(KanaClass::Sokuon)
            && pos != str_len - 1
            && get_char_class(chars[pos + 1]).is_some_and(is_kana_class)
        {
            out.push(pos + 1);
            continue;
        }

        if let Some(cc) = char_class {
            if is_modifier_or_iter_class(cc) {
                let suppress = pos == str_len - 1
                    && (cc == KanaClass::LongVowel
                        || (pos > 0 && long_vowel_modifier_p(cc, chars[pos - 1])));
                if !suppress {
                    out.push(pos);
                }
            }
        }
    }

    out
}

fn is_kana_class(cc: KanaClass) -> bool {
    KANA_CHARACTERS.iter().any(|(k, _)| *k == cc)
}

fn is_modifier_or_iter_class(cc: KanaClass) -> bool {
    MODIFIER_CHARACTERS.iter().any(|(k, _)| *k == cc)
        || ITERATION_CHARACTERS.iter().any(|(k, _)| *k == cc)
}

#[cfg(test)]
mod tests {
    use super::find_sticky_positions;

    #[test]
    fn empty_string() {
        assert_eq!(find_sticky_positions(""), Vec::<usize>::new());
    }

    #[test]
    fn no_stickies_kanji() {
        assert_eq!(find_sticky_positions("食べる"), Vec::<usize>::new());
        assert_eq!(find_sticky_positions("学校"), Vec::<usize>::new());
        assert_eq!(find_sticky_positions("私はその本を読みました"), Vec::<usize>::new());
        assert_eq!(find_sticky_positions("東京特許許可局"), Vec::<usize>::new());
    }

    #[test]
    fn modifier_mid_word() {
        assert_eq!(find_sticky_positions("きゃく"), vec![1]);
        assert_eq!(find_sticky_positions("けーき"), vec![1]);
        assert_eq!(find_sticky_positions("あぁい"), vec![1]);
    }

    #[test]
    fn modifier_at_end_collected_when_no_long_vowel_match() {
        // +YA at end: long_vowel_modifier_p returns false (not in +A/+I/+U/+E/+O).
        assert_eq!(find_sticky_positions("きゃ"), vec![1]);
        // +A after KI: vowels don't agree (KI ends in I), so collected.
        assert_eq!(find_sticky_positions("きぁ"), vec![1]);
        // +O after NI: vowels don't agree, collected.
        assert_eq!(find_sticky_positions("にぉ"), vec![1]);
        // Modifier after non-kana char (prev has no KanaClass): collected.
        assert_eq!(find_sticky_positions("漢ぁ"), vec![1]);
        // +WA at end: long_vowel_modifier_p false for PlusWa, collected.
        assert_eq!(find_sticky_positions("かゎ"), vec![1]);
    }

    #[test]
    fn modifier_at_end_suppressed_when_long_vowel_matches() {
        // +A after KA: vowel agrees, suppressed.
        assert_eq!(find_sticky_positions("かぁ"), Vec::<usize>::new());
        // +I after NI: vowel agrees, suppressed.
        assert_eq!(find_sticky_positions("にぃ"), Vec::<usize>::new());
    }

    #[test]
    fn long_vowel_at_end_suppressed() {
        assert_eq!(find_sticky_positions("かー"), Vec::<usize>::new());
        assert_eq!(find_sticky_positions("あー"), Vec::<usize>::new());
    }

    #[test]
    fn long_vowel_at_start_collected() {
        assert_eq!(find_sticky_positions("ーあ"), vec![0]);
    }

    #[test]
    fn modifier_first_char_not_last_collected() {
        // Modifier at pos 0 with str_len > 1: not last, so lvmp branch irrelevant.
        assert_eq!(find_sticky_positions("ぁか"), vec![0]);
    }

    #[test]
    fn modifier_lone_char_collected() {
        // pos==0, last, but `(> pos 0)` is false, so lvmp branch short-circuits.
        assert_eq!(find_sticky_positions("ぁ"), vec![0]);
        // Same — PlusWa at lone position.
        assert_eq!(find_sticky_positions("ゎ"), vec![0]);
    }

    #[test]
    fn sokuon_mid_word_collects_pos_plus_one() {
        assert_eq!(find_sticky_positions("いっぱい"), vec![2]);
        assert_eq!(find_sticky_positions("ニッポン"), vec![2]);
        assert_eq!(find_sticky_positions("ニッキ"), vec![2]);
        assert_eq!(find_sticky_positions("っあっい"), vec![1, 3]);
    }

    #[test]
    fn sokuon_at_end_not_collected() {
        assert_eq!(find_sticky_positions("いっ"), Vec::<usize>::new());
        assert_eq!(find_sticky_positions("っ"), Vec::<usize>::new());
    }

    #[test]
    fn sokuon_followed_by_non_kana_not_collected() {
        assert_eq!(find_sticky_positions("っ漢"), Vec::<usize>::new());
        assert_eq!(find_sticky_positions("っX"), Vec::<usize>::new());
    }

    #[test]
    fn iteration_characters() {
        // Both iter marks: pos 0 not last (collect 0), pos 1 last & not long-vowel & lvmp false → collect 1.
        assert_eq!(find_sticky_positions("ゝゞ"), vec![0, 1]);
        // ゝ at end after い: lvmp false, long-vowel false → collected.
        assert_eq!(find_sticky_positions("いゝ"), vec![1]);
    }

    #[test]
    fn single_kana_char_no_sticky() {
        assert_eq!(find_sticky_positions("あ"), Vec::<usize>::new());
        assert_eq!(find_sticky_positions("いろは"), Vec::<usize>::new());
    }

    #[test]
    fn combined_sokuon_and_modifier() {
        assert_eq!(find_sticky_positions("きゃっき"), vec![1, 3]);
    }
}
