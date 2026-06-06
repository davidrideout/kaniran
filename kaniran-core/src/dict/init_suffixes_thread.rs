//! Transliteration of `ichiran/dict:init-suffixes-thread` (`dict-grammar.lisp:169`).
//!
//! Populator for the suffix cache / suffix class maps — runs the
//! `load_conjs` / `load_kf` / `load_abbr` callsites and returns the
//! built maps.

use crate::conn::kani_context::KaniranContext;
use crate::dict::_star_suffix_cache_star_::SuffixCache;
use crate::dict::_star_suffix_class_star_::SuffixClass;
use crate::dict::find_word_conj_of::find_word_conj_of;
use crate::dict::find_word_seq::WordSeqRows;
use crate::dict::get_kana_form::get_kana_form;
use crate::dict::get_kana_forms::get_kana_forms;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::simple_text_class::WordConjugations;

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
    fn update_suffix_cache(
        &mut self,
        text: &str,
        new: (String, Option<KanaText>),
        join: bool,
    ) {
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

    b.load_conjs(ctx, "ren-", 2772730, Some("nikui"), false).await?;
    b.load_conjs(ctx, "ren-", 2867504, Some("gatai"), false).await?;

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
        let key = if tkf.chars().count() > 1 { "teiru+" } else { "teiru" };
        b.cache.insert(tkf.clone(), vec![(key.to_string(), Some(kf.clone()))]);
        b.class.insert(kf.seq, "iru".to_string());
        if tkf.chars().count() > 1 {
            // text[1..] in Lisp is `(subseq tkf 1)` — drop the first
            // character. Use `chars().skip(1)` to mirror character-
            // index semantics for multi-byte UTF-8.
            let short: String = tkf.chars().skip(1).collect();
            b.cache.insert(short, vec![("teiru".to_string(), Some(kf))]);
        }
    }

    b.load_conjs(ctx, "te", 1547720, Some("kuru"), false).await?; // くる

    b.load_conjs(ctx, "te", 1421850, Some("oku"), false).await?; // おく
    b.load_conjs(ctx, "to", 2108590, Some("oku"), false).await?; // とく

    b.load_conjs(ctx, "te", 1305380, Some("chau"), false).await?; // しまう

    b.load_conjs(ctx, "te+space", 1269130, Some("kureru"), false).await?; // くれる
    b.load_conjs(ctx, "te+space", 1535910, Some("morau"), false).await?; // もらう
    b.load_conjs(ctx, "te+space", 1587290, Some("itadaku"), false).await?; // いただく

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

    let kudasai_kf = require_kana_form(
        ctx, 1184270, "ください", Some(WordConjugations::Root),
    ).await?;
    b.load_kf("kudasai", kudasai_kf, None, None, false);

    b.load_conjs(ctx, "suru", 1157170, None, false).await?; // する
    b.load_conjs(ctx, "suru", 1421900, Some("itasu"), false).await?; // いたす
    b.load_conjs(ctx, "suru", 2269820, Some("sareru"), false).await?; // される
    b.load_conjs(ctx, "suru", 1005160, Some("saseru"), false).await?; // させる

    b.load_conjs(ctx, "sou", 1006610, None, false).await?; // そう
    b.load_conjs(ctx, "sou+", 2141080, None, false).await?; // そうにない

    let darou_kf = require_kana_form(ctx, 1928670, "だろう", None).await?;
    b.load_kf("rou", darou_kf, None, Some("ろう"), false);

    b.load_conjs(ctx, "sugiru", 1195970, None, false).await?; // すぎる

    let sa_kf = require_kana_form(ctx, 2029120, "さ", None).await?;
    b.load_kf("sa", sa_kf, None, None, false);

    let tsutsu_kf = require_kana_form(ctx, 1008120, "つつ", None).await?;
    b.load_kf("ren", tsutsu_kf, Some("tsutsu"), None, false);
    b.load_conjs(ctx, "ren", 2027910, Some("tsutsuaru"), false).await?;

    let uru_kf = require_kana_form(ctx, 1454500, "うる", None).await?;
    b.load_kf("ren", uru_kf, Some("uru"), None, false);

    // (load-kf :neg (car (find-word-conj-of "なく" 1529520)) :class :nai)
    let naku_rows = find_word_conj_of(ctx, "なく", &[1529520]).await?;
    let naku_kf = match naku_rows {
        WordSeqRows::Kana(mut v) => v.drain(..).next().ok_or(sqlx::Error::RowNotFound)?,
        WordSeqRows::Kanji(_) => unreachable!("'なく' is kana"),
    };
    b.load_kf("neg", naku_kf, Some("nai"), None, false);

    b.load_conjs(ctx, "adv", 1375610, Some("naru"), false).await?; // なる

    b.load_conjs(ctx, "teren", 1012740, Some("yagaru"), false).await?;

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
