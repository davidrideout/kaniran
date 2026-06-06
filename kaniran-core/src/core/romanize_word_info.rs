//! Port of `ichiran:romanize-word-info` (`romanize.lisp:248-255`).
//!
//! Romanizes each kana reading of a word-info, or strips hints under the
//! `:kana` method.

use super::kani_romanize_method::KaniRomanizeMethod;
use super::romanize_word::romanize_word;
use crate::dict::map_word_info_kana::map_word_info_kana;
use crate::dict::strip_hints::strip_hints;
use crate::dict::word_info_class::{WordInfo, WordInfoKana};

pub fn romanize_word_info(word_info: &WordInfo, method: KaniRomanizeMethod<'_>) -> String {
    let orig_text = &word_info.text;
    match method {
        KaniRomanizeMethod::Kana => {
            map_word_info_kana(|wk| strip_hints(strip_hints_input(wk)), word_info, "/")
        }
        KaniRomanizeMethod::Method(method) => map_word_info_kana(
            |wk| romanize_word(romanize_word_input(wk), method, Some(orig_text), false),
            word_info,
            "/",
        ),
    }
}

/// The kana element `map-word-info-kana` binds to `wk` for the romanize-word
/// closure. Every element a segmenter word-info carries is a reading string
/// (corpus: 928764/928764 Single). A nil element romanizes to "" upstream
/// (`(romanize-word nil … :normalize nil)` ≡ `(romanize-word "" …)`, REPL
/// 2026-05-24); a nested list is a type error in romanize-word's
/// process-hints, so it panics.
fn romanize_word_input(wk: &Option<WordInfoKana>) -> &str {
    match wk {
        Some(WordInfoKana::Single(reading)) => reading,
        None => "",
        Some(WordInfoKana::Multi(_)) => {
            panic!("romanize-word-info: nested-list kana element (upstream romanize-word type error)")
        }
    }
}

/// As [`romanize_word_input`] but for the `:kana` (strip-hints) closure.
/// Here a nil element errors upstream rather than yielding "": `(strip-hints
/// nil)` returns nil and `map-word-info-kana`'s `simplify-reading-list` then
/// fails on it (REPL 2026-05-24); a nested list errors the same way. Both
/// panic; only reading strings occur in practice.
fn strip_hints_input(wk: &Option<WordInfoKana>) -> &str {
    match wk {
        Some(WordInfoKana::Single(reading)) => reading,
        _ => panic!(
            "romanize-word-info :kana: non-string kana element (upstream simplify-reading-list error)"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::_star_hepburn_traditional_star_::hepburn_traditional;
    use crate::core::generic_romanization_class::RomanizationMethod;
    use crate::dict::_star_kana_hint_mod_star_::KANA_HINT_MOD;
    use crate::dict::word_info_class::WordInfoType;

    fn wi(text: &str, kana: Option<WordInfoKana>) -> WordInfo {
        WordInfo {
            kind: WordInfoType::Kana,
            text: text.to_string(),
            kana,
            ..Default::default()
        }
    }

    fn single(reading: &str) -> Option<WordInfoKana> {
        Some(WordInfoKana::Single(reading.to_string()))
    }

    #[test]
    fn romanize_word_info_fixtures() {
        // REPL fixtures (.103, ichiran:romanize-word-info via simple-segment),
        // 2026-05-24. Each row: (text, kana, traditional, :kana).
        let traditional = KaniRomanizeMethod::Method(RomanizationMethod::TraditionalHepburn(
            hepburn_traditional(),
        ));
        let kana = KaniRomanizeMethod::Kana;

        // 教会 — single-string kana, long-vowel macrons under traditional.
        let kyoukai = wi("教会", single("きょうかい"));
        assert_eq!(romanize_word_info(&kyoukai, traditional), "kyōkai");
        assert_eq!(romanize_word_info(&kyoukai, kana), "きょうかい");

        // 小学校 — geminate + macron.
        let shougakkou = wi("小学校", single("しょうがっこう"));
        assert_eq!(romanize_word_info(&shougakkou, traditional), "shōgakkō");
        assert_eq!(romanize_word_info(&shougakkou, kana), "しょうがっこう");

        // topic-particle は — kana carries a *kana-hint-mod* sentinel before
        // は: traditional romanizes to "wa" (process-hints), :kana strips
        // the sentinel back to "は".
        let hinted_ha = wi("は", single(&format!("{}は", KANA_HINT_MOD)));
        assert_eq!(romanize_word_info(&hinted_ha, traditional), "wa");
        assert_eq!(romanize_word_info(&hinted_ha, kana), "は");

        // はた — list kana with one reading: map over the list, simplify,
        // join with "/".
        let hata = wi("はた", Some(WordInfoKana::Multi(vec![single("はた")])));
        assert_eq!(romanize_word_info(&hata, traditional), "hata");
        assert_eq!(romanize_word_info(&hata, kana), "はた");

        // text (= original-spelling) drives r-special: a lone small tsu /
        // long-vowel bar romanizes to "!" / "~" under a method, and strips
        // to itself under :kana.
        let tsu = wi("っ", single("っ"));
        assert_eq!(romanize_word_info(&tsu, traditional), "!");
        assert_eq!(romanize_word_info(&tsu, kana), "っ");
        let bar = wi("ー", single("ー"));
        assert_eq!(romanize_word_info(&bar, traditional), "~");
        assert_eq!(romanize_word_info(&bar, kana), "ー");
    }

    #[test]
    fn romanize_word_info_method_arm_nil_element() {
        // REPL fixture (.103): a kana list with a nil element romanizes the
        // nil to "" under a method ((romanize-word nil …) -> ""), giving
        // "a/" for kana=("あ" nil).
        let traditional = KaniRomanizeMethod::Method(RomanizationMethod::TraditionalHepburn(
            hepburn_traditional(),
        ));
        let wi_nil_elem = wi("x", Some(WordInfoKana::Multi(vec![single("あ"), None])));
        assert_eq!(romanize_word_info(&wi_nil_elem, traditional), "a/");
    }

    #[test]
    #[should_panic]
    fn romanize_word_info_kana_arm_nil_element_errors() {
        // REPL fixture (.103): the :kana path errors on a nil list element
        // ((strip-hints nil) -> nil, then simplify-reading-list fails).
        let wi_nil_elem = wi("x", Some(WordInfoKana::Multi(vec![single("あ"), None])));
        romanize_word_info(&wi_nil_elem, KaniRomanizeMethod::Kana);
    }

    #[test]
    #[should_panic]
    fn romanize_word_info_nested_element_errors() {
        // REPL fixture (.103): a nested-list kana element is a type error
        // under a method (romanize-word on a list).
        let traditional = KaniRomanizeMethod::Method(RomanizationMethod::TraditionalHepburn(
            hepburn_traditional(),
        ));
        let wi_nested = wi(
            "x",
            Some(WordInfoKana::Multi(vec![Some(WordInfoKana::Multi(vec![single("あ")]))])),
        );
        romanize_word_info(&wi_nested, traditional);
    }

    #[test]
    fn romanize_word_info_nil_kana() {
        // REPL fixture (.103): a word-info with kana=nil takes the list
        // branch (`(listp nil)` is true) and yields "".
        let traditional = KaniRomanizeMethod::Method(RomanizationMethod::TraditionalHepburn(
            hepburn_traditional(),
        ));
        let empty = wi("x", None);
        assert_eq!(romanize_word_info(&empty, traditional), "");
        assert_eq!(romanize_word_info(&empty, KaniRomanizeMethod::Kana), "");
    }
}
