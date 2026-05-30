//! Port of `ichiran/dict-load.lisp` — the boot-time loaders for the
//! `*pos-index*` / `*pos-by-index*` / `*conj-description*` /
//! `*conj-rules*` hashes (the upstream `csv-hash` macroexpansions),
//! their accessors, the `conjugation-rule` defstruct, the conjugation
//! constructor [`construct_conjugation`], the `lex-compare` comparator
//! factory, and the seven flat parameter lists that drive conjugation
//! coverage (`*do-not-conjugate*`, `*do-not-conjugate-seq*`,
//! `*pos-with-conj-rules*`, `*secondary-conjugation-types*`,
//! `*secondary-conjugation-types-from*`).
//!
//! Each loader / accessor pair stays paired here: the upstream
//! `csv-hash` macro both declares the global and emits the loader /
//! accessor, so the section ordering follows the upstream macro
//! emission order rather than `dict-load.lisp`'s line-by-line order.
//! Conjugation-row layer follows (`conjugation_rule` struct →
//! `construct_conjugation` → `lex_compare` → the five flat lists).

use std::collections::HashMap;
use std::sync::OnceLock;

use super::errata::errata_conj_description_hook;
use crate::characters::char_classes::{test_word, CharClass};

// =========================================================================
// *pos-index* / load-pos-index / get-pos-index (dict-load.lisp:249)
// =========================================================================

const KWPOS_CSV: &str = include_str!("../../data/kwpos.csv");

/// Build the part-of-speech → (id, description) map from the embedded
/// kwpos.csv (tab-separated, header skipped — mirrors `cl-csv:read-csv
/// :separator #\Tab :skip-first-p t`). The trailing `ents` column is
/// dropped, as in the upstream `(pos-id pos description)` row binding.
///
/// Diverges from upstream `(merge-pathnames *jmdict-data* "kwpos.csv")`:
/// kwpos.csv is vendored into the crate and embedded with `include_str!`.
/// Returns the built map; the upstream `setf` of `*pos-index*` lives on
/// the `OnceLock` in [`pos_index`].
pub fn load_pos_index() -> HashMap<String, (i32, String)> {
    let mut hash = HashMap::new();
    for row in KWPOS_CSV.lines().skip(1) {
        let mut cols = row.split('\t');
        let pos_id = cols
            .next()
            .expect("kwpos.csv row missing id column")
            .parse::<i32>()
            .expect("kwpos.csv id column not an integer");
        let pos = cols.next().expect("kwpos.csv row missing kw column");
        let description = cols.next().expect("kwpos.csv row missing descr column");
        // dict-load.lisp:250 — value-form (cons (parse-integer pos-id) description)
        hash.insert(pos.to_string(), (pos_id, description.to_string()));
    }
    hash
}

/// part-of-speech tag → (numeric id, English description). Lazily built
/// once by [`load_pos_index`] (vendored kwpos.csv) and cached, mirroring
/// the upstream `defparameter *pos-index* nil` that `load-pos-index`
/// fills on first `get-pos-index`.
pub fn pos_index() -> &'static HashMap<String, (i32, String)> {
    static POS_INDEX: OnceLock<HashMap<String, (i32, String)>> = OnceLock::new();
    POS_INDEX.get_or_init(load_pos_index)
}

/// Look up the numeric part-of-speech id for a tag string in
/// `*pos-index*`, returning `nil`/`None` on a miss. The stored value
/// is `(cons id description)`; the accessor returns `(car val)` = the
/// id. The lazy `(unless *pos-index* (load-pos-index))` is the
/// `OnceLock` in [`pos_index`].
pub fn get_pos_index(key: &str) -> Option<i32> {
    // dict-load.lisp:251 — (val (car val)), val = (cons id description)
    pos_index().get(key).map(|val| val.0)
}

// =========================================================================
// *pos-by-index* / load-pos-by-index / get-pos (dict-load.lisp:253)
// =========================================================================

/// Build the numeric id → part-of-speech tag map from the embedded
/// kwpos.csv. The `description` / `ents` columns are unused by this
/// loader's value-form (`pos`).
///
/// Diverges from upstream `(merge-pathnames *jmdict-data* "kwpos.csv")`:
/// kwpos.csv is vendored into the crate and embedded with `include_str!`.
/// Returns the built map; the upstream `setf` of `*pos-by-index*` lives
/// on the `OnceLock` in [`pos_by_index`].
pub fn load_pos_by_index() -> HashMap<i32, String> {
    let mut hash = HashMap::new();
    for row in KWPOS_CSV.lines().skip(1) {
        let mut cols = row.split('\t');
        // dict-load.lisp:254 — row-key-form (parse-integer pos-id), value-form pos
        let pos_id = cols
            .next()
            .expect("kwpos.csv row missing id column")
            .parse::<i32>()
            .expect("kwpos.csv id column not an integer");
        let pos = cols.next().expect("kwpos.csv row missing kw column");
        hash.insert(pos_id, pos.to_string());
    }
    hash
}

/// numeric id → part-of-speech tag. Lazily built once by
/// [`load_pos_by_index`] (vendored kwpos.csv) and cached, mirroring the
/// upstream `defparameter *pos-by-index* nil` that `load-pos-by-index`
/// fills on first `get-pos`.
pub fn pos_by_index() -> &'static HashMap<i32, String> {
    static POS_BY_INDEX: OnceLock<HashMap<i32, String>> = OnceLock::new();
    POS_BY_INDEX.get_or_init(load_pos_by_index)
}

/// Look up the part-of-speech tag for a numeric id in `*pos-by-index*`,
/// returning `nil`/`None` on a miss. The upstream `(unless
/// *pos-by-index* (load-pos-by-index))` lazy-load is the `OnceLock` in
/// [`pos_by_index`].
pub fn get_pos(key: i32) -> Option<&'static str> {
    pos_by_index().get(&key).map(String::as_str)
}

// =========================================================================
// *conj-description* / load-conj-description / get-conj-description
// (dict-load.lisp:257)
// =========================================================================

const CONJ_CSV: &str = include_str!("../../data/conj.csv");

/// Build the conj-id → description map: parse the embedded conj.csv
/// (tab-separated, header skipped — mirrors `cl-csv:read-csv
/// :separator #\Tab :skip-first-p t`), then apply
/// [`errata_conj_description_hook`].
///
/// Diverges from upstream `(merge-pathnames *jmdict-data* "conj.csv")`:
/// conj.csv is an external jmdictdb file, vendored into the crate and
/// embedded with `include_str!`, so kaniran needs no jmdictdb checkout.
/// Returns the built map; the upstream `setf` of `*conj-description*`
/// lives on the `OnceLock` in [`conj_description`].
pub fn load_conj_description() -> HashMap<i32, String> {
    let mut hash = HashMap::new();
    for row in CONJ_CSV.lines().skip(1) {
        let mut cols = row.split('\t');
        let conj_id = cols
            .next()
            .expect("conj.csv row missing id column")
            .parse::<i32>()
            .expect("conj.csv id column not an integer");
        let description = cols.next().expect("conj.csv row missing name column");
        hash.insert(conj_id, description.to_string());
    }
    errata_conj_description_hook(&mut hash);
    hash
}

/// conj-id → English conjugation-type description. Lazily built once
/// by [`load_conj_description`] (vendored conj.csv + errata) and
/// cached, mirroring the upstream `defparameter *conj-description* nil`
/// that `load-conj-description` fills on first `get-conj-description`.
pub fn conj_description() -> &'static HashMap<i32, String> {
    static CONJ_DESCRIPTION: OnceLock<HashMap<i32, String>> = OnceLock::new();
    CONJ_DESCRIPTION.get_or_init(load_conj_description)
}

/// Looks up a conj-id's description in [`conj_description`]; `None`
/// when the id is absent (upstream `gethash` → `nil`).
pub fn get_conj_description(key: i32) -> Option<&'static str> {
    conj_description().get(&key).map(String::as_str)
}

// =========================================================================
// conjugation-rule struct (dict-load.lisp:262)
// =========================================================================

/// In-memory record carrying one row of `conjo.csv` — the conjugation
/// rules table that maps a (part-of-speech, conjugation-id, neg, fml,
/// onum) key to the stem index and three okurigana / euphonic
/// fragments (`okuri`, `euphr`, `euphk`) used to assemble the
/// conjugated form. Built once per CSV row inside the upstream
/// `csv-hash *conj-rules*` loader (`dict-load.lisp:266`) and pushed
/// onto the per-pos hash entry. The defstruct uses a positional
/// constructor — the Rust port preserves that shape.
#[derive(Debug, Clone)]
pub struct ConjugationRule {
    pub pos: i32,
    pub conj: i32,
    pub neg: bool,
    pub fml: bool,
    pub onum: i32,
    pub stem: i32,
    pub okuri: String,
    pub euphr: String,
    pub euphk: String,
}

// =========================================================================
// construct-conjugation (dict-load.lisp:284)
// =========================================================================

/// Assemble a conjugated reading from a dictionary `word` and a
/// `ConjugationRule`: drop `stem` trailing characters (one extra when
/// the applicable euphonic fragment is non-empty), then append the
/// euphonic fragment (`euphr` when the last two characters are kana,
/// `euphk` otherwise) and `okuri`.
///
/// Lisp `length` / `subseq` index by character; the port collects
/// `chars()` so offsets stay character-based for multi-byte readings.
pub fn construct_conjugation(word: &str, rule: &ConjugationRule) -> String {
    let chars: Vec<char> = word.chars().collect();
    let len = chars.len();
    // (subseq word (max 0 (- (length word) 2))) — last two characters
    let last_two: String = chars[len.saturating_sub(2)..].iter().collect();
    let iskana = test_word(&last_two, CharClass::Kana);
    let euphr = &rule.euphr;
    let euphk = &rule.euphk;
    let stem = rule.stem
        + if (iskana && !euphr.is_empty()) || (!iskana && !euphk.is_empty()) {
            1
        } else {
            0
        };
    let mut result: String = chars[..len - stem as usize].iter().collect();
    result.push_str(if iskana { euphr } else { euphk });
    result.push_str(&rule.okuri);
    result
}

// =========================================================================
// lex-compare (dict-load.lisp:367)
// =========================================================================

/// Returns a lexicographic comparator (a closure) parameterised on the
/// element-level `predicate`. Walks two equal-length sequences in
/// lockstep; the first pair where `predicate(e1, e2)` is true makes the
/// comparator return `true`, the first pair where `predicate(e2, e1)`
/// is true makes it return `false`. If neither holds for any pair
/// (sequences compare equal under `predicate`), the comparator returns
/// `false`. Per the upstream docstring, sequences must be of equal
/// length; mismatched lengths walk only the shared prefix and then
/// return `false`, matching Common Lisp's `(map nil …)` semantics.
///
/// ## Divergences from Lisp
///
/// - Returns `impl Fn(&[T], &[T]) -> bool` instead of a CL function
///   value. Rust closures are statically typed; the generic parameters
///   `T` (element type) and `P` (predicate type) are inferred from the
///   `predicate` argument and the eventual call site.
/// - The element predicate takes `&T, &T` rather than CL's by-value
///   convention, per CONVENTIONS §4.9 (prefer references over clones).
/// - Comparator inputs are slices (`&[T]`) rather than CL `sequence`
///   designators. Verified against each upstream caller's sort-key
///   shape:
///     * `insert-conjugation` (`dict-load.lisp:379`) — `:key #'cdddr`
///       on `readings`, where the cdddr tail is the propagated
///       conjugation key (a list of integers). Ports to `&[i32]`.
///     * `select-conjs-and-props` (`dict.lisp:1645`) — `:key 'third`
///       returns `(list (if (eql (seq-via conj) :null) 0 1) val)`, a
///       2-element list of integers. Ports to `&[i32]`.
///     * `pair-words-by-conj` (`dict-grammar.lisp:64`) — sort key is
///       a list of `(seq-from via)` 2-tuples, each integers. Ports to
///       `&[i32]` after flattening the per-conj-id pair into the
///       outer comparison.
///   All three are homogeneous integer sequences; the slice signature
///   is sufficient. If a future caller needs heterogeneous element
///   types (e.g. a `(i32, &str)` sort key), the slice signature must
///   be widened or a parallel implementation added — flag at that
///   port site.
pub fn lex_compare<T, P>(predicate: P) -> impl Fn(&[T], &[T]) -> bool
where
    P: Fn(&T, &T) -> bool,
{
    move |seq1, seq2| {
        // dict-load.lisp:371 (map nil (lambda (e1 e2) …) seq1 seq2)
        for (e1, e2) in seq1.iter().zip(seq2.iter()) {
            if predicate(e1, e2) {
                return true;
            }
            if predicate(e2, e1) {
                return false;
            }
        }
        // dict-load.lisp:370 (block nil …) — falls off `map nil` to nil.
        false
    }
}

// =========================================================================
// *do-not-conjugate* (dict-load.lisp:303)
// =========================================================================

pub static DO_NOT_CONJUGATE: &[&str] = &["n", "vs", "adj-na"];

// =========================================================================
// *do-not-conjugate-seq* (dict-load.lisp:305)
// =========================================================================

pub static DO_NOT_CONJUGATE_SEQ: &[i32] = &[2765070, 2835284];

// =========================================================================
// *pos-with-conj-rules* (dict-load.lisp:307)
// =========================================================================

pub static POS_WITH_CONJ_RULES: &[&str] = &[
    "adj-i", "adj-ix", "cop-da", "v1", "v1-s", "v5aru", "v5b", "v5g", "v5k", "v5k-s", "v5m", "v5n",
    "v5r", "v5r-i", "v5s", "v5t", "v5u", "v5u-s", "vk", "vs-s", "vs-i",
];

// =========================================================================
// *secondary-conjugation-types-from* (dict-load.lisp:312)
// =========================================================================

/// Upstream form is `` `(5 6 7 8 ,+conj-causative-su+) ``; the final
/// `53` is `+conj-causative-su+` (`dict-errata.lisp:1239`), inlined
/// following the [`super::errata::WEAK_CONJ_FORMS`] precedent (the
/// `+conj-*+` `defconstant` forms aren't picked up by the introspector).
pub static SECONDARY_CONJUGATION_TYPES_FROM: &[i32] = &[5, 6, 7, 8, 53];

// =========================================================================
// *secondary-conjugation-types* (dict-load.lisp:314)
// =========================================================================

pub static SECONDARY_CONJUGATION_TYPES: &[i32] = &[2, 3, 4, 9, 10, 11, 12, 13];

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// The `OnceLock` builder wiring runs `load_pos_index` (92 entries
    /// per the .103 REPL).
    #[test]
    fn pos_index_builds_once() {
        assert_eq!(pos_index().len(), 92);
    }

    /// The `OnceLock` builder wiring runs `load_pos_by_index` (92
    /// entries per the .103 REPL).
    #[test]
    fn pos_by_index_builds_once() {
        assert_eq!(pos_by_index().len(), 92);
    }

    /// The `OnceLock` builder wiring runs `load_conj_description`
    /// (18 entries per the .103 REPL).
    #[test]
    fn conj_description_builds_once() {
        assert_eq!(conj_description().len(), 18);
    }

    /// REPL (.103, after `(load-pos-index)`): `(hash-table-count
    /// *pos-index*)` = 92 (93 kwpos.csv lines − header). Spot-checks
    /// pin the tab-split, the `(id . description)` value, and the
    /// dropped header / `ents` column.
    #[test]
    fn load_pos_index_loads_kwpos_csv() {
        let map = load_pos_index();
        assert_eq!(map.len(), 92);

        let cases: &[(&str, (i32, &str))] = &[
            ("adj-i", (1, "adjective (keiyoushi)")),
            ("v5u", (41, "Godan verb with 'u' ending")),
            ("unc", (98, "unclassified")),
        ];
        for (pos, (pos_id, description)) in cases {
            let value = map.get(*pos).expect("pos present");
            assert_eq!(value.0, *pos_id, "pos={pos}");
            assert_eq!(value.1.as_str(), *description, "pos={pos}");
        }

        // header row not parsed as data
        assert_eq!(map.get("kw"), None);
    }

    /// REPL (.103, after `(load-pos-by-index)`): `(hash-table-count
    /// *pos-by-index*)` = 92 (93 kwpos.csv lines − header). Spot-checks
    /// pin the tab-split and the id→tag value.
    #[test]
    fn load_pos_by_index_loads_kwpos_csv() {
        let map = load_pos_by_index();
        assert_eq!(map.len(), 92);

        let cases: &[(i32, &str)] = &[(1, "adj-i"), (28, "v1"), (98, "unc")];
        for (pos_id, pos) in cases {
            assert_eq!(map.get(pos_id).map(String::as_str), Some(*pos), "id={pos_id}");
        }

        // header row not parsed as data
        assert_eq!(map.get(&0), None);
    }

    /// REPL (.103, after `(load-conj-description)`): 18 entries — 13
    /// conj.csv rows + 5 from the errata hook. Spot-checks pin the
    /// tab-split parse (a `(~tara)` value), the CSV bounds, and the
    /// errata boundary keys (50, 54).
    #[test]
    fn load_conj_description_loads_conj_csv_with_errata() {
        let map = load_conj_description();
        assert_eq!(map.len(), 18);

        let cases: &[(i32, &str)] = &[
            (1, "Non-past"),
            (11, "Conditional (~tara)"),
            (13, "Continuative (~i)"),
            (50, "Adverbial"),         // +conj-adverbial+ (errata)
            (54, "Old/literary form"), // +conj-adjective-literary+ (errata)
        ];
        for (conj_id, description) in cases {
            assert_eq!(
                map.get(conj_id).map(String::as_str),
                Some(*description),
                "conj_id={conj_id}"
            );
        }

        // header row not parsed; out-of-range / between-range keys absent
        assert_eq!(map.get(&0), None);
        assert_eq!(map.get(&14), None);
        assert_eq!(map.get(&999), None);
    }

    /// REPL (.103, `ichiran/dict::get-pos-index`), 2026-05-24. Spot-checks
    /// across the kwpos.csv tags plus two misses (`nil` on absent key /
    /// empty string).
    #[test]
    fn get_pos_index_lookups() {
        let cases: &[(&str, Option<i32>)] = &[
            ("adj-i", Some(1)),
            ("adj-ix", Some(7)),
            ("v5aru", Some(30)),
            ("v1", Some(28)),
            ("v1-s", Some(29)),
            ("v5u", Some(41)),
            ("vs-s", Some(47)),
            ("v5r", Some(37)),
            ("n", Some(17)),
            ("nonexistent-pos", None),
            ("", None),
        ];
        for (key, expected) in cases {
            assert_eq!(get_pos_index(key), *expected, "key={key:?}");
        }
    }

    /// REPL (.103, after `(load-pos-by-index)`): `(get-pos id)` for
    /// present ids and a miss (`(get-pos 99999)` → nil).
    #[test]
    fn get_pos_lookups() {
        let cases: &[(i32, Option<&str>)] = &[
            (1, Some("adj-i")),
            (28, Some("v1")),
            (98, Some("unc")),
            (99999, None),
        ];
        for (key, expected) in cases {
            assert_eq!(get_pos(*key), *expected, "key={key}");
        }
    }

    /// REPL fixtures (.103, ichiran/dict::get-conj-description), 2026-05-24.
    /// Present ids resolve to their description; an absent id returns
    /// `None` (upstream nil).
    #[test]
    fn get_conj_description_fixtures() {
        let cases: &[(i32, Option<&str>)] = &[
            (1, Some("Non-past")),
            (2, Some("Past (~ta)")),
            (11, Some("Conditional (~tara)")),
            (50, Some("Adverbial")),
            (999, None),
        ];
        for (key, expected) in cases {
            assert_eq!(get_conj_description(*key), *expected, "key={key}");
        }
    }

    mod test_construct_conjugation {
        use super::*;

        fn rule(stem: i32, okuri: &str, euphr: &str, euphk: &str) -> ConjugationRule {
            ConjugationRule {
                pos: 0,
                conj: 0,
                neg: false,
                fml: false,
                onum: 1,
                stem,
                okuri: okuri.to_string(),
                euphr: euphr.to_string(),
                euphk: euphk.to_string(),
            }
        }

        /// REPL (.103, `(construct-conjugation reading rule)` over real
        /// conjugation rows): exercises every branch — kana vs kanji ending
        /// (`iskana`), euphr vs euphk selection, the +1 stem bump when the
        /// applicable euphonic fragment is non-empty (and no bump when it is
        /// empty), the plain no-euphonic case, and the `(max 0 …)` guard on
        /// a one-character reading.
        #[test]
        fn construct_conjugation_paths() {
            // (reading, rule, expected)
            let cases: &[(&str, ConjugationRule, &str)] = &[
                // vs-i conj=5: euphr non-empty, kana ending -> euphr + bump
                ("する", rule(1, "る", "でき", "出来"), "できる"),
                // vs-i conj=5: euphk non-empty, kanji ending -> euphk + bump
                ("為る", rule(1, "る", "でき", "出来"), "出来る"),
                // adj-ix conj=2: euphr non-empty, kana ending -> euphr + bump
                ("いい", rule(1, "かった", "よ", ""), "よかった"),
                // adj-ix conj=2: euphk empty, kanji ending -> no bump, euphk ""
                ("良い", rule(1, "かった", "よ", ""), "良かった"),
                // vk conj=1: euphr non-empty, kana ending -> euphr + bump
                ("くる", rule(1, "ます", "き", ""), "きます"),
                // vk conj=1: euphk empty, kanji ending -> no bump, euphk ""
                ("来る", rule(1, "ます", "き", ""), "来ます"),
                // v5u conj=3: no euphonic fragments, kana ending -> no bump
                ("かう", rule(1, "って", "", ""), "かって"),
                // v5u conj=3: no euphonic fragments, kanji ending -> no bump
                ("買う", rule(1, "って", "", ""), "買って"),
                // one-character reading: (max 0 (- 1 2)) guard, stem=1 -> prefix ""
                ("る", rule(1, "わ", "", ""), "わ"),
            ];
            for (reading, conj_rule, expected) in cases {
                assert_eq!(
                    construct_conjugation(reading, conj_rule),
                    *expected,
                    "reading={reading}"
                );
            }
        }
    }

    mod test_lex_compare {
        use super::*;

        /// REPL: `(funcall (lex-compare #'<) #(1 2 3) #(1 2 4))` → `T`.
        #[test]
        fn smaller_at_last_position_is_less() {
            let cmp = lex_compare(|a: &i32, b: &i32| a < b);
            assert!(cmp(&[1, 2, 3], &[1, 2, 4]));
        }

        /// REPL: `(funcall (lex-compare #'<) #(1 2 4) #(1 2 3))` → `NIL`.
        #[test]
        fn larger_at_last_position_is_not_less() {
            let cmp = lex_compare(|a: &i32, b: &i32| a < b);
            assert!(!cmp(&[1, 2, 4], &[1, 2, 3]));
        }

        /// REPL: `(funcall (lex-compare #'<) #(1 2 3) #(1 2 3))` → `NIL`.
        /// Equal sequences fall off the end of `map nil` to `nil`.
        #[test]
        fn equal_sequences_return_false() {
            let cmp = lex_compare(|a: &i32, b: &i32| a < b);
            assert!(!cmp(&[1, 2, 3], &[1, 2, 3]));
        }

        /// REPL: `(funcall (lex-compare #'<) (list 1 2 3) (list 1 2 4))` → `T`.
        /// List input matches vector input — both go through `map nil`.
        #[test]
        fn list_inputs_behave_the_same() {
            let cmp = lex_compare(|a: &i32, b: &i32| a < b);
            let s1: Vec<i32> = vec![1, 2, 3];
            let s2: Vec<i32> = vec![1, 2, 4];
            assert!(cmp(&s1, &s2));
        }

        /// REPL: `(funcall (lex-compare #'<) #() #())` → `NIL`.
        #[test]
        fn empty_sequences_return_false() {
            let cmp = lex_compare(|a: &i32, b: &i32| a < b);
            let empty: [i32; 0] = [];
            assert!(!cmp(&empty, &empty));
        }

        /// REPL: `(funcall (lex-compare #'<) (list 1 2) (list 1 2 3))` → `NIL`.
        /// Unequal lengths walk the shared prefix only; falls through to
        /// `nil` when the shared prefix compares equal. (Upstream docstring
        /// notes "Only can sort sequences of equal length"; this pins the
        /// observable behavior at the boundary.)
        #[test]
        fn shorter_seq_with_equal_prefix_returns_false() {
            let cmp = lex_compare(|a: &i32, b: &i32| a < b);
            assert!(!cmp(&[1, 2], &[1, 2, 3]));
        }

        /// REPL: `(funcall (lex-compare #'<) (list 1 2 3) (list 1 2))` → `NIL`.
        /// Symmetric mismatched-length case from `lex-compare`'s perspective.
        #[test]
        fn longer_seq_with_equal_prefix_returns_false() {
            let cmp = lex_compare(|a: &i32, b: &i32| a < b);
            assert!(!cmp(&[1, 2, 3], &[1, 2]));
        }

        /// REPL: `(funcall (lex-compare #'<) (list 1) ())` → `NIL`.
        /// Empty-vs-non-empty: `map nil` exits before any pair is examined.
        #[test]
        fn empty_vs_non_empty_returns_false() {
            let cmp = lex_compare(|a: &i32, b: &i32| a < b);
            let empty: [i32; 0] = [];
            assert!(!cmp(&[1], &empty));
        }

        /// First-position dominates: `(2,9) < (3,1)` under `#'<` since the
        /// first pair already settles the order.
        /// REPL: `(funcall (lex-compare #'<) #(2 9) #(3 1))` → `T`.
        #[test]
        fn first_position_dominates_when_unequal() {
            let cmp = lex_compare(|a: &i32, b: &i32| a < b);
            let result = cmp(&[2, 9], &[3, 1]);
            assert!(result);
        }
    }
}
