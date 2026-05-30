//! Port of the dict-grammar.lisp suffix-init layer.

use crate::conn::kani_context::KaniranContext;
use crate::dict::dao::KanaText;
use crate::dict::grammar::abbr::{
    abbr_beba, abbr_dewanai, abbr_geba, abbr_ii, abbr_keba, abbr_meba, abbr_n, abbr_nakereba,
    abbr_neba, abbr_nee, abbr_nx, abbr_reba, abbr_seba, abbr_shimasho, abbr_teba,
};
use crate::dict::grammar::find_word::{
    find_word_conj_of, get_kana_form, get_kana_forms, WordSeqRows,
};
use crate::dict::grammar::suffix_rules::{
    suffix_adv, suffix_chau, suffix_desho, suffix_desu, suffix_garu, suffix_iadj, suffix_kudasai,
    suffix_kurai, suffix_neg, suffix_ra, suffix_rashii, suffix_ren, suffix_ren_, suffix_rou,
    suffix_sa, suffix_sou, suffix_sou_plus_, suffix_sugiru, suffix_suru, suffix_tai, suffix_te,
    suffix_te_plus_space, suffix_te_ren, suffix_teii, suffix_teiru, suffix_teiru_plus_, suffix_to,
    suffix_tosuru,
};
use crate::dict::kani::KaniWordDispatchEnum;
use crate::dict::text_classes::WordConjugations;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::OnceLock;

pub type SuffixCache = HashMap<String, Vec<(String, Option<KanaText>)>>;

pub fn suffix_cache(ctx: &KaniranContext) -> &SuffixCache {
    &ctx.suffix_cache
}

pub type SuffixClass = HashMap<i32, String>;

pub fn suffix_class(ctx: &KaniranContext) -> &SuffixClass {
    &ctx.suffix_class
}

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

#[derive(Default)]
struct SuffixCacheBuilder {
    cache: SuffixCache,
    class: SuffixClass,
}

impl SuffixCacheBuilder {
    /// Port of the labels-local `update-suffix-cache` at
    /// `dict-grammar.lisp:172-183`. The two `b.cache.insert(…)` direct
    /// writes in `build_suffix_caches` (the いる block at L141-153 and
    /// the いく/く block at L168-178) bypass this helper to match the
    /// upstream direct `(setf (gethash …))` at
    /// `dict-grammar.lisp:210-215, 233-236` — both behaviors are
    /// parity-preserved by mirroring the bypass, not by routing through
    /// `update_suffix_cache`.
    fn update_suffix_cache(&mut self, text: &str, new: (String, Option<KanaText>), join: bool) {
        match self.cache.get_mut(text) {
            None => {
                self.cache.insert(text.to_string(), vec![new]);
            }
            Some(existing) if join => {
                existing.insert(0, new);
            }
            Some(existing) => {
                *existing = vec![new];
            }
        }
    }

    fn load_kf(
        &mut self,
        key: &str,
        kf: KanaText,
        class: Option<&str>,
        text: Option<&str>,
        join: bool,
    ) {
        let resolved_text = text.unwrap_or(&kf.text).to_string();
        let resolved_class = class.unwrap_or(key).to_string();
        let kf_seq = kf.seq;
        self.update_suffix_cache(&resolved_text, (key.to_string(), Some(kf)), join);
        self.class.insert(kf_seq, resolved_class);
    }

    async fn load_conjs(
        &mut self,
        ctx: &KaniranContext,
        key: &str,
        seq: i32,
        class: Option<&str>,
        join: bool,
    ) -> Result<(), sqlx::Error> {
        let kfs = get_kana_forms(ctx, seq).await?;
        for kf in kfs {
            self.load_kf(key, kf, class, None, join);
        }
        Ok(())
    }

    fn load_abbr(&mut self, key: &str, text: &str, join: bool) {
        self.update_suffix_cache(text, (key.to_string(), None), join);
    }
}

/// Look up a `(seq, text)` kana_form row that the populator depends on
/// being present. Missing means the JMdict dump is incomplete; surface
/// it as a sqlx::Error::RowNotFound so the construction error path
/// reports meaningfully.
async fn require_kana_form(
    ctx: &KaniranContext,
    seq: i32,
    text: &str,
    conj: Option<WordConjugations>,
) -> Result<KanaText, sqlx::Error> {
    get_kana_form(ctx, seq, text, conj)
        .await?
        .ok_or(sqlx::Error::RowNotFound)
}

pub async fn build_suffix_caches(
    ctx: &KaniranContext,
) -> Result<(SuffixCache, SuffixClass), sqlx::Error> {
    let mut b = SuffixCacheBuilder::default();

    // ちゃう
    b.load_conjs(ctx, "chau", 2013800, None, false).await?;
    // ちまう
    b.load_conjs(ctx, "chau", 2210750, None, false).await?;
    // (load-kf :chau (get-kana-form 2028920 "は") :class :ha :text "ちゃ"/"じゃ")
    let ha_kf = require_kana_form(ctx, 2028920, "は", None).await?;
    b.load_kf("chau", ha_kf.clone(), Some("ha"), Some("ちゃ"), false);
    b.load_kf("chau", ha_kf, Some("ha"), Some("じゃ"), false);

    b.load_conjs(ctx, "tai", 2017560, None, false).await?;
    // たそう (synthetic seq 900000)
    let tasou_kf = require_kana_form(ctx, 900000, "たそう", None).await?;
    b.load_kf("tai", tasou_kf, Some("tasou"), None, false);

    b.load_conjs(ctx, "ren-", 2772730, Some("nikui"), false)
        .await?;
    b.load_conjs(ctx, "ren-", 2867504, Some("gatai"), false)
        .await?;

    b.load_conjs(ctx, "te", 1577985, Some("oru"), false).await?; // おる
    b.load_conjs(ctx, "te", 1296400, Some("aru"), false).await?; // ある

    // いる (る) — direct setf with teiru / teiru+ split.
    // Mirrors dict-grammar.lisp:210-215: upstream writes the long-form
    // and (unconditionally for any tkf-length > 1) the short variant
    // straight via `(setf (gethash …))`, never routing through the
    // labels-local `update-suffix-cache` — see this file's helper
    // doc-comment for the parity rationale.
    let iru_kfs = get_kana_forms(ctx, 1577980).await?;
    for kf in iru_kfs {
        let tkf = kf.text.clone();
        let key = if tkf.chars().count() > 1 {
            "teiru+"
        } else {
            "teiru"
        };
        b.cache
            .insert(tkf.clone(), vec![(key.to_string(), Some(kf.clone()))]);
        b.class.insert(kf.seq, "iru".to_string());
        if tkf.chars().count() > 1 {
            // text[1..] in Lisp is `(subseq tkf 1)` — drop the first
            // character. Use `chars().skip(1)` to mirror character-
            // index semantics for multi-byte UTF-8.
            let short: String = tkf.chars().skip(1).collect();
            b.cache.insert(short, vec![("teiru".to_string(), Some(kf))]);
        }
    }

    b.load_conjs(ctx, "te", 1547720, Some("kuru"), false)
        .await?; // くる

    b.load_conjs(ctx, "te", 1421850, Some("oku"), false).await?; // おく
    b.load_conjs(ctx, "to", 2108590, Some("oku"), false).await?; // とく

    b.load_conjs(ctx, "te", 1305380, Some("chau"), false)
        .await?; // しまう

    b.load_conjs(ctx, "te+space", 1269130, Some("kureru"), false)
        .await?; // くれる
    b.load_conjs(ctx, "te+space", 1535910, Some("morau"), false)
        .await?; // もらう
    b.load_conjs(ctx, "te+space", 1587290, Some("itadaku"), false)
        .await?; // いただく

    // いく/く — direct setf, gated on first char being い (HIRAGANA_LETTER_I).
    // Mirrors dict-grammar.lisp:233-236: upstream writes the long form
    // unconditionally and the short form only `unless (gethash short …)`
    // — i.e. first-write-wins for the short variant — bypassing
    // `update-suffix-cache`. The `b.cache.entry(short).or_insert(val)`
    // below pins that "only if absent" semantics.
    let iku_kfs = get_kana_forms(ctx, 1578850).await?;
    for kf in iku_kfs {
        let tkf = kf.text.clone();
        if tkf.chars().next() != Some('\u{3044}') {
            continue;
        }
        let val = vec![("te".to_string(), Some(kf.clone()))];
        b.cache.insert(tkf.clone(), val.clone());
        b.class.insert(kf.seq, "iku".to_string());
        let short: String = tkf.chars().skip(1).collect();
        b.cache.entry(short).or_insert(val);
    }

    let ii_kf = require_kana_form(ctx, 2820690, "いい", None).await?;
    b.load_kf("teii", ii_kf, Some("ii"), None, false);
    let moii_kf = require_kana_form(ctx, 900001, "もいい", None).await?;
    b.load_kf("teii", moii_kf, Some("ii"), Some("もいい"), false);
    let mo_kf = require_kana_form(ctx, 2028940, "も", None).await?;
    b.load_kf("te", mo_kf, Some("mo"), None, false);

    let kudasai_kf =
        require_kana_form(ctx, 1184270, "ください", Some(WordConjugations::Root)).await?;
    b.load_kf("kudasai", kudasai_kf, None, None, false);

    b.load_conjs(ctx, "suru", 1157170, None, false).await?; // する
    b.load_conjs(ctx, "suru", 1421900, Some("itasu"), false)
        .await?; // いたす
    b.load_conjs(ctx, "suru", 2269820, Some("sareru"), false)
        .await?; // される
    b.load_conjs(ctx, "suru", 1005160, Some("saseru"), false)
        .await?; // させる

    b.load_conjs(ctx, "sou", 1006610, None, false).await?; // そう
    b.load_conjs(ctx, "sou+", 2141080, None, false).await?; // そうにない

    let darou_kf = require_kana_form(ctx, 1928670, "だろう", None).await?;
    b.load_kf("rou", darou_kf, None, Some("ろう"), false);

    b.load_conjs(ctx, "sugiru", 1195970, None, false).await?; // すぎる

    let sa_kf = require_kana_form(ctx, 2029120, "さ", None).await?;
    b.load_kf("sa", sa_kf, None, None, false);

    let tsutsu_kf = require_kana_form(ctx, 1008120, "つつ", None).await?;
    b.load_kf("ren", tsutsu_kf, Some("tsutsu"), None, false);
    b.load_conjs(ctx, "ren", 2027910, Some("tsutsuaru"), false)
        .await?;

    let uru_kf = require_kana_form(ctx, 1454500, "うる", None).await?;
    b.load_kf("ren", uru_kf, Some("uru"), None, false);

    // (load-kf :neg (car (find-word-conj-of "なく" 1529520)) :class :nai)
    let naku_rows = find_word_conj_of(ctx, "なく", &[1529520]).await?;
    let naku_kf = match naku_rows {
        WordSeqRows::Kana(mut v) => v.drain(..).next().ok_or(sqlx::Error::RowNotFound)?,
        WordSeqRows::Kanji(_) => unreachable!("'なく' is kana"),
    };
    b.load_kf("neg", naku_kf, Some("nai"), None, false);

    b.load_conjs(ctx, "adv", 1375610, Some("naru"), false)
        .await?; // なる

    b.load_conjs(ctx, "teren", 1012740, Some("yagaru"), false)
        .await?;

    let ra_kf = require_kana_form(ctx, 2067770, "ら", None).await?;
    b.load_kf("ra", ra_kf, None, None, false);

    b.load_conjs(ctx, "rashii", 1013240, None, false).await?;

    let desu_kf = require_kana_form(ctx, 1628500, "です", None).await?;
    b.load_kf("desu", desu_kf, None, None, false);

    let deshou_kf = require_kana_form(ctx, 1008420, "でしょう", None).await?;
    b.load_kf("desho", deshou_kf, None, None, false);
    let desho_kf = require_kana_form(ctx, 1008420, "でしょ", None).await?;
    b.load_kf("desho", desho_kf, None, None, false);

    b.load_conjs(ctx, "tosuru", 2136890, None, false).await?; // とする

    let kurai_kf = require_kana_form(ctx, 1154340, "くらい", None).await?;
    b.load_kf("kurai", kurai_kf, None, None, false);
    let gurai_kf = require_kana_form(ctx, 1154340, "ぐらい", None).await?;
    b.load_kf("kurai", gurai_kf, None, None, false);

    b.load_conjs(ctx, "garu", 1631750, None, false).await?; // がる

    let gachi_kf = require_kana_form(ctx, 2016470, "がち", None).await?;
    b.load_kf("ren", gachi_kf, Some("gachi"), None, false);

    let ge_kf = require_kana_form(ctx, 2006580, "げ", None).await?;
    b.load_kf("iadj", ge_kf, None, None, false);
    let me_kf = require_kana_form(ctx, 1604890, "め", None).await?;
    b.load_kf("iadj", me_kf, Some("me"), None, false);

    let gai_kf = require_kana_form(ctx, 2606690, "がい", None).await?;
    b.load_kf("ren-", gai_kf, Some("gai"), None, false);

    // load-abbr block
    b.load_abbr("nai", "ねえ", false);
    b.load_abbr("nai", "ねぇ", false);
    b.load_abbr("nai", "ねー", false);
    b.load_abbr("nai-x", "ず", false);
    b.load_abbr("nai-x", "ざる", false);
    b.load_abbr("nai-x", "ぬ", false);
    b.load_abbr("nai-n", "ん", false);

    b.load_abbr("nakereba", "なきゃ", false);
    b.load_abbr("nakereba", "なくちゃ", false);
    b.load_abbr("nakereba", "ねば", false);

    b.load_abbr("teba", "ちゃ", true); // つ — only `:join t` callsite
    b.load_abbr("reba", "りゃ", false);
    b.load_abbr("keba", "きゃ", false);
    b.load_abbr("geba", "ぎゃ", false);
    b.load_abbr("neba", "にゃ", false);
    b.load_abbr("beba", "びゃ", false);
    b.load_abbr("meba", "みゃ", false);
    b.load_abbr("seba", "しゃ", false);

    b.load_abbr("shimashou", "しましょ", false);
    b.load_abbr("dewanai", "じゃない", false);

    b.load_abbr("ii", "ええ", false);

    Ok((b.cache, b.class))
}

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
/// keyword strings already used by [`SuffixCache`]. Linear scan via
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

pub fn get_suffix_description(ctx: &KaniranContext, seq: i32) -> Option<&'static str> {
    let key = match suffix_class(ctx).get(&seq) {
        Some(class) => SuffixDescKey::Class(class.clone()),
        None => SuffixDescKey::Seq(seq),
    };
    suffix_description().get(&key).copied()
}

#[cfg(test)]
mod test__star_suffix_description_star {
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
        assert_eq!(
            map.get(&SuffixDescKey::Seq(2826528)).copied(),
            Some("polite prefix")
        );
        assert_eq!(
            map.get(&SuffixDescKey::Seq(2028980)).copied(),
            Some("at / in / by")
        );
        assert_eq!(
            map.get(&SuffixDescKey::Seq(1002980)).copied(),
            Some("from / because")
        );

        // miss
        assert_eq!(
            map.get(&SuffixDescKey::Class("nonexistent".to_string()))
                .copied(),
            None
        );
        assert_eq!(map.get(&SuffixDescKey::Seq(0)).copied(), None);
    }

    /// Pin the class/seq partition counts so adding/removing
    /// entries on one side trips the test.
    #[test]
    fn class_seq_partition() {
        let map = suffix_description();
        let class_count = map
            .keys()
            .filter(|k| matches!(k, SuffixDescKey::Class(_)))
            .count();
        let seq_count = map
            .keys()
            .filter(|k| matches!(k, SuffixDescKey::Seq(_)))
            .count();
        assert_eq!(class_count, 39);
        assert_eq!(seq_count, 8);
    }
}

#[cfg(test)]
mod test__star_suffix_list_star {
    use super::*;

    /// Every upstream-published keyword resolves through `lookup_suffix_fn`.
    /// Pins the full set so a row removal regresses visibly.
    #[test]
    fn registered_keys_resolve() {
        // def-simple-suffix
        for key in [
            "suru", "ra", "tai", "ren", "ren-", "neg", "te", "teiru", "teiru+", "te+space",
            "kudasai", "teren", "teii", "rou", "adv", "iadj", "tosuru", "kurai", "chau", "to",
            "sa", "sou", "sou+", "sugiru", "garu", "desu", "desho", "rashii",
        ] {
            assert!(lookup_suffix_fn(key).is_some(), "missing key: {}", key);
        }
        // def-abbr-suffix
        for key in [
            "nai",
            "nai-x",
            "nai-n",
            "nakereba",
            "shimashou",
            "dewanai",
            "teba",
            "reba",
            "keba",
            "geba",
            "neba",
            "beba",
            "meba",
            "seba",
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

#[cfg(test)]
mod test_get_suffix_description {
    use super::*;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
    }

    /// REPL (.103, after `(init-suffixes t)`):
    /// `(get-suffix-description seq)` across all four lookup paths —
    /// seq→class→desc, seq→class→no-desc, seq→direct-key→desc, and
    /// miss on both.
    #[tokio::test]
    async fn get_suffix_description_paths() {
        let ctx = ctx().await;
        let cases: &[(i32, Option<&str>)] = &[
            // seq in *suffix-class*, class has a description
            (2013800, Some("indicates completion (to finish ...)")), // :chau
            (2017560, Some("want to... / would like to...")),        // :tai
            (2028920, Some("topic marker particle")),                // :ha
            (1006610, Some("looking like ... / seeming ...")),       // :sou
            // seq in *suffix-class*, class has no description
            (2141080, None), // :sou+
            // seq not in *suffix-class*, seq is a direct *suffix-description* key
            (2826528, Some("polite prefix")),
            (2028980, Some("at / in / by")),
            (1002980, Some("from / because")),
            // in neither table
            (1005530, None),
            (99999999, None),
        ];
        for (seq, expected) in cases {
            assert_eq!(get_suffix_description(&ctx, *seq), *expected, "seq={seq}");
        }
    }
}
