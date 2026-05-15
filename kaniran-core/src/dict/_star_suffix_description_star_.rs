//! Port of `ichiran/dict:*suffix-description*` (`dict-grammar.lisp:108`).
//!
//! Hashtable mapping a suffix-class keyword (`:chau`, `:ha`, `:tai`,
//! …) **or** a JMdict seq integer (`2826528`, `2028980`, …) to a
//! human-readable description string. Upstream builds it once at
//! load time via `(hash-from-list *suffix-description* '(...))`
//! against the literal payload at `dict-grammar.lisp:110-158`. There
//! is no DB or other-global derivation — the value is a pure
//! literal — and no later form extends it.
//!
//! Read by [`get-suffix-description`](`dict-grammar.lisp:160`)
//! which dispatches on `(or (gethash seq *suffix-class*) seq)`: a
//! seq that is registered to a class gets looked up under the
//! class keyword, otherwise the seq integer itself is the key. The
//! mixed-type key surface is the load-bearing shape of the table.
//!
//! ## Rust shape
//!
//! Mirrors the upstream single-hashtable shape by carrying mixed
//! keys in an inline two-variant enum, per CONVENTIONS §4.3. Class
//! keywords are stored as `String` without the Lisp leading `:`,
//! matching how [`super::_star_suffix_class_star_::SuffixClass`]
//! and [`super::init_suffixes_thread`] carry class strings (e.g.
//! `:chau` → `"chau"`). Seq keys are plain `i32`.
//!
//! Built once under [`OnceLock`] from the literal payload. The
//! integer-seq half of the literal corresponds to the upstream
//! comment "these are used for splitsegs" (`dict-grammar.lisp:149`).

use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum SuffixDescKey {
    Class(String),
    Seq(i32),
}

static MAP: OnceLock<HashMap<SuffixDescKey, &'static str>> = OnceLock::new();

pub fn suffix_description() -> &'static HashMap<SuffixDescKey, &'static str> {
    MAP.get_or_init(|| {
        let mut map: HashMap<SuffixDescKey, &'static str> = HashMap::with_capacity(47);
        // dict-grammar.lisp:110-148 (hash-from-list payload — class keywords)
        map.insert(SuffixDescKey::Class("chau".to_string()), "indicates completion (to finish ...)");
        map.insert(SuffixDescKey::Class("ha".to_string()), "topic marker particle");
        map.insert(SuffixDescKey::Class("tai".to_string()), "want to... / would like to...");
        map.insert(SuffixDescKey::Class("iru".to_string()), "indicates continuing action (to be ...ing)");
        map.insert(SuffixDescKey::Class("oru".to_string()), "indicates continuing action (to be ...ing) (humble)");
        map.insert(SuffixDescKey::Class("aru".to_string()), "indicates completion / finished action");
        map.insert(SuffixDescKey::Class("kuru".to_string()), "indicates action that had been continuing up till now / came to be ");
        map.insert(SuffixDescKey::Class("oku".to_string()), "to do in advance / to leave in the current state expecting a later change");
        map.insert(SuffixDescKey::Class("kureru".to_string()), "(asking) to do something for one");
        map.insert(SuffixDescKey::Class("morau".to_string()), "(asking) to get somebody to do something");
        map.insert(SuffixDescKey::Class("itadaku".to_string()), "(asking) to get somebody to do something (polite)");
        map.insert(SuffixDescKey::Class("iku".to_string()), "is becoming / action starting now and continuing");
        map.insert(SuffixDescKey::Class("suru".to_string()), "makes a verb from a noun");
        map.insert(SuffixDescKey::Class("itasu".to_string()), "makes a verb from a noun (humble)");
        map.insert(SuffixDescKey::Class("sareru".to_string()), "makes a verb from a noun (honorific or passive)");
        map.insert(SuffixDescKey::Class("saseru".to_string()), "let/make someone/something do ...");
        map.insert(SuffixDescKey::Class("rou".to_string()), "probably / it seems that... / I guess ...");
        map.insert(SuffixDescKey::Class("ii".to_string()), "it's ok if ... / is it ok if ...?");
        map.insert(SuffixDescKey::Class("mo".to_string()), "even if ...");
        map.insert(SuffixDescKey::Class("sugiru".to_string()), "to be too (much) ...");
        map.insert(SuffixDescKey::Class("nikui".to_string()), "difficult to...");
        map.insert(SuffixDescKey::Class("gatai".to_string()), "difficult to...");
        map.insert(SuffixDescKey::Class("sa".to_string()), "-ness (degree or condition of adjective)");
        map.insert(SuffixDescKey::Class("tsutsu".to_string()), "while ... / in the process of ...");
        map.insert(SuffixDescKey::Class("tsutsuaru".to_string()), "to be doing ... / to be in the process of doing ...");
        map.insert(SuffixDescKey::Class("uru".to_string()), "can ... / to be able to ...");
        map.insert(SuffixDescKey::Class("sou".to_string()), "looking like ... / seeming ...");
        map.insert(SuffixDescKey::Class("nai".to_string()), "negative suffix");
        map.insert(SuffixDescKey::Class("ra".to_string()), "pluralizing suffix (not polite)");
        map.insert(SuffixDescKey::Class("kudasai".to_string()), "please do ...");
        map.insert(SuffixDescKey::Class("yagaru".to_string()), "indicates disdain or contempt");
        map.insert(SuffixDescKey::Class("naru".to_string()), "to become ...");
        map.insert(SuffixDescKey::Class("desu".to_string()), "formal copula");
        map.insert(SuffixDescKey::Class("desho".to_string()), "it seems/perhaps/don't you think?");
        map.insert(SuffixDescKey::Class("tosuru".to_string()), "to try to .../to be about to...");
        map.insert(SuffixDescKey::Class("garu".to_string()), "to feel .../have a ... impression of someone");
        map.insert(SuffixDescKey::Class("me".to_string()), "somewhat/-ish");
        map.insert(SuffixDescKey::Class("gai".to_string()), "worth it to ...");
        map.insert(SuffixDescKey::Class("tasou".to_string()), "seem to want to... (tai+sou)");
        // dict-grammar.lisp:149-157 (hash-from-list payload — seq keys for splitsegs)
        map.insert(SuffixDescKey::Seq(2826528), "polite prefix");
        map.insert(SuffixDescKey::Seq(2028980), "at / in / by");
        map.insert(SuffixDescKey::Seq(2028970), "or / questioning particle");
        map.insert(SuffixDescKey::Seq(2028990), "to / at / in");
        map.insert(SuffixDescKey::Seq(2029010), "indicates direct object of action");
        map.insert(SuffixDescKey::Seq(1469800), "indicates possessive (...'s)");
        map.insert(SuffixDescKey::Seq(2086960), "quoting particle");
        map.insert(SuffixDescKey::Seq(1002980), "from / because");
        map
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cardinality and spot-check against the live image. Probed
    /// on .103 (`(hash-table-count *suffix-description*) => 47`,
    /// per-key values dumped via `maphash`).
    #[test]
    fn matches_introspected_value() {
        let map = suffix_description();
        assert_eq!(map.len(), 47);

        // class keywords
        assert_eq!(
            map.get(&SuffixDescKey::Class("chau".to_string())).copied(),
            Some("indicates completion (to finish ...)"),
        );
        assert_eq!(
            map.get(&SuffixDescKey::Class("ha".to_string())).copied(),
            Some("topic marker particle"),
        );
        // trailing space is load-bearing — preserved from upstream literal
        assert_eq!(
            map.get(&SuffixDescKey::Class("kuru".to_string())).copied(),
            Some("indicates action that had been continuing up till now / came to be "),
        );
        assert_eq!(
            map.get(&SuffixDescKey::Class("tasou".to_string())).copied(),
            Some("seem to want to... (tai+sou)"),
        );

        // seq keys
        assert_eq!(map.get(&SuffixDescKey::Seq(2826528)).copied(), Some("polite prefix"));
        assert_eq!(map.get(&SuffixDescKey::Seq(2028980)).copied(), Some("at / in / by"));
        assert_eq!(map.get(&SuffixDescKey::Seq(1002980)).copied(), Some("from / because"));

        // miss
        assert_eq!(map.get(&SuffixDescKey::Class("nonexistent".to_string())).copied(), None);
        assert_eq!(map.get(&SuffixDescKey::Seq(0)).copied(), None);
    }

    /// Pin the class/seq partition counts so adding/removing
    /// entries on one side trips the test.
    #[test]
    fn class_seq_partition() {
        let map = suffix_description();
        let class_count = map.keys().filter(|k| matches!(k, SuffixDescKey::Class(_))).count();
        let seq_count = map.keys().filter(|k| matches!(k, SuffixDescKey::Seq(_))).count();
        assert_eq!(class_count, 39);
        assert_eq!(seq_count, 8);
    }
}
