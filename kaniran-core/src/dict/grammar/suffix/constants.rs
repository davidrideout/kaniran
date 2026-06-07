use crate::conn::kani_context::KaniranContext;
use crate::dict::grammar::suffix::abbr::{
    abbr_beba, abbr_dewanai, abbr_geba, abbr_ii, abbr_keba, abbr_meba, abbr_n, abbr_nakereba,
    abbr_neba, abbr_nee, abbr_nx, abbr_reba, abbr_seba, abbr_shimasho, abbr_teba,
};
use crate::dict::grammar::suffix::rules::{
    suffix_adv, suffix_chau, suffix_desho, suffix_desu, suffix_garu, suffix_iadj, suffix_kudasai,
    suffix_kurai, suffix_neg, suffix_ra, suffix_rashii, suffix_ren, suffix_ren_, suffix_rou,
    suffix_sa, suffix_sou, suffix_sou_plus_, suffix_sugiru, suffix_suru, suffix_tai, suffix_te,
    suffix_te_plus_space, suffix_te_ren, suffix_teii, suffix_teiru, suffix_teiru_plus_, suffix_to,
    suffix_tosuru,
};
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kani_word::KaniWordDispatchEnum;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::OnceLock;

/// Port of `ichiran/dict:*suffix-cache*` (`dict-grammar.lisp:5`).
///
/// Suffix surface text → list of `(class, optional kana-form row)`
/// grammatical-suffix matches loaded for that text.
pub type SuffixCache = HashMap<String, Vec<(String, Option<KanaText>)>>;

pub fn suffix_cache(ctx: &KaniranContext) -> &SuffixCache {
    &ctx.suffix_cache
}

/// Port of `ichiran/dict:*suffix-class*` (`dict-grammar.lisp:6`).
///
/// JMdict seq → suffix class (`:teiru`, `:te`, `:iru`, `:ha`, …) the
/// entry belongs to.
pub type SuffixClass = HashMap<i32, String>;

pub fn suffix_class(ctx: &KaniranContext) -> &SuffixClass {
    &ctx.suffix_class
}

/// Port of `ichiran/dict:*suffix-description*` (`dict-grammar.lisp:108`).
///
/// Suffix-class keyword (`:chau`, `:ha`, …) **or** JMdict seq integer
/// → human-readable description string.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum SuffixDescKey {
    Class(String),
    Seq(i32),
}

pub fn suffix_description() -> &'static HashMap<SuffixDescKey, &'static str> {
    static MAP: OnceLock<HashMap<SuffixDescKey, &'static str>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut map: HashMap<SuffixDescKey, &'static str> = HashMap::with_capacity(47);
        // dict-grammar.lisp:110-148 (hash-from-list payload — class keywords)
        map.insert(
            SuffixDescKey::Class("chau".to_string()),
            "indicates completion (to finish ...)",
        );
        map.insert(
            SuffixDescKey::Class("ha".to_string()),
            "topic marker particle",
        );
        map.insert(
            SuffixDescKey::Class("tai".to_string()),
            "want to... / would like to...",
        );
        map.insert(
            SuffixDescKey::Class("iru".to_string()),
            "indicates continuing action (to be ...ing)",
        );
        map.insert(
            SuffixDescKey::Class("oru".to_string()),
            "indicates continuing action (to be ...ing) (humble)",
        );
        map.insert(
            SuffixDescKey::Class("aru".to_string()),
            "indicates completion / finished action",
        );
        map.insert(
            SuffixDescKey::Class("kuru".to_string()),
            "indicates action that had been continuing up till now / came to be ",
        );
        map.insert(
            SuffixDescKey::Class("oku".to_string()),
            "to do in advance / to leave in the current state expecting a later change",
        );
        map.insert(
            SuffixDescKey::Class("kureru".to_string()),
            "(asking) to do something for one",
        );
        map.insert(
            SuffixDescKey::Class("morau".to_string()),
            "(asking) to get somebody to do something",
        );
        map.insert(
            SuffixDescKey::Class("itadaku".to_string()),
            "(asking) to get somebody to do something (polite)",
        );
        map.insert(
            SuffixDescKey::Class("iku".to_string()),
            "is becoming / action starting now and continuing",
        );
        map.insert(
            SuffixDescKey::Class("suru".to_string()),
            "makes a verb from a noun",
        );
        map.insert(
            SuffixDescKey::Class("itasu".to_string()),
            "makes a verb from a noun (humble)",
        );
        map.insert(
            SuffixDescKey::Class("sareru".to_string()),
            "makes a verb from a noun (honorific or passive)",
        );
        map.insert(
            SuffixDescKey::Class("saseru".to_string()),
            "let/make someone/something do ...",
        );
        map.insert(
            SuffixDescKey::Class("rou".to_string()),
            "probably / it seems that... / I guess ...",
        );
        map.insert(
            SuffixDescKey::Class("ii".to_string()),
            "it's ok if ... / is it ok if ...?",
        );
        map.insert(SuffixDescKey::Class("mo".to_string()), "even if ...");
        map.insert(
            SuffixDescKey::Class("sugiru".to_string()),
            "to be too (much) ...",
        );
        map.insert(SuffixDescKey::Class("nikui".to_string()), "difficult to...");
        map.insert(SuffixDescKey::Class("gatai".to_string()), "difficult to...");
        map.insert(
            SuffixDescKey::Class("sa".to_string()),
            "-ness (degree or condition of adjective)",
        );
        map.insert(
            SuffixDescKey::Class("tsutsu".to_string()),
            "while ... / in the process of ...",
        );
        map.insert(
            SuffixDescKey::Class("tsutsuaru".to_string()),
            "to be doing ... / to be in the process of doing ...",
        );
        map.insert(
            SuffixDescKey::Class("uru".to_string()),
            "can ... / to be able to ...",
        );
        map.insert(
            SuffixDescKey::Class("sou".to_string()),
            "looking like ... / seeming ...",
        );
        map.insert(SuffixDescKey::Class("nai".to_string()), "negative suffix");
        map.insert(
            SuffixDescKey::Class("ra".to_string()),
            "pluralizing suffix (not polite)",
        );
        map.insert(SuffixDescKey::Class("kudasai".to_string()), "please do ...");
        map.insert(
            SuffixDescKey::Class("yagaru".to_string()),
            "indicates disdain or contempt",
        );
        map.insert(SuffixDescKey::Class("naru".to_string()), "to become ...");
        map.insert(SuffixDescKey::Class("desu".to_string()), "formal copula");
        map.insert(
            SuffixDescKey::Class("desho".to_string()),
            "it seems/perhaps/don't you think?",
        );
        map.insert(
            SuffixDescKey::Class("tosuru".to_string()),
            "to try to .../to be about to...",
        );
        map.insert(
            SuffixDescKey::Class("garu".to_string()),
            "to feel .../have a ... impression of someone",
        );
        map.insert(SuffixDescKey::Class("me".to_string()), "somewhat/-ish");
        map.insert(SuffixDescKey::Class("gai".to_string()), "worth it to ...");
        map.insert(
            SuffixDescKey::Class("tasou".to_string()),
            "seem to want to... (tai+sou)",
        );
        // dict-grammar.lisp:149-157 (hash-from-list payload — seq keys for splitsegs)
        map.insert(SuffixDescKey::Seq(2826528), "polite prefix");
        map.insert(SuffixDescKey::Seq(2028980), "at / in / by");
        map.insert(SuffixDescKey::Seq(2028970), "or / questioning particle");
        map.insert(SuffixDescKey::Seq(2028990), "to / at / in");
        map.insert(
            SuffixDescKey::Seq(2029010),
            "indicates direct object of action",
        );
        map.insert(SuffixDescKey::Seq(1469800), "indicates possessive (...'s)");
        map.insert(SuffixDescKey::Seq(2086960), "quoting particle");
        map.insert(SuffixDescKey::Seq(1002980), "from / because");
        map
    })
}

/// Port of `ichiran/dict:*suffix-list*` (`dict-grammar.lisp:329`).
///
/// Suffix keyword → function that materializes compound-text /
/// proxy-text candidates for that suffix.
/// Dispatch signature for one entry in [`SUFFIX_LIST`]. Mirrors the
/// `(funcall suffix-fn root suf kf)` shape at
/// `dict-grammar.lisp:707`: `root` is the prefix substring being
/// treated as a verb / noun stem, `suf` is the suffix surface text
/// from the cache, and `kf` is the optional kana-text row carrying
/// that suffix (`nil` upstream for abbreviated suffixes loaded with
/// `(load-abbr …)`).
pub type SuffixFn = for<'a> fn(
    &'a KaniranContext,
    &'a str,
    &'a str,
    Option<&'a KanaText>,
) -> Pin<
    Box<dyn Future<Output = Result<Vec<KaniWordDispatchEnum>, sqlx::Error>> + Send + 'a>,
>;

// Macro: wrap a `def-simple-suffix` body (returning `Vec<CompoundText>`
// with non-Option kf) into the unified SuffixFn shape. `.expect` is
// load-bearing — see the module doc's "Adapter `kf` unwrap policy".
macro_rules! simple_suffix_dispatch {
    ($name:ident, $fn:ident, $key:literal, $cache_loader:literal) => {
        fn $name<'a>(
            ctx: &'a KaniranContext,
            root: &'a str,
            suf: &'a str,
            kf: Option<&'a KanaText>,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<KaniWordDispatchEnum>, sqlx::Error>> + Send + 'a>>
        {
            Box::pin(async move {
                let kf = kf.expect(concat!(
                    "suffix-list :",
                    $key,
                    " dispatch: kf is nil; cache invariant (",
                    $cache_loader,
                    ") broken",
                ));
                let compounds = $fn(ctx, root, suf, kf).await?;
                Ok(compounds.into_iter().map(KaniWordDispatchEnum::Compound).collect())
            })
        }
    };
}

// Macro: wrap a `def-abbr-suffix` body (already returning
// `Vec<KaniWordDispatchEnum>` with Option kf — proxy-text + compound-
// text mixed per the etypecase arms at `dict-grammar.lisp:565-577`)
// into the unified SuffixFn shape. No `.expect` because the
// `def-abbr-suffix` body ignores `kf` (`(declare (ignore ,suf))`).
macro_rules! abbr_suffix_dispatch {
    ($name:ident, $fn:ident) => {
        fn $name<'a>(
            ctx: &'a KaniranContext,
            root: &'a str,
            suf: &'a str,
            kf: Option<&'a KanaText>,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<KaniWordDispatchEnum>, sqlx::Error>> + Send + 'a>>
        {
            Box::pin(async move { $fn(ctx, root, suf, kf).await })
        }
    };
}

// --- def-simple-suffix adapters --------------------------------------------
//
// One per upstream `(def-simple-suffix …)` form. Line refs are to the
// upstream `dict-grammar.lisp` defsuffix callsite; the cache-loader
// hint identifies which `(load-conjs …)` / `(load-kf …)` populator
// puts the keyword's row into `*suffix-cache*`.

simple_suffix_dispatch!(
    suffix_suru_dispatch,
    suffix_suru,
    "suru",
    "load-conjs :suru"
);
simple_suffix_dispatch!(suffix_ra_dispatch, suffix_ra, "ra", "load-kf :ra");
simple_suffix_dispatch!(suffix_tai_dispatch, suffix_tai, "tai", "load-conjs :tai");
simple_suffix_dispatch!(suffix_ren_dispatch, suffix_ren, "ren", "load-kf :ren");
simple_suffix_dispatch!(
    suffix_ren_minus_dispatch,
    suffix_ren_,
    "ren-",
    "load-conjs :ren-"
);
simple_suffix_dispatch!(suffix_neg_dispatch, suffix_neg, "neg", "load-kf :neg");
simple_suffix_dispatch!(
    suffix_te_dispatch,
    suffix_te,
    "te",
    "load-conjs :te / load-kf :te"
);
simple_suffix_dispatch!(
    suffix_teiru_dispatch,
    suffix_teiru,
    "teiru",
    "いる(る) loop"
);
simple_suffix_dispatch!(
    suffix_teiru_plus_dispatch,
    suffix_teiru_plus_,
    "teiru+",
    "いる(る) loop"
);
simple_suffix_dispatch!(
    suffix_te_plus_space_dispatch,
    suffix_te_plus_space,
    "te+space",
    "load-conjs :te+space"
);
simple_suffix_dispatch!(
    suffix_kudasai_dispatch,
    suffix_kudasai,
    "kudasai",
    "load-kf :kudasai"
);
simple_suffix_dispatch!(
    suffix_te_ren_dispatch,
    suffix_te_ren,
    "teren",
    "load-conjs :teren"
);
simple_suffix_dispatch!(suffix_teii_dispatch, suffix_teii, "teii", "load-kf :teii");
simple_suffix_dispatch!(suffix_rou_dispatch, suffix_rou, "rou", "load-kf :rou");
simple_suffix_dispatch!(suffix_adv_dispatch, suffix_adv, "adv", "load-conjs :adv");
simple_suffix_dispatch!(suffix_iadj_dispatch, suffix_iadj, "iadj", "load-kf :iadj");
simple_suffix_dispatch!(
    suffix_tosuru_dispatch,
    suffix_tosuru,
    "tosuru",
    "load-conjs :tosuru"
);
simple_suffix_dispatch!(
    suffix_kurai_dispatch,
    suffix_kurai,
    "kurai",
    "load-kf :kurai"
);
simple_suffix_dispatch!(
    suffix_chau_dispatch,
    suffix_chau,
    "chau",
    "load-conjs :chau"
);
simple_suffix_dispatch!(suffix_to_dispatch, suffix_to, "to", "load-conjs :to");
simple_suffix_dispatch!(suffix_sa_dispatch, suffix_sa, "sa", "load-kf :sa");
simple_suffix_dispatch!(suffix_sou_dispatch, suffix_sou, "sou", "load-kf :sou");
simple_suffix_dispatch!(
    suffix_sou_plus_dispatch,
    suffix_sou_plus_,
    "sou+",
    "load-kf :sou+"
);
simple_suffix_dispatch!(
    suffix_sugiru_dispatch,
    suffix_sugiru,
    "sugiru",
    "load-conjs :sugiru"
);
simple_suffix_dispatch!(
    suffix_garu_dispatch,
    suffix_garu,
    "garu",
    "load-conjs :garu"
);
simple_suffix_dispatch!(suffix_desu_dispatch, suffix_desu, "desu", "load-kf :desu");
simple_suffix_dispatch!(
    suffix_desho_dispatch,
    suffix_desho,
    "desho",
    "load-kf :desho"
);
simple_suffix_dispatch!(
    suffix_rashii_dispatch,
    suffix_rashii,
    "rashii",
    "load-kf :rashii"
);

// --- def-abbr-suffix adapters ---------------------------------------------
//
// One per upstream `(def-abbr-suffix …)` form. The keyword the
// upstream form publishes into `*suffix-list*` is the `keyword` arg of
// the macro, NOT the rust-side fn name. Mapping:
//   abbr_nee       → :nai       (dict-grammar.lisp:566)
//   abbr_nx        → :nai-x     (dict-grammar.lisp:572)
//   abbr_n         → :nai-n     (dict-grammar.lisp:594)
//   abbr_nakereba  → :nakereba  (dict-grammar.lisp:612)
//   abbr_shimasho  → :shimashou (dict-grammar.lisp:615)
//   abbr_dewanai   → :dewanai   (dict-grammar.lisp:618)
//   abbr_teba      → :teba      (dict-grammar.lisp:626)
//   abbr_reba      → :reba      (dict-grammar.lisp:629)
//   abbr_keba      → :keba      (dict-grammar.lisp:632)
//   abbr_geba      → :geba      (dict-grammar.lisp:635)
//   abbr_neba      → :neba      (dict-grammar.lisp:638)
//   abbr_beba      → :beba      (dict-grammar.lisp:641)
//   abbr_meba      → :meba      (dict-grammar.lisp:644)
//   abbr_seba      → :seba      (dict-grammar.lisp:647)
//   abbr_ii        → :ii        (dict-grammar.lisp:660)

abbr_suffix_dispatch!(abbr_nee_dispatch, abbr_nee);
abbr_suffix_dispatch!(abbr_nx_dispatch, abbr_nx);
abbr_suffix_dispatch!(abbr_n_dispatch, abbr_n);
abbr_suffix_dispatch!(abbr_nakereba_dispatch, abbr_nakereba);
abbr_suffix_dispatch!(abbr_shimasho_dispatch, abbr_shimasho);
abbr_suffix_dispatch!(abbr_dewanai_dispatch, abbr_dewanai);
abbr_suffix_dispatch!(abbr_teba_dispatch, abbr_teba);
abbr_suffix_dispatch!(abbr_reba_dispatch, abbr_reba);
abbr_suffix_dispatch!(abbr_keba_dispatch, abbr_keba);
abbr_suffix_dispatch!(abbr_geba_dispatch, abbr_geba);
abbr_suffix_dispatch!(abbr_neba_dispatch, abbr_neba);
abbr_suffix_dispatch!(abbr_beba_dispatch, abbr_beba);
abbr_suffix_dispatch!(abbr_meba_dispatch, abbr_meba);
abbr_suffix_dispatch!(abbr_seba_dispatch, abbr_seba);
abbr_suffix_dispatch!(abbr_ii_dispatch, abbr_ii);

/// Full port of `*suffix-list*`: 43 of 43 upstream entries (28
/// def-simple-suffix + 15 def-abbr-suffix). Keys are the lowercase
/// keyword strings already used by the suffix cache
/// (`crate::dict::_star_suffix_cache_star_`). Linear scan via
/// [`lookup_suffix_fn`] mirrors the upstream `(assoc keyword
/// *suffix-list*)`; with N = 43, the constant factor is negligible.
pub static SUFFIX_LIST: &[(&str, SuffixFn)] = &[
    // def-simple-suffix entries
    ("suru", suffix_suru_dispatch),
    ("ra", suffix_ra_dispatch),
    ("tai", suffix_tai_dispatch),
    ("ren", suffix_ren_dispatch),
    ("ren-", suffix_ren_minus_dispatch),
    ("neg", suffix_neg_dispatch),
    ("te", suffix_te_dispatch),
    ("teiru", suffix_teiru_dispatch),
    ("teiru+", suffix_teiru_plus_dispatch),
    ("te+space", suffix_te_plus_space_dispatch),
    ("kudasai", suffix_kudasai_dispatch),
    ("teren", suffix_te_ren_dispatch),
    ("teii", suffix_teii_dispatch),
    ("rou", suffix_rou_dispatch),
    ("adv", suffix_adv_dispatch),
    ("iadj", suffix_iadj_dispatch),
    ("tosuru", suffix_tosuru_dispatch),
    ("kurai", suffix_kurai_dispatch),
    ("chau", suffix_chau_dispatch),
    ("to", suffix_to_dispatch),
    ("sa", suffix_sa_dispatch),
    ("sou", suffix_sou_dispatch),
    ("sou+", suffix_sou_plus_dispatch),
    ("sugiru", suffix_sugiru_dispatch),
    ("garu", suffix_garu_dispatch),
    ("desu", suffix_desu_dispatch),
    ("desho", suffix_desho_dispatch),
    ("rashii", suffix_rashii_dispatch),
    // def-abbr-suffix entries
    ("nai", abbr_nee_dispatch),
    ("nai-x", abbr_nx_dispatch),
    ("nai-n", abbr_n_dispatch),
    ("nakereba", abbr_nakereba_dispatch),
    ("shimashou", abbr_shimasho_dispatch),
    ("dewanai", abbr_dewanai_dispatch),
    ("teba", abbr_teba_dispatch),
    ("reba", abbr_reba_dispatch),
    ("keba", abbr_keba_dispatch),
    ("geba", abbr_geba_dispatch),
    ("neba", abbr_neba_dispatch),
    ("beba", abbr_beba_dispatch),
    ("meba", abbr_meba_dispatch),
    ("seba", abbr_seba_dispatch),
    ("ii", abbr_ii_dispatch),
];

/// `(cdr (assoc keyword *suffix-list*))` — returns the dispatch fn for
/// `keyword`, or `None` when the keyword is absent.
pub fn lookup_suffix_fn(keyword: &str) -> Option<SuffixFn> {
    SUFFIX_LIST
        .iter()
        .find_map(|(k, f)| if *k == keyword { Some(*f) } else { None })
}

/// Port of `ichiran/dict:*suffix-unique-only*` (`dict-grammar.lisp:330`).
///
/// Registry of suffix classes that suppress the current suffix's
/// expansion in `find-word-suffix`, tagged with one of three match
/// behaviors (bare, `:desu`, `:sa`).
#[derive(Debug, Clone, Copy)]
pub enum SuffixUniqueOnly {
    Bare,
    Desu,
    Sa,
}

pub static SUFFIX_UNIQUE_ONLY: &[(&str, SuffixUniqueOnly)] = &[
    ("ii", SuffixUniqueOnly::Bare),
    ("seba", SuffixUniqueOnly::Bare),
    ("meba", SuffixUniqueOnly::Bare),
    ("beba", SuffixUniqueOnly::Bare),
    ("neba", SuffixUniqueOnly::Bare),
    ("geba", SuffixUniqueOnly::Bare),
    ("keba", SuffixUniqueOnly::Bare),
    ("reba", SuffixUniqueOnly::Bare),
    ("teba", SuffixUniqueOnly::Bare),
    ("eba", SuffixUniqueOnly::Bare),
    ("dewanai", SuffixUniqueOnly::Bare),
    ("nai-n", SuffixUniqueOnly::Bare),
    ("gai", SuffixUniqueOnly::Bare),
    ("nikui", SuffixUniqueOnly::Bare),
    ("mo", SuffixUniqueOnly::Bare),
    ("desu", SuffixUniqueOnly::Desu),
    ("ra", SuffixUniqueOnly::Bare),
    ("sa", SuffixUniqueOnly::Sa),
];

#[cfg(test)]
mod tests;
