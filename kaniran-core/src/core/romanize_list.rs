//! Port of `ichiran:romanize-list` (`romanize.lisp:205-208`).
//!
//! Romanizes a character-class list: expand iteration markers, fold
//! modifiers into a tree, romanize it, then simplify per the method.

use super::generic_romanization_class::RomanizationMethod;
use super::kani_cc_item::CcItem;
use super::process_iteration_characters::process_iteration_characters;
use super::process_modifiers::process_modifiers;
use super::r_simplify::r_simplify;
use super::romanize_core::romanize_core;

pub fn romanize_list(cc_list: &[CcItem], method: RomanizationMethod<'_>) -> String {
    let cc_tree = process_modifiers(&process_iteration_characters(cc_list));
    r_simplify(method, &romanize_core(method, &cc_tree))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::generic_hepburn_class::GenericHepburn;
    use crate::core::get_character_classes::get_character_classes;
    use crate::core::kunrei_siki_class::KunreiSiki;
    use crate::core::simplified_hepburn_class::SimplifiedHepburn;
    use crate::core::traditional_hepburn_class::TraditionalHepburn;

    #[test]
    fn romanize_list_fixtures() {
        // REPL fixtures (.103, (romanize-list (get-character-classes W) :method M)),
        // 2026-05-24. Words drawn from common Japanese vocabulary.
        let generic = GenericHepburn::new();
        let simple = SimplifiedHepburn::new(vec!["oo", "o", "ou", "o", "uu", "u"]);
        let traditional = TraditionalHepburn::new();
        let kunrei = KunreiSiki::new();
        let basic = RomanizationMethod::GenericHepburn(&generic);
        let simple = RomanizationMethod::SimplifiedHepburn(&simple);
        let traditional = RomanizationMethod::TraditionalHepburn(&traditional);
        let kunrei = RomanizationMethod::KunreiSiki(&kunrei);
        // (word, method, expected).
        let cases: &[(&str, RomanizationMethod, &str)] = &[
            ("きっぷ", traditional, "kippu"),
            ("きっぷ", kunrei, "kippu"),
            ("まっちゃ", traditional, "matcha"),
            ("まっちゃ", kunrei, "mattya"),
            ("コーヒー", traditional, "kohi"),
            ("しゃしん", basic, "shashin"),
            ("しゃしん", kunrei, "syasin"),
            ("がっこう", traditional, "gakkō"),
            ("がっこう", basic, "gakkou"),
            ("がっこう", kunrei, "gakkô"),
            ("がっこう", simple, "gakko"),
            ("こんにちは", traditional, "konnichiha"),
            ("こんにちは", kunrei, "konnitiha"),
            ("とうきょう", traditional, "tōkyō"),
            ("とうきょう", basic, "toukyou"),
            ("とうきょう", kunrei, "tôkyô"),
            ("とうきょう", simple, "tokyo"),
            ("ありがとう", traditional, "arigatō"),
            ("しんぶん", traditional, "shimbun"),
            ("しんぶん", basic, "shinbun"),
            ("しんぶん", kunrei, "sinbun"),
        ];
        for (word, method, expected) in cases {
            let cc_list = get_character_classes(word);
            assert_eq!(&romanize_list(&cc_list, *method), expected, "word={word}");
        }
    }
}
