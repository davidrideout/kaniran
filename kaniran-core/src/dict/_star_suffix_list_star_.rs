//! Port of `ichiran/dict:*suffix-list*` (`dict-grammar.lisp:329`).
//!
//! ```lisp
//! (defparameter *suffix-list* nil)
//! ```
//!
//! Alist mapping a suffix keyword (the `:tag` cached alongside a
//! reading in `*suffix-cache*`) to the function that materializes
//! compound-text / proxy-text candidates for that suffix. Upstream is
//! populated at load time by `(defsuffix ...)` / `(def-simple-suffix
//! ...)` / `(def-abbr-suffix ...)` forms, each appending one
//! `(cons keyword fn-name)` pair via `(pushnew (cons ,key ',name)
//! *suffix-list*)`. The full Lisp table has 43 entries; this port now
//! carries all 43 — the 28 `def-simple-suffix` bodies (suru / ra / tai
//! / ren / ren- / neg / te / teiru / teiru+ / te+space / kudasai /
//! teren / teii / rou / adv / iadj / tosuru / kurai / chau / to / sa
//! / sou / sou+ / sugiru / garu / desu / desho / rashii) and the 15
//! `def-abbr-suffix` bodies (nai / nai-x / nai-n / nakereba /
//! shimashou / dewanai / teba / reba / keba / geba / neba / beba /
//! meba / seba / ii).
//!
//! Consumed by `find-word-suffix` (`dict-grammar.lisp:695-707`):
//! `(cdr (assoc keyword *suffix-list*))` returns the function (or
//! `nil`); the loop's `(when suffix-fn …)` gate silently drops any
//! suffix whose keyword has no entry — matching upstream behavior for
//! a suffix-list missing rows.
//!
//! ## Divergences from Lisp
//!
//! - **Sidecar `fn` pointer table.** The Lisp value is an alist of
//!   `(keyword . symbol-naming-a-fn)` pairs. The Rust mirror uses a
//!   `&'static [(&'static str, SuffixFn)]` slice where each entry's
//!   second element is a `fn` adapter that boxes the suffix-fn's
//!   returned future. Keys are the lowercase keyword strings that
//!   `*suffix-cache*` already stores (e.g. `"suru"`, `"ra"`, `"nai"`),
//!   matching [`super::_star_suffix_cache_star_::SuffixCache`]'s value
//!   shape.
//! - **`SuffixFn` returns `Vec<KaniWordDispatchEnum>`.** Upstream's
//!   defsuffix bodies return `(list of (or compound-text proxy-text))`
//!   — `def-simple-suffix` always emits compound-text via `adjoin-word`
//!   (`dict.lisp:643`), but `def-abbr-suffix`'s `etypecase` arm at
//!   `dict-grammar.lisp:565-577` produces a `proxy-text`
//!   (`make-instance 'proxy-text :source pw …`) when the primary word
//!   is a simple-text. The Rust mirror flattens both arms into the
//!   `KaniWordDispatchEnum::{Compound,Proxy}` polymorphic enum so
//!   find-word-suffix can append heterogeneous rows without losing
//!   shape.
//! - **Adapter `kf` unwrap policy.** `def-simple-suffix` bodies were
//!   ported with non-`Option<&KanaText>` `kf` parameters (the cache
//!   invariant `(load-conjs :key …)` / `(load-kf :key …)` always
//!   produces a kana-text under those keywords). The dispatch adapters
//!   accept `Option<&KanaText>` to match the uniform
//!   `(funcall suffix-fn root suf kf)` shape at the find-word-suffix
//!   callsite (`dict-grammar.lisp:707`) and `.expect(...)` on the
//!   simple-suffix branch — a `None` would mean a cache regression and
//!   surfaces at the boundary rather than corrupting a downstream
//!   lookup. `def-abbr-suffix` adapters pass through `Option<&KanaText>`
//!   since the upstream `(load-abbr …)` populator routinely caches
//!   abbreviated suffixes without a backing kana-text (the `kf`
//!   parameter in `def-abbr-suffix` is `(declare (ignore ,suf))`).
//!
//! ## Future suffix-fn registrations
//!
//! All 43 upstream entries are wired today. If upstream adds another
//! `defsuffix` / `def-simple-suffix` / `def-abbr-suffix`, add an
//! adapter fn following one of the two patterns below (compound-only
//! vs compound+proxy) and append a row to [`SUFFIX_LIST`] keyed by
//! the lowercase keyword the upstream form passes as `key`.

use std::future::Future;
use std::pin::Pin;

use crate::conn::kani_context::KaniranContext;
use crate::dict::abbr_beba::abbr_beba;
use crate::dict::abbr_dewanai::abbr_dewanai;
use crate::dict::abbr_geba::abbr_geba;
use crate::dict::abbr_ii::abbr_ii;
use crate::dict::abbr_keba::abbr_keba;
use crate::dict::abbr_meba::abbr_meba;
use crate::dict::abbr_n::abbr_n;
use crate::dict::abbr_nakereba::abbr_nakereba;
use crate::dict::abbr_neba::abbr_neba;
use crate::dict::abbr_nee::abbr_nee;
use crate::dict::abbr_nx::abbr_nx;
use crate::dict::abbr_reba::abbr_reba;
use crate::dict::abbr_seba::abbr_seba;
use crate::dict::abbr_shimasho::abbr_shimasho;
use crate::dict::abbr_teba::abbr_teba;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kani::KaniWordDispatchEnum;
use crate::dict::suffix_adv::suffix_adv;
use crate::dict::suffix_chau::suffix_chau;
use crate::dict::suffix_desho::suffix_desho;
use crate::dict::suffix_desu::suffix_desu;
use crate::dict::suffix_garu::suffix_garu;
use crate::dict::suffix_iadj::suffix_iadj;
use crate::dict::suffix_kudasai::suffix_kudasai;
use crate::dict::suffix_kurai::suffix_kurai;
use crate::dict::suffix_neg::suffix_neg;
use crate::dict::suffix_ra::suffix_ra;
use crate::dict::suffix_rashii::suffix_rashii;
use crate::dict::suffix_ren::suffix_ren;
use crate::dict::suffix_ren_::suffix_ren_;
use crate::dict::suffix_rou::suffix_rou;
use crate::dict::suffix_sa::suffix_sa;
use crate::dict::suffix_sou::suffix_sou;
use crate::dict::suffix_sou_plus_::suffix_sou_plus_;
use crate::dict::suffix_sugiru::suffix_sugiru;
use crate::dict::suffix_suru::suffix_suru;
use crate::dict::suffix_tai::suffix_tai;
use crate::dict::suffix_te::suffix_te;
use crate::dict::suffix_te_plus_space::suffix_te_plus_space;
use crate::dict::suffix_te_ren::suffix_te_ren;
use crate::dict::suffix_teii::suffix_teii;
use crate::dict::suffix_teiru::suffix_teiru;
use crate::dict::suffix_teiru_plus_::suffix_teiru_plus_;
use crate::dict::suffix_to::suffix_to;
use crate::dict::suffix_tosuru::suffix_tosuru;

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
) -> Pin<Box<dyn Future<Output = Result<Vec<KaniWordDispatchEnum>, sqlx::Error>> + Send + 'a>>;

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

simple_suffix_dispatch!(suffix_suru_dispatch,        suffix_suru,         "suru",     "load-conjs :suru");
simple_suffix_dispatch!(suffix_ra_dispatch,          suffix_ra,           "ra",       "load-kf :ra");
simple_suffix_dispatch!(suffix_tai_dispatch,         suffix_tai,          "tai",      "load-conjs :tai");
simple_suffix_dispatch!(suffix_ren_dispatch,         suffix_ren,          "ren",      "load-kf :ren");
simple_suffix_dispatch!(suffix_ren_minus_dispatch,   suffix_ren_,         "ren-",     "load-conjs :ren-");
simple_suffix_dispatch!(suffix_neg_dispatch,         suffix_neg,          "neg",      "load-kf :neg");
simple_suffix_dispatch!(suffix_te_dispatch,          suffix_te,           "te",       "load-conjs :te / load-kf :te");
simple_suffix_dispatch!(suffix_teiru_dispatch,       suffix_teiru,        "teiru",    "いる(る) loop");
simple_suffix_dispatch!(suffix_teiru_plus_dispatch,  suffix_teiru_plus_,  "teiru+",   "いる(る) loop");
simple_suffix_dispatch!(suffix_te_plus_space_dispatch, suffix_te_plus_space, "te+space", "load-conjs :te+space");
simple_suffix_dispatch!(suffix_kudasai_dispatch,     suffix_kudasai,      "kudasai",  "load-kf :kudasai");
simple_suffix_dispatch!(suffix_te_ren_dispatch,      suffix_te_ren,       "teren",    "load-conjs :teren");
simple_suffix_dispatch!(suffix_teii_dispatch,        suffix_teii,         "teii",     "load-kf :teii");
simple_suffix_dispatch!(suffix_rou_dispatch,         suffix_rou,          "rou",      "load-kf :rou");
simple_suffix_dispatch!(suffix_adv_dispatch,         suffix_adv,          "adv",      "load-conjs :adv");
simple_suffix_dispatch!(suffix_iadj_dispatch,        suffix_iadj,         "iadj",     "load-kf :iadj");
simple_suffix_dispatch!(suffix_tosuru_dispatch,      suffix_tosuru,       "tosuru",   "load-conjs :tosuru");
simple_suffix_dispatch!(suffix_kurai_dispatch,       suffix_kurai,        "kurai",    "load-kf :kurai");
simple_suffix_dispatch!(suffix_chau_dispatch,        suffix_chau,         "chau",     "load-conjs :chau");
simple_suffix_dispatch!(suffix_to_dispatch,          suffix_to,           "to",       "load-conjs :to");
simple_suffix_dispatch!(suffix_sa_dispatch,          suffix_sa,           "sa",       "load-kf :sa");
simple_suffix_dispatch!(suffix_sou_dispatch,         suffix_sou,          "sou",      "load-kf :sou");
simple_suffix_dispatch!(suffix_sou_plus_dispatch,    suffix_sou_plus_,    "sou+",     "load-kf :sou+");
simple_suffix_dispatch!(suffix_sugiru_dispatch,      suffix_sugiru,       "sugiru",   "load-conjs :sugiru");
simple_suffix_dispatch!(suffix_garu_dispatch,        suffix_garu,         "garu",     "load-conjs :garu");
simple_suffix_dispatch!(suffix_desu_dispatch,        suffix_desu,         "desu",     "load-kf :desu");
simple_suffix_dispatch!(suffix_desho_dispatch,       suffix_desho,        "desho",    "load-kf :desho");
simple_suffix_dispatch!(suffix_rashii_dispatch,      suffix_rashii,       "rashii",   "load-kf :rashii");

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

abbr_suffix_dispatch!(abbr_nee_dispatch,       abbr_nee);
abbr_suffix_dispatch!(abbr_nx_dispatch,        abbr_nx);
abbr_suffix_dispatch!(abbr_n_dispatch,         abbr_n);
abbr_suffix_dispatch!(abbr_nakereba_dispatch,  abbr_nakereba);
abbr_suffix_dispatch!(abbr_shimasho_dispatch,  abbr_shimasho);
abbr_suffix_dispatch!(abbr_dewanai_dispatch,   abbr_dewanai);
abbr_suffix_dispatch!(abbr_teba_dispatch,      abbr_teba);
abbr_suffix_dispatch!(abbr_reba_dispatch,      abbr_reba);
abbr_suffix_dispatch!(abbr_keba_dispatch,      abbr_keba);
abbr_suffix_dispatch!(abbr_geba_dispatch,      abbr_geba);
abbr_suffix_dispatch!(abbr_neba_dispatch,      abbr_neba);
abbr_suffix_dispatch!(abbr_beba_dispatch,      abbr_beba);
abbr_suffix_dispatch!(abbr_meba_dispatch,      abbr_meba);
abbr_suffix_dispatch!(abbr_seba_dispatch,      abbr_seba);
abbr_suffix_dispatch!(abbr_ii_dispatch,        abbr_ii);

/// Full port of `*suffix-list*`: 43 of 43 upstream entries (28
/// def-simple-suffix + 15 def-abbr-suffix). Keys are the lowercase
/// keyword strings already used by the suffix cache
/// (`super::_star_suffix_cache_star_`). Linear scan via
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Every upstream-published keyword resolves through `lookup_suffix_fn`.
    /// Pins the full set so a row removal regresses visibly.
    #[test]
    fn registered_keys_resolve() {
        // def-simple-suffix
        for key in [
            "suru", "ra", "tai", "ren", "ren-", "neg", "te", "teiru", "teiru+",
            "te+space", "kudasai", "teren", "teii", "rou", "adv", "iadj",
            "tosuru", "kurai", "chau", "to", "sa", "sou", "sou+", "sugiru",
            "garu", "desu", "desho", "rashii",
        ] {
            assert!(lookup_suffix_fn(key).is_some(), "missing key: {}", key);
        }
        // def-abbr-suffix
        for key in [
            "nai", "nai-x", "nai-n", "nakereba", "shimashou", "dewanai",
            "teba", "reba", "keba", "geba", "neba", "beba", "meba", "seba",
            "ii",
        ] {
            assert!(lookup_suffix_fn(key).is_some(), "missing key: {}", key);
        }
    }

    #[test]
    fn unregistered_keys_return_none() {
        assert!(lookup_suffix_fn("").is_none());
        assert!(lookup_suffix_fn("unknown").is_none());
    }

    #[test]
    fn entry_count_matches_upstream() {
        assert_eq!(SUFFIX_LIST.len(), 43);
    }
}
