//! Translation between Common Lisp symbol names (FQNs) and the Rust
//! source paths used in `kaniran-core`.
//!
//! The mapping is mechanical so that any FQN drawn from
//! `reverse/scripts/symbols.csv` can be turned into a unique location
//! (and back) without ambiguity. This is the single source of truth
//! for how Lisp names land in the Rust tree.
//!
//! ## Rules
//!
//! - The package qualifier becomes a subdirectory:
//!   `ichiran/characters` → `characters/`,
//!   `ichiran/dict`       → `dict/`,
//!   the bare `ichiran`   → `core/`  (the root package is renamed so it
//!                                     does not shadow the crate root).
//! - The bare symbol name becomes the file stem with these character
//!   substitutions: lowercase, `*` → `_star_`, `+` → `_plus_`,
//!   `-` → `_`. Runs of `_` are collapsed but leading/trailing `_` is
//!   preserved — the codebase contains pairs like `suffix-ren` /
//!   `suffix-ren-` that would otherwise collide on the same filename.
//! - Macros get a `_macro` stem suffix so they cannot be mistaken for
//!   ported functions (per the port plan, macros do not become Rust
//!   functions; they collapse into data tables).
//! - Generic functions and ordinary functions share the same path —
//!   the generic itself goes in `<name>.rs`; method implementations
//!   are a separate decision handled in their own files.
//! - The introspector annotates generic-function FQNs with a literal
//!   trailing ` (generic function)`; that annotation is stripped here.
//!
//! ## Coverage
//!
//! Verified against all 944 symbols in `reverse/scripts/symbols.csv`
//! (689 fn, 126 global, 38 gf, 36 macro, 28 class, 14 dao, 11 struct,
//! 1 type, 1 condition): every one produces a path matching
//! `[a-z0-9_]+(/[a-z0-9_]+)*\.rs`, and no two symbols collide on the
//! same case-insensitive path.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Fn,
    Gf,
    Macro,
    Global,
    Struct,
    Class,
    Dao,
    Type,
    Condition,
}

impl SymbolKind {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "fn" => Ok(Self::Fn),
            "gf" => Ok(Self::Gf),
            "macro" => Ok(Self::Macro),
            "global" => Ok(Self::Global),
            "struct" => Ok(Self::Struct),
            "class" => Ok(Self::Class),
            "dao" => Ok(Self::Dao),
            "type" => Ok(Self::Type),
            "condition" => Ok(Self::Condition),
            other => Err(format!("unknown symbol kind: {other:?}")),
        }
    }

    /// Stem suffix that distinguishes this kind from a same-named
    /// function in the same package. Mirrors the suffix scheme used by
    /// the introspector's md-file output.
    fn stem_suffix(self) -> &'static str {
        match self {
            Self::Fn | Self::Gf | Self::Global => "",
            Self::Macro => "_macro",
            Self::Struct => "_struct",
            Self::Class => "_class",
            Self::Dao => "_dao",
            Self::Type => "_type",
            Self::Condition => "_condition",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RustPath {
    /// Subdirectory under `src/`, e.g. `"characters"`.
    pub module_dir: String,
    /// File stem, e.g. `"as_hiragana"` or `"some_macro_macro"`.
    pub file_stem: String,
}

impl RustPath {
    /// File path under `src/`, e.g. `"characters/as_hiragana.rs"`.
    pub fn file_path(&self) -> String {
        format!("{}/{}.rs", self.module_dir, self.file_stem)
    }

    /// Rust module path, e.g. `"characters::as_hiragana"`.
    pub fn module_path(&self) -> String {
        format!("{}::{}", self.module_dir, self.file_stem)
    }
}

/// Translate a Lisp FQN such as `"ICHIRAN/CHARACTERS:AS-HIRAGANA"`
/// into its Rust source location, given the symbol kind.
pub fn fqn_to_path(fqn: &str, kind: SymbolKind) -> Result<RustPath, String> {
    let cleaned = fqn.trim_end_matches(" (generic function)");
    let (package, name) = cleaned
        .split_once(':')
        .ok_or_else(|| format!("missing ':' in FQN: {fqn}"))?;
    if name.is_empty() {
        return Err(format!("empty symbol name in FQN: {fqn}"));
    }
    let mut file_stem = name_to_stem(name);
    let suffix = kind.stem_suffix();
    if !suffix.is_empty() {
        file_stem.push_str(suffix);
        file_stem = collapse_underscores(&file_stem);
    }
    Ok(RustPath {
        module_dir: package_to_module_dir(package),
        file_stem,
    })
}

fn package_to_module_dir(pkg: &str) -> String {
    let lower = pkg.to_ascii_lowercase();
    let bare = if let Some(rest) = lower.strip_prefix("ichiran/") {
        rest.to_string()
    } else if lower == "ichiran" {
        "core".to_string()
    } else {
        lower
    };
    translate_chars(&bare)
}

fn name_to_stem(name: &str) -> String {
    translate_chars(&name.to_ascii_lowercase())
}

/// Apply the `* → _star_`, `+ → _plus_`, `- → _` substitutions, then
/// collapse runs of `_`. Leading/trailing `_` is preserved so that
/// names that differ only in trailing punctuation (e.g. `suffix-ren`
/// vs. `suffix-ren-`) land in distinct files.
fn translate_chars(s: &str) -> String {
    let expanded: String = s
        .chars()
        .map(|c| match c {
            '*' => "_star_".to_string(),
            '+' => "_plus_".to_string(),
            '-' => "_".to_string(),
            other => other.to_string(),
        })
        .collect();
    collapse_underscores(&expanded)
}

fn collapse_underscores(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_underscore = false;
    for c in s.chars() {
        if c == '_' {
            if !prev_underscore {
                out.push(c);
            }
            prev_underscore = true;
        } else {
            out.push(c);
            prev_underscore = false;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_simple_function() {
        let p = fqn_to_path("ICHIRAN/CHARACTERS:AS-HIRAGANA", SymbolKind::Fn).unwrap();
        assert_eq!(p.module_dir, "characters");
        assert_eq!(p.file_stem, "as_hiragana");
        assert_eq!(p.file_path(), "characters/as_hiragana.rs");
        assert_eq!(p.module_path(), "characters::as_hiragana");
    }

    #[test]
    fn maps_star_suffix() {
        let p = fqn_to_path("ICHIRAN/DICT:GET-SPLIT*", SymbolKind::Fn).unwrap();
        assert_eq!(p.file_stem, "get_split_star_");
    }

    #[test]
    fn maps_plus_suffix() {
        let p = fqn_to_path("ICHIRAN/DICT:SUFFIX-SOU+", SymbolKind::Fn).unwrap();
        assert_eq!(p.file_stem, "suffix_sou_plus_");
    }

    #[test]
    fn distinguishes_trailing_dash_from_no_dash() {
        // Real collision in the codebase: both symbols exist in ichiran/dict.
        let a = fqn_to_path("ICHIRAN/DICT:SUFFIX-REN", SymbolKind::Fn).unwrap();
        let b = fqn_to_path("ICHIRAN/DICT:SUFFIX-REN-", SymbolKind::Fn).unwrap();
        assert_eq!(a.file_stem, "suffix_ren");
        assert_eq!(b.file_stem, "suffix_ren_");
        assert_ne!(a.file_path(), b.file_path());
    }

    #[test]
    fn maps_internal_plus() {
        let p = fqn_to_path("ICHIRAN/DICT:SUFFIX-TE+SPACE", SymbolKind::Fn).unwrap();
        assert_eq!(p.file_stem, "suffix_te_plus_space");
    }

    #[test]
    fn maps_root_package_to_core() {
        let p = fqn_to_path("ICHIRAN:JOIN-PARTS", SymbolKind::Fn).unwrap();
        assert_eq!(p.module_dir, "core");
        assert_eq!(p.file_stem, "join_parts");
    }

    #[test]
    fn strips_generic_function_annotation() {
        let p = fqn_to_path("ICHIRAN/CONN:ENSURE (generic function)", SymbolKind::Gf).unwrap();
        assert_eq!(p.module_dir, "conn");
        assert_eq!(p.file_stem, "ensure");
    }

    #[test]
    fn global_keeps_earmuffs_no_suffix() {
        let p = fqn_to_path("ICHIRAN/CHARACTERS:*ABNORMAL-CHARS*", SymbolKind::Global).unwrap();
        assert_eq!(p.module_dir, "characters");
        assert_eq!(p.file_stem, "_star_abnormal_chars_star_");
        assert_eq!(p.file_path(), "characters/_star_abnormal_chars_star_.rs");
    }

    #[test]
    fn struct_class_dao_type_condition_get_suffix() {
        let cases = [
            (SymbolKind::Struct, "kana-representation", "kana_representation_struct"),
            (SymbolKind::Class, "simple-text", "simple_text_class"),
            (SymbolKind::Dao, "kanji", "kanji_dao"),
            (SymbolKind::Type, "char-class", "char_class_type"),
            (SymbolKind::Condition, "not-a-number", "not_a_number_condition"),
        ];
        for (kind, name, expected_stem) in cases {
            let fqn = format!("ICHIRAN/CHARACTERS:{name}");
            let p = fqn_to_path(&fqn, kind).unwrap();
            assert_eq!(p.file_stem, expected_stem, "kind={kind:?} name={name}");
        }
    }

    #[test]
    fn macro_gets_macro_suffix() {
        let p = fqn_to_path("ICHIRAN/DICT:DEF-COUNTER", SymbolKind::Macro).unwrap();
        assert_eq!(p.file_stem, "def_counter_macro");
        assert_eq!(p.file_path(), "dict/def_counter_macro.rs");
    }

    #[test]
    fn rejects_missing_colon() {
        assert!(fqn_to_path("ICHIRAN-NO-COLON-HERE", SymbolKind::Fn).is_err());
    }

    #[test]
    fn rejects_empty_name() {
        assert!(fqn_to_path("ICHIRAN/DICT:", SymbolKind::Fn).is_err());
    }

    fn read_symbols_csv() -> Vec<(String, SymbolKind)> {
        let csv_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../reverse/scripts/symbols.csv");
        let body = std::fs::read_to_string(&csv_path)
            .unwrap_or_else(|e| panic!("can't read {}: {e}", csv_path.display()));
        let mut lines = body.lines();
        let header = lines.next().expect("symbols.csv has no header");
        let cols: Vec<_> = header.split(',').collect();
        let fqn_idx = cols.iter().position(|c| *c == "fqn").expect("no fqn col");
        let kind_idx = cols.iter().position(|c| *c == "kind").expect("no kind col");
        let mut out = Vec::new();
        for line in lines {
            let cells: Vec<_> = line.split(',').collect();
            if cells.len() <= kind_idx.max(fqn_idx) {
                continue;
            }
            let kind = SymbolKind::parse(cells[kind_idx])
                .unwrap_or_else(|e| panic!("{e} for line: {line}"));
            out.push((cells[fqn_idx].to_string(), kind));
        }
        out
    }

    #[test]
    fn covers_every_symbol_in_the_codebase() {
        let valid_seg = |seg: &str| {
            !seg.is_empty()
                && seg
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        };
        let symbols = read_symbols_csv();
        assert!(
            symbols.len() > 700,
            "expected to scan >700 symbols, got {}",
            symbols.len()
        );

        let mut bad: Vec<(String, String)> = Vec::new();
        for (fqn, kind) in &symbols {
            match fqn_to_path(fqn, *kind) {
                Err(e) => bad.push((fqn.clone(), e)),
                Ok(p) => {
                    for seg in p.module_dir.split('/') {
                        if !valid_seg(seg) {
                            bad.push((fqn.clone(), format!("bad module_dir segment: {seg:?}")));
                        }
                    }
                    if !valid_seg(&p.file_stem) {
                        bad.push((fqn.clone(), format!("bad file_stem: {:?}", p.file_stem)));
                    }
                }
            }
        }
        assert!(
            bad.is_empty(),
            "{} symbols failed translation:\n{}",
            bad.len(),
            bad.iter()
                .take(20)
                .map(|(f, e)| format!("  {f}: {e}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    #[test]
    fn no_two_symbols_collide_on_the_same_path() {
        use std::collections::HashMap;
        let symbols = read_symbols_csv();
        let mut seen: HashMap<String, String> = HashMap::new();
        let mut collisions: Vec<(String, String, String)> = Vec::new();
        for (fqn, kind) in &symbols {
            let p = fqn_to_path(fqn, *kind).unwrap();
            let key = p.file_path().to_ascii_lowercase();
            if let Some(prev) = seen.insert(key.clone(), fqn.clone()) {
                collisions.push((key, prev, fqn.clone()));
            }
        }
        assert!(
            collisions.is_empty(),
            "{} path collisions:\n{}",
            collisions.len(),
            collisions
                .iter()
                .take(20)
                .map(|(p, a, b)| format!("  {p}: {a}  vs  {b}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}
