use super::kani_cc_item::CcItem;
use super::kani_cc_tree::CcTree;
use super::kani_romanize_method::KaniRomanizeMethod;
use super::methods::RomanizationMethod;
use super::rules::{
    process_iteration_characters, process_modifiers, r_apply, r_base, r_simplify, r_special,
};
use crate::characters::char_class::{basic_split, SegmentKind};
use crate::characters::helpers::char_class_hash;
use crate::characters::kana::{normalize, NormalizationContext};
use crate::conn::kani_context::KaniranContext;
use crate::dict::dict_segment::dict_segment;
use crate::dict::map_word_info_kana::map_word_info_kana;
use crate::dict::split::hint::process_hints;
use crate::dict::simple_segment::simple_segment;
use crate::dict::split::hint::strip_hints;
use crate::dict::word_info_class::{WordInfo, WordInfoKana};
use crate::dict::word_info_str::word_info_str;
use unicode_properties::{GeneralCategory, GeneralCategoryGroup, UnicodeGeneralCategory};

/// Port of `ichiran:get-character-classes` (`romanize.lisp:5-7`).
///
/// Maps each character of `word` to its character class, falling back to
/// the glyph itself when it is not kana.
pub fn get_character_classes(word: &str) -> Vec<CcItem> {
    word.chars()
        .map(|char| match char_class_hash().get(&char) {
            Some(&class) => CcItem::Class(class),
            None => CcItem::Char(char),
        })
        .collect()
}

/// Port of `ichiran:leftmost-atom` (`romanize.lisp:27-29`).
///
/// Descends the leftmost branch of a character-class tree and returns
/// the first atom it reaches (`None` for an empty list or nil leaf).
pub fn leftmost_atom(cc_list: &[CcTree]) -> Option<CcItem> {
    match cc_list.first() {
        // (car nil) is nil and (atom nil) is true -> return nil.
        None | Some(CcTree::Nil) => None,
        Some(CcTree::Atom(item)) => Some(*item),
        Some(CcTree::Node(_, rest)) => leftmost_atom(rest),
    }
}

/// Port of `ichiran:romanize-core` (`romanize.lisp:31-37`).
///
/// Walks a character-class tree, concatenating each node's romanization:
/// `nil` drops out, a raw character passes through unchanged, a mora class
/// goes through `r-base`, and a modifier node through `r-apply`.
pub fn romanize_core(method: RomanizationMethod<'_>, cc_tree: &[CcTree]) -> String {
    let mut out = String::new();
    for item in cc_tree {
        match item {
            // (null item)
            CcTree::Nil => {}
            // (characterp item) (princ item out)
            CcTree::Atom(CcItem::Char(character)) => out.push(*character),
            // (atom item) (princ (r-base method item) out)
            CcTree::Atom(CcItem::Class(class)) => out.push_str(&r_base(method, *class)),
            // (listp item) (princ (r-apply (car item) method (cdr item)) out)
            CcTree::Node(modifier, tail) => out.push_str(&r_apply(*modifier, method, tail)),
        }
    }
    out
}

/// Port of `ichiran:romanize-list` (`romanize.lisp:205-208`).
///
/// Romanizes a character-class list: expand iteration markers, fold
/// modifiers into a tree, romanize it, then simplify per the method.
pub fn romanize_list(cc_list: &[CcItem], method: RomanizationMethod<'_>) -> String {
    let cc_tree = process_modifiers(&process_iteration_characters(cc_list));
    r_simplify(method, &romanize_core(method, &cc_tree))
}

/// Port of `ichiran:romanize-word` (`romanize.lisp:224-230`).
///
/// Romanizes a single word: returns the `r-special` mapping when one
/// applies, otherwise processes hints and romanizes its character
/// classes.
pub fn romanize_word(
    word: &str,
    method: RomanizationMethod<'_>,
    original_spelling: Option<&str>,
    normalize: bool,
) -> String {
    // (when normalize (setf word (normalize word))) — normalize is called
    // with no :context, i.e. NormalizationContext::Default.
    let normalized;
    let word: &str = if normalize {
        normalized = crate::characters::kana::normalize(word, NormalizationContext::Default);
        &normalized
    } else {
        word
    };
    // (or (r-special method (or original-spelling word)) …) — empty
    // original-spelling ("") is truthy in CL, so unwrap_or only falls back
    // to word when the keyword is absent.
    if let Some(special) = r_special(method, original_spelling.unwrap_or(word)) {
        return special;
    }
    let word = process_hints(word);
    romanize_list(&get_character_classes(&word), method)
}

/// Port of `ichiran:romanize-word-geo` (`romanize.lisp:232-233`).
///
/// Romanizes `input` (normalized) and capitalizes the result, for place
/// names.
pub fn romanize_word_geo(input: &str, method: RomanizationMethod<'_>) -> String {
    string_capitalize(&romanize_word(input, method, None, true))
}

/// `string-capitalize` (CL builtin): upcase the first cased character of
/// every alphanumeric-delimited word and downcase the rest; a
/// non-alphanumeric character passes through unchanged and starts a new
/// word.
fn string_capitalize(string: &str) -> String {
    let mut out = String::with_capacity(string.len());
    let mut newword = true;
    for char in string.chars() {
        if !romanize_word_geo_alphanumericp(char) {
            out.push(char);
            newword = true;
        } else if newword {
            out.extend(char.to_uppercase());
            newword = false;
        } else {
            out.extend(char.to_lowercase());
        }
    }
    out
}

/// `(romanize_word_geo_alphanumericp char)` — letter categories (Lu, Ll, Lt, Lm, Lo) or
/// decimal-number (Nd). Std `char::is_alphanumeric` diverges by also
/// counting Nl and No, which `romanize_word_geo_alphanumericp` rejects.
fn romanize_word_geo_alphanumericp(char: char) -> bool {
    char.general_category_group() == GeneralCategoryGroup::Letter
        || char.general_category() == GeneralCategory::DecimalNumber
}

/// Port of `ichiran:join-parts` (`romanize.lisp:235-246`).
///
/// Concatenates `parts`, inserting a single space before a part that
/// begins with an alphanumeric character when the running output did not
/// already end in whitespace. Empty parts are skipped entirely.
pub fn join_parts<S: AsRef<str>>(parts: &[S]) -> String {
    let mut out = String::new();
    let mut last_space = true;
    for part in parts {
        let part = part.as_ref();
        let chars: Vec<char> = part.chars().collect();
        let len = chars.len();
        // romanize.lisp:240-243
        if len != 0 && !last_space && join_parts_alphanumericp(chars[0]) {
            out.push(' ');
        }
        out.push_str(part);
        // romanize.lisp:245-246
        if len != 0 {
            last_space = chars[len - 1].is_whitespace();
        }
    }
    out
}

/// `(join_parts_alphanumericp char)` — Lisp `alpha-char-p` (letter categories Lu,
/// Ll, Lt, Lm, Lo) or `digit-char-p` radix 10 (decimal-number category
/// Nd). Std `char::is_alphanumeric` diverges by also counting Nl and No
/// (roman numerals, circled / superscript numbers) that `join_parts_alphanumericp`
/// rejects, so the general category is consulted directly.
fn join_parts_alphanumericp(char: char) -> bool {
    char.general_category_group() == GeneralCategoryGroup::Letter
        || char.general_category() == GeneralCategory::DecimalNumber
}

/// Port of `ichiran:romanize-word-info` (`romanize.lisp:248-255`).
///
/// Romanizes each kana reading of a word-info, or strips hints under the
/// `:kana` method.
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
            panic!(
                "romanize-word-info: nested-list kana element (upstream romanize-word type error)"
            )
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

/// Port of `ichiran:romanize` (`romanize.lisp:257-271`).
///
/// Romanizes `input` and joins the parts into one string; with `with_info`
/// also returns each romanization paired with its word-info string.
pub async fn romanize(
    ctx: &KaniranContext,
    input: &str,
    method: KaniRomanizeMethod<'_>,
    with_info: bool,
) -> Result<(String, Vec<(String, String)>), sqlx::Error> {
    // (normalize input :context method) — characters.lisp:230 tests (eql context :kana)
    let context = match method {
        KaniRomanizeMethod::Kana => NormalizationContext::Kana,
        KaniRomanizeMethod::Method(_) => NormalizationContext::Default,
    };
    let input = normalize(input, context);
    let mut definitions: Vec<(String, String)> = Vec::new();
    let mut parts: Vec<String> = Vec::new();
    for (split_type, split_text) in basic_split(&input) {
        if split_type == SegmentKind::Word {
            for word in simple_segment(ctx, &split_text, None).await? {
                let rom = romanize_word_info(&word, method);
                if with_info {
                    definitions.push((rom.clone(), word_info_str(ctx, &word).await?));
                }
                parts.push(rom);
            }
        } else {
            parts.push(split_text);
        }
    }
    // (nreverse definitions) — push-to-back already collects in encounter order
    Ok((join_parts(&parts), definitions))
}

/// Port of `ichiran:romanize*` (`romanize.lisp:273-290`).
///
/// Splits `input` and, for each word segment, romanizes every
/// `dict-segment` alternative into `(romanized, word-info, prop)` triples
/// paired with the segment score; misc segments pass through as text.
#[derive(Debug)]
pub enum RomanizeStarSegment<P> {
    Misc(String),
    Word(Vec<(Vec<(String, WordInfo, P)>, i32)>),
}

pub async fn romanize_star_<P>(
    ctx: &KaniranContext,
    input: &str,
    method: KaniRomanizeMethod<'_>,
    limit: Option<usize>,
    wordprop_fn: impl Fn(&str, &WordInfo) -> P,
) -> Result<Vec<RomanizeStarSegment<P>>, sqlx::Error> {
    // (normalize input :context method) — characters.lisp:230 tests (eql context :kana)
    let context = match method {
        KaniRomanizeMethod::Kana => NormalizationContext::Kana,
        KaniRomanizeMethod::Method(_) => NormalizationContext::Default,
    };
    let input = normalize(input, context);
    let mut result: Vec<RomanizeStarSegment<P>> = Vec::new();
    for (split_type, split_text) in basic_split(&input) {
        if split_type == SegmentKind::Word {
            let mut alternatives = Vec::new();
            for (word_list, score) in dict_segment(ctx, &split_text, limit).await? {
                let mut word_props = Vec::new();
                for word in word_list {
                    let romanized = romanize_word_info(&word, method);
                    let prop = wordprop_fn(&romanized, &word);
                    word_props.push((romanized, word, prop));
                }
                alternatives.push((word_props, score));
            }
            result.push(RomanizeStarSegment::Word(alternatives));
        } else {
            result.push(RomanizeStarSegment::Misc(split_text));
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests;
