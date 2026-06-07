use crate::characters::char_class::CharClass;
use crate::characters::kana::destem;
use crate::conn::kani_context::KaniranContext;
use crate::dict::accessors::adjoin_word;
use crate::dict::text_classes::{CompoundText, ScoreMod};
use crate::dict::readings::FindWordRows;
use crate::dict::accessors::get_kana;
use crate::dict::grammar::lookup::{
    find_word_seq, find_word_with_conj_prop, find_word_with_conj_type, find_word_with_pos,
    find_word_with_suffix, or_as_hiragana, pair_words_by_conj, OrAsHiraganaFinder,
    OrAsHiraganaRows, WordSeqRows, WordWithPosRows,
};
use crate::dict::dao::KanaText;
use crate::dict::kani_word::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use std::sync::Arc;

/// Port of `ichiran/dict:def-simple-suffix` (`dict-grammar.lisp:340-368`).
///
/// Shared body for the simple-suffix definers: maps each primary word
/// through `adjoin-word`, building a compound whose text/kana splice
/// the root, destemmed reading, connector, and suffix, carrying the
/// macro's `score` and optional `score-base`.
/// Mirrors the macro's `(when (listp pw) (setf score-base (second pw)
/// pw (first pw)))` two-shape input. A bare word is `Bare(w)`; a
/// `(word score-base)` cons cell from `pair-words-by-conj` style
/// producers is `WithScoreBase(w, base)`.
pub enum PrimaryWord {
    Bare(KaniWordDispatchEnum),
    WithScoreBase(KaniWordDispatchEnum, KaniWordDispatchEnum),
}

impl From<KaniWordDispatchEnum> for PrimaryWord {
    fn from(word: KaniWordDispatchEnum) -> Self {
        PrimaryWord::Bare(word)
    }
}

/// The `def-simple-suffix` macro's keyword arguments + the
/// `patch-var` slot. `patch` mirrors a non-nil `,patch-var` value
/// `(car . cdr)`; the helper's kana branch is
/// `(destem k (length car)) + cdr` for `Some((car, cdr))` and
/// `(destem k stem)` for `None`.
pub struct DefSimpleSuffixOpts<'a> {
    pub stem: usize,
    pub score: ScoreMod,
    pub connector: &'a str,
    pub patch: Option<(&'a str, &'a str)>,
}

pub async fn def_simple_suffix_body(
    ctx: &KaniranContext,
    primary_words: Vec<PrimaryWord>,
    root: &str,
    suffix: &str,
    kf: &KanaText,
    opts: &DefSimpleSuffixOpts<'_>,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    let mut out = Vec::with_capacity(primary_words.len());
    for entry in primary_words {
        // dict-grammar.lisp:352-354 (when (listp pw) (setf score-base (second pw) pw (first pw)))
        let (pw, score_base) = match entry {
            PrimaryWord::Bare(word) => (word, None),
            PrimaryWord::WithScoreBase(word, base) => (word, Some(base)),
        };

        // dict-grammar.lisp:357 (let ((k (get-kana pw))) …); nil → ""
        let pw_kana = get_kana(ctx, &pw).await?.unwrap_or_default();

        // dict-grammar.lisp:358-363 — patch-or-stem branch.
        let pw_kana_trimmed = match opts.patch {
            // (concatenate 'string (destem k (length (car patch-var))) (cdr patch-var))
            Some((car, cdr)) => {
                let car_len = car.chars().count();
                format!("{}{}", destem(&pw_kana, car_len, CharClass::Kana), cdr)
            }
            // (destem k stem)
            None => destem(&pw_kana, opts.stem, CharClass::Kana),
        };

        // dict-grammar.lisp:356 — (:text (concatenate 'string root suf-var))
        let text = format!("{}{}", root, suffix);
        // dict-grammar.lisp:357-365 — (:kana (concatenate 'string <pw_kana_trimmed> connector suf-var))
        let kana = format!("{}{}{}", pw_kana_trimmed, opts.connector, suffix);

        // dict-grammar.lisp:355 — (adjoin-word pw suf :text … :kana … :score-mod score :score-base score-base)
        let compound = adjoin_word(
            ctx,
            pw,
            KaniSimpleTextDispatchEnum::Kana(kf.clone()),
            Some(text),
            Some(kana),
            Some(opts.score.clone()),
            score_base,
        )
        .await?;
        out.push(compound);
    }
    Ok(out)
}

/// Port of `ichiran/dict:suffix-tai` (`dict-grammar.lisp:370`).
///
/// Handles the desiderative ～たい on a root other than い: looks up the
/// root as a conj-type 13 conjugation.
pub async fn suffix_tai(
    ctx: &KaniranContext,
    root: &str,
    suffix: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:371 — (unless (member root '("い") :test 'equal) …)
    let primary_words: Vec<PrimaryWord> = if root == "い" {
        Vec::new()
    } else {
        // dict-grammar.lisp:372 — (find-word-with-conj-type root 13)
        find_word_with_conj_type(ctx, root, &[13])
            .await?
            .into_iter()
            .map(PrimaryWord::from)
            .collect()
    };

    // dict-grammar.lisp:370 — (:connector "" :score 5), :stem 0 default.
    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(5),
        connector: "",
        patch: None,
    };
    def_simple_suffix_body(ctx, primary_words, root, suffix, kf, &opts).await
}

/// Port of `ichiran/dict:suffix-ren` (`dict-grammar.lisp:374`).
///
/// Generic ren'youkei (continuative-stem) suffix: looks up the root as a
/// conj-type 13 conjugation.
pub async fn suffix_ren(
    ctx: &KaniranContext,
    root: &str,
    suffix: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:376 — (find-word-with-conj-type root 13)
    let primary_words: Vec<PrimaryWord> = find_word_with_conj_type(ctx, root, &[13])
        .await?
        .into_iter()
        .map(PrimaryWord::from)
        .collect();

    // dict-grammar.lisp:374 — (:connector "" :score 5), :stem 0 default.
    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(5),
        connector: "",
        patch: None,
    };
    def_simple_suffix_body(ctx, primary_words, root, suffix, kf, &opts).await
}

/// Port of `ichiran/dict:suffix-ren-` (`dict-grammar.lisp:378`).
///
/// Score-0 ren'youkei suffix variant: looks up the root as a conj-type 13
/// conjugation (same body as `suffix-ren`, different score).
pub async fn suffix_ren_(
    ctx: &KaniranContext,
    root: &str,
    suffix: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:379 — (find-word-with-conj-type root 13)
    let primary_words: Vec<PrimaryWord> = find_word_with_conj_type(ctx, root, &[13])
        .await?
        .into_iter()
        .map(PrimaryWord::from)
        .collect();

    // dict-grammar.lisp:378 — (:connector "" :score 0), :stem 0 default.
    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(0),
        connector: "",
        patch: None,
    };
    def_simple_suffix_body(ctx, primary_words, root, suffix, kf, &opts).await
}

/// Port of `ichiran/dict:suffix-neg` (`dict-grammar.lisp:381`).
///
/// Handles ～なく (negative ～ない stem): looks up the root as a conjugation
/// of type 13 or negative-stem (52).
pub async fn suffix_neg(
    ctx: &KaniranContext,
    root: &str,
    suffix: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:382 — (find-word-with-conj-type root 13 +conj-negative-stem+)
    // dict-errata.lisp:1238 — (defconstant +conj-negative-stem+ 52)
    let primary_words: Vec<PrimaryWord> = find_word_with_conj_type(ctx, root, &[13, 52])
        .await?
        .into_iter()
        .map(PrimaryWord::from)
        .collect();

    // dict-grammar.lisp:381 — (:connector "" :score 5), :stem 0 default.
    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(5),
        connector: "",
        patch: None,
    };
    def_simple_suffix_body(ctx, primary_words, root, suffix, kf, &opts).await
}

/// Port of `ichiran/dict:te-check` (`dict-grammar.lisp:384`).
///
/// Returns -te-form (conjugation type 3) words for a root ending in て
/// or で, excluding bare "で".
pub async fn te_check(
    ctx: &KaniranContext,
    root: &str,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    // dict-grammar.lisp:385 (not (equal root "で"))
    if root == "で" {
        return Ok(Vec::new());
    }
    // dict-grammar.lisp:386 (find (char root (1- (length root))) "てで")
    // (char "" -1) signals SIMPLE-TYPE-ERROR upstream; mirror via panic
    // so a caller passing empty root sees an equivalent abort rather
    // than a silent empty result. Dead in practice — find-word-suffix
    // gates the callsite with (> offset 0) so root is never empty.
    let last = root
        .chars()
        .last()
        .expect("te-check: (char root (1- (length root))) on empty root signals upstream");
    if last != 'て' && last != 'で' {
        return Ok(Vec::new());
    }
    // dict-grammar.lisp:387 (find-word-with-conj-type root 3)
    find_word_with_conj_type(ctx, root, &[3]).await
}

/// Port of `ichiran/dict:suffix-te` (`dict-grammar.lisp:389`).
///
/// Handles a bare te-form auxiliary: looks up the root via `te-check`.
pub async fn suffix_te(
    ctx: &KaniranContext,
    root: &str,
    suffix: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:390 — (te-check root)
    let primary_words: Vec<PrimaryWord> = te_check(ctx, root)
        .await?
        .into_iter()
        .map(PrimaryWord::from)
        .collect();

    // dict-grammar.lisp:389 — (:connector "" :score 0), :stem 0 default.
    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(0),
        connector: "",
        patch: None,
    };
    def_simple_suffix_body(ctx, primary_words, root, suffix, kf, &opts).await
}

/// Port of `ichiran/dict:teiru-check` (`dict-grammar.lisp:392`).
///
/// Stricter [`te_check`]: excludes the literal "いて", then delegates.
pub async fn teiru_check(
    ctx: &KaniranContext,
    root: &str,
) -> Result<Vec<KaniWordDispatchEnum>, sqlx::Error> {
    // dict-grammar.lisp:393 (not (equal root "いて"))
    if root == "いて" {
        return Ok(Vec::new());
    }
    te_check(ctx, root).await
}

/// Port of `ichiran/dict:suffix-teiru` (`dict-grammar.lisp:395`).
///
/// Handles the ～ている progressive auxiliary: looks up the root via
/// `teiru-check`.
pub async fn suffix_teiru(
    ctx: &KaniranContext,
    root: &str,
    suffix: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:396 — (teiru-check root)
    let primary_words: Vec<PrimaryWord> = teiru_check(ctx, root)
        .await?
        .into_iter()
        .map(PrimaryWord::from)
        .collect();

    // dict-grammar.lisp:395 — (:connector "" :score 3), :stem 0 default.
    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(3),
        connector: "",
        patch: None,
    };
    def_simple_suffix_body(ctx, primary_words, root, suffix, kf, &opts).await
}

/// Port of `ichiran/dict:suffix-teiru+` (`dict-grammar.lisp:398`).
///
/// Score-6 variant of the ～ている auxiliary: looks up the root via
/// `teiru-check`.
pub async fn suffix_teiru_plus_(
    ctx: &KaniranContext,
    root: &str,
    suffix: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:399 — (teiru-check root)
    let primary_words: Vec<PrimaryWord> = teiru_check(ctx, root)
        .await?
        .into_iter()
        .map(PrimaryWord::from)
        .collect();

    // dict-grammar.lisp:398 — (:connector "" :score 6), :stem 0 default.
    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(6),
        connector: "",
        patch: None,
    };
    def_simple_suffix_body(ctx, primary_words, root, suffix, kf, &opts).await
}

/// Port of `ichiran/dict:suffix-te+space` (`dict-grammar.lisp:401`).
///
/// Handles a te-form followed by a space-joined auxiliary (くれる/もらう/
/// いただく): looks up the root via `te-check`.
pub async fn suffix_te_plus_space(
    ctx: &KaniranContext,
    root: &str,
    suffix: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:402 — (te-check root)
    let primary_words: Vec<PrimaryWord> = te_check(ctx, root)
        .await?
        .into_iter()
        .map(PrimaryWord::from)
        .collect();

    // dict-grammar.lisp:401 — (:connector " " :score 3), :stem 0 default.
    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(3),
        connector: " ",
        patch: None,
    };
    def_simple_suffix_body(ctx, primary_words, root, suffix, kf, &opts).await
}

/// Port of `ichiran/dict:suffix-kudasai` (`dict-grammar.lisp:404`).
///
/// Handles ～ください: looks up the root as a te-form via `te-check`.
pub async fn suffix_kudasai(
    ctx: &KaniranContext,
    root: &str,
    suffix: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:405 — (te-check root)
    let primary_words: Vec<PrimaryWord> = te_check(ctx, root)
        .await?
        .into_iter()
        .map(PrimaryWord::from)
        .collect();

    // dict-grammar.lisp:404 — (:connector " " :score (constantly 360)), :stem 0 default.
    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Constant(360),
        connector: " ",
        patch: None,
    };
    def_simple_suffix_body(ctx, primary_words, root, suffix, kf, &opts).await
}

/// Port of `ichiran/dict:suffix-te-ren` (`dict-grammar.lisp:407`).
///
/// Handles a te-form continuative auxiliary: for a root other than で,
/// looks up conj-type 3 if it ends in て/で, else conj-type 13 (unless
/// the root is い).
pub async fn suffix_te_ren(
    ctx: &KaniranContext,
    root: &str,
    suffix: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:408 — (not (equal root "で"))
    let primary_words: Vec<PrimaryWord> = if root == "で" {
        Vec::new()
    } else {
        // dict-grammar.lisp:409 — (find (char root (1- (length root))) "てで")
        // Upstream `(char "" -1)` signals SIMPLE-TYPE-ERROR; the
        // surrounding find-word-suffix gates with (> offset 0) so the
        // root is never empty.
        let last = root
            .chars()
            .last()
            .expect("suffix-te-ren: (char root (1- (length root))) on empty root signals upstream");
        if last == 'て' || last == 'で' {
            // dict-grammar.lisp:410 — (find-word-with-conj-type root 3)
            find_word_with_conj_type(ctx, root, &[3])
                .await?
                .into_iter()
                .map(PrimaryWord::from)
                .collect()
        } else if root != "い" {
            // dict-grammar.lisp:411-412 — (not (member root '("い") :test 'equal))
            //                              (find-word-with-conj-type root 13)
            find_word_with_conj_type(ctx, root, &[13])
                .await?
                .into_iter()
                .map(PrimaryWord::from)
                .collect()
        } else {
            // Upstream `cond` falls through to nil when both arms fail.
            Vec::new()
        }
    };

    // dict-grammar.lisp:407 — (:connector "" :score 4), :stem 0 default.
    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(4),
        connector: "",
        patch: None,
    };
    def_simple_suffix_body(ctx, primary_words, root, suffix, kf, &opts).await
}

/// Port of `ichiran/dict:suffix-teii` (`dict-grammar.lisp:414`).
///
/// Handles ～ていい/～てもいい: for a root ending in て/で, looks up
/// conj-type 3. Unlike `te-check` there is no で guard, so a bare で root
/// passes.
pub async fn suffix_teii(
    ctx: &KaniranContext,
    root: &str,
    suffix: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:415 — (find (char root (1- (length root))) "てで")
    // Upstream `(char "" -1)` signals SIMPLE-TYPE-ERROR; the
    // surrounding find-word-suffix gates with (> offset 0) so root is
    // never empty.
    let last = root
        .chars()
        .last()
        .expect("suffix-teii: (char root (1- (length root))) on empty root signals upstream");
    let primary_words: Vec<PrimaryWord> = if last == 'て' || last == 'で' {
        // dict-grammar.lisp:416 — (find-word-with-conj-type root 3)
        find_word_with_conj_type(ctx, root, &[3])
            .await?
            .into_iter()
            .map(PrimaryWord::from)
            .collect()
    } else {
        Vec::new()
    };

    // dict-grammar.lisp:414 — (:connector " " :score 1), :stem 0 default.
    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(1),
        connector: " ",
        patch: None,
    };
    def_simple_suffix_body(ctx, primary_words, root, suffix, kf, &opts).await
}

/// Port of `ichiran/dict:suffix-chau` (`dict-grammar.lisp:418`).
///
/// Handles the ～ちゃう/～じゃう contraction: maps the suffix's first
/// kana (じ→で, ち→て) and looks up the root plus that te-form.
pub async fn suffix_chau(
    ctx: &KaniranContext,
    root: &str,
    suf: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:418 — (def-simple-suffix suffix-chau :chau (:stem 1 …))
    // macro emits (let* ((*suffix-map-temp* nil)) …) for stem != 0.
    let ctx_rebound = ctx.with_suffix_map_temp(None);

    // dict-grammar.lisp:419-421 — (case (char suf 0) (#\じ "で") (#\ち "て"))
    let te = match suf.chars().next() {
        Some('じ') => Some("で"),
        Some('ち') => Some("て"),
        _ => None,
    };

    // dict-grammar.lisp:422-423 — (when te (find-word-with-conj-type (concatenate root te) 3))
    let primary_words: Vec<PrimaryWord> = match te {
        Some(te) => {
            let word = format!("{}{}", root, te);
            find_word_with_conj_type(&ctx_rebound, &word, &[3])
                .await?
                .into_iter()
                .map(PrimaryWord::from)
                .collect()
        }
        None => Vec::new(),
    };

    // dict-grammar.lisp:418 — (:stem 1 :score 5), :connector "" default.
    let opts = DefSimpleSuffixOpts {
        stem: 1,
        score: ScoreMod::Single(5),
        connector: "",
        patch: None,
    };
    def_simple_suffix_body(&ctx_rebound, primary_words, root, suf, kf, &opts).await
}

/// Port of `ichiran/dict:suffix-to` (`dict-grammar.lisp:425`).
///
/// Handles the ～とく/～どく contraction: maps the suffix's first kana
/// (と→て, ど→で) and looks up the root plus that te-form (conj-type 3).
pub async fn suffix_to(
    ctx: &KaniranContext,
    root: &str,
    suf: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:425 — (def-simple-suffix suffix-to :to (:stem 1 …))
    // macro emits (let* ((*suffix-map-temp* nil)) …) for stem != 0.
    let ctx_rebound = ctx.with_suffix_map_temp(None);

    // dict-grammar.lisp:426-428 — (case (char suf 0) (#\と "て") (#\ど "で"))
    let te = match suf.chars().next() {
        Some('と') => Some("て"),
        Some('ど') => Some("で"),
        _ => None,
    };

    // dict-grammar.lisp:429-430 — (when te (find-word-with-conj-type (concatenate root te) 3))
    let primary_words: Vec<PrimaryWord> = match te {
        Some(te) => {
            let word = format!("{}{}", root, te);
            find_word_with_conj_type(&ctx_rebound, &word, &[3])
                .await?
                .into_iter()
                .map(PrimaryWord::from)
                .collect()
        }
        None => Vec::new(),
    };

    // dict-grammar.lisp:425 — (:stem 1 :score 0), :connector "" default.
    let opts = DefSimpleSuffixOpts {
        stem: 1,
        score: ScoreMod::Single(0),
        connector: "",
        patch: None,
    };
    def_simple_suffix_body(&ctx_rebound, primary_words, root, suf, kf, &opts).await
}

/// Port of `ichiran/dict:suffix-suru` (`dict-grammar.lisp:432`).
///
/// Handles ～する on a suru-verb root: looks up the root as a "vs"
/// part-of-speech word.
pub async fn suffix_suru(
    ctx: &KaniranContext,
    root: &str,
    suffix: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:433 — (find-word-with-pos root "vs")
    let primary_words: Vec<PrimaryWord> = match find_word_with_pos(ctx, root, &["vs"]).await? {
        WordWithPosRows::Kana(rows) => rows
            .into_iter()
            .map(|r| PrimaryWord::from(KaniWordDispatchEnum::Kana(r)))
            .collect(),
        WordWithPosRows::Kanji(rows) => rows
            .into_iter()
            .map(|r| PrimaryWord::from(KaniWordDispatchEnum::Kanji(r)))
            .collect(),
    };

    // dict-grammar.lisp:432 — (:connector " " :score 5), :stem 0 default.
    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(5),
        connector: " ",
        patch: None,
    };
    def_simple_suffix_body(ctx, primary_words, root, suffix, kf, &opts).await
}

/// Port of `ichiran/dict:apply-patch` (`dict-grammar.lisp:435`).
///
/// Replaces the trailing `removed` chars of `root` with `replacement`
/// (patch = `(replacement, removed)`), used by suffix definers to
/// rewrite a candidate root before re-querying the dictionary. Length
/// is measured in characters, not bytes.
pub fn apply_patch(root: &str, patch: (&str, &str)) -> String {
    let (replacement, removed) = patch;
    let removed_chars = removed.chars().count();
    let root_chars = root.chars().count();
    // dict-grammar.lisp:436 (concatenate 'string (subseq root 0 (- ...)) (car patch))
    // Upstream errors via `subseq`'s end-bound check when removed > root.
    // The SBCL message text is implementation detail — only the
    // panic-on-overflow shape is load-bearing.
    let prefix_chars = root_chars
        .checked_sub(removed_chars)
        .expect("apply-patch: removed length exceeds root length");
    let byte_split = root
        .char_indices()
        .nth(prefix_chars)
        .map(|(b, _)| b)
        .unwrap_or(root.len());
    format!("{}{}", &root[..byte_split], replacement)
}

/// Port of `ichiran/dict:suffix-sou-base` (`dict-grammar.lisp:445`).
///
/// Shared body for the ～そう suffixes: for a root ending in なさ patches
/// it to ～なさ and finds negated conjugations, otherwise (unless the root
/// is one of な/よ/よさ/に/き) finds conj-types 13/51/50.
pub async fn suffix_sou_base_body(
    ctx: &KaniranContext,
    root: &str,
) -> Result<(Vec<PrimaryWord>, Option<(&'static str, &'static str)>), sqlx::Error> {
    // dict-grammar.lisp:446 (alexandria:ends-with-subseq "なさ" ,root)
    if root.ends_with("なさ") {
        // dict-grammar.lisp:447 (setf ,patch '("い" . "さ"))
        let patch = ("い", "さ");
        // dict-grammar.lisp:448 (let ((root (apply-patch ,root ,patch))
        //                              (*suffix-map-temp* nil)) …)
        let new_root = apply_patch(root, patch);
        let ctx_inner = ctx.with_suffix_map_temp(None);
        // dict-grammar.lisp:450-451 (find-word-with-conj-prop root
        //   (lambda (cdata) (conj-neg (conj-data-prop cdata))))
        let words = find_word_with_conj_prop(
            &ctx_inner,
            &new_root,
            |cd| cd.prop.as_ref().is_some_and(|p| p.neg != Some(false)),
            false,
        )
        .await?;
        let primary_words = words.into_iter().map(PrimaryWord::from).collect();
        Ok((primary_words, Some(patch)))
    } else if !matches!(root, "な" | "よ" | "よさ" | "に" | "き") {
        // dict-grammar.lisp:452-453 ((not (member ,root '("な" "よ" "よさ" "に" "き") :test 'equal))
        //   (find-word-with-conj-type ,root 13 +conj-adjective-stem+ +conj-adverbial+))
        let words = find_word_with_conj_type(ctx, root, &[13, 51, 50]).await?;
        let primary_words = words.into_iter().map(PrimaryWord::from).collect();
        Ok((primary_words, None))
    } else {
        Ok((Vec::new(), None))
    }
}

/// Port of `ichiran/dict:suffix-sou` (`dict-grammar.lisp:454`).
///
/// Handles ～そう (appearance), delegating to the shared
/// `suffix-sou-base` body; the score depends on the root
/// (から→40, い→0, 出来→100, else 70).
pub async fn suffix_sou(
    ctx: &KaniranContext,
    root: &str,
    suf: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:455-458 (constantly (cond …)) — resolved once over `root`.
    let score_val: i64 = if root == "から" {
        40
    } else if root == "い" {
        0
    } else if root == "出来" {
        100
    } else {
        70
    };

    // dict-grammar.lisp:461 (suffix-sou-base root patch)
    let (primary_words, patch) = suffix_sou_base_body(ctx, root).await?;

    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Constant(score_val),
        connector: "",
        patch,
    };
    def_simple_suffix_body(ctx, primary_words, root, suf, kf, &opts).await
}

/// Port of `ichiran/dict:suffix-sou+` (`dict-grammar.lisp:468`).
///
/// Score-1 variant of the ～そう suffix, delegating to the shared
/// `suffix-sou-base` body.
pub async fn suffix_sou_plus_(
    ctx: &KaniranContext,
    root: &str,
    suf: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:470 (suffix-sou-base root patch)
    let (primary_words, patch) = suffix_sou_base_body(ctx, root).await?;

    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(1),
        connector: "",
        patch,
    };
    def_simple_suffix_body(ctx, primary_words, root, suf, kf, &opts).await
}

/// Port of `ichiran/dict:suffix-rou` (`dict-grammar.lisp:461`).
///
/// Handles ～ろう (だろう volitional): looks up the root as a past-plain
/// (た-form, conj-type 2) conjugation.
pub async fn suffix_rou(
    ctx: &KaniranContext,
    root: &str,
    suffix: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:462 — (find-word-with-conj-type root 2)
    let primary_words: Vec<PrimaryWord> = find_word_with_conj_type(ctx, root, &[2])
        .await?
        .into_iter()
        .map(PrimaryWord::from)
        .collect();

    // dict-grammar.lisp:461 — (:connector "" :score 1), :stem 0 default.
    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(1),
        connector: "",
        patch: None,
    };
    def_simple_suffix_body(ctx, primary_words, root, suffix, kf, &opts).await
}

/// Port of `ichiran/dict:suffix-adv` (`dict-grammar.lisp:464`).
///
/// `:adv` suffix handler: finds words derived from `root` by an
/// adverbial conjugation (conj-type 50).
pub async fn suffix_adv(
    ctx: &KaniranContext,
    root: &str,
    suffix: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:465 — (find-word-with-conj-type root +conj-adverbial+)
    // dict-errata.lisp:1236 — (defconstant +conj-adverbial+ 50)
    let primary_words: Vec<PrimaryWord> = find_word_with_conj_type(ctx, root, &[50])
        .await?
        .into_iter()
        .map(PrimaryWord::from)
        .collect();

    // dict-grammar.lisp:464 — (:connector "" :score 1), :stem 0 default.
    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(1),
        connector: "",
        patch: None,
    };
    def_simple_suffix_body(ctx, primary_words, root, suffix, kf, &opts).await
}

/// Port of `ichiran/dict:suffix-sugiru` (`dict-grammar.lisp:475`).
///
/// Handles ～すぎる (excess): reconstitutes the adjective root (なさ/無さ
/// patched to ～さ, otherwise root+い) and looks it up either as a negated
/// conjugation or as an adj-i.
pub async fn suffix_sugiru(
    ctx: &KaniranContext,
    root: &str,
    suf: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:475 (:stem 1) — outer rebind to nil.
    let ctx_rebound = ctx.with_suffix_map_temp(None);

    // dict-grammar.lisp:476-479 (let ((root (cond …))) …)
    let (new_root_opt, patch_set): (Option<String>, Option<(&'static str, &'static str)>) =
        if root == "い" {
            (None, None)
        } else if root.ends_with("なさ") || root.ends_with("無さ") {
            let patch = ("い", "さ");
            (Some(apply_patch(root, patch)), Some(patch))
        } else {
            (Some(format!("{}い", root)), None)
        };

    // dict-grammar.lisp:480 (when root …)
    let primary_words: Vec<PrimaryWord> = match new_root_opt {
        None => Vec::new(),
        Some(new_root) => {
            if patch_set.is_some() && new_root.chars().count() > 2 {
                // dict-grammar.lisp:482-484 (find-word-with-conj-prop root
                //   (lambda (cdata) (conj-neg (conj-data-prop cdata))))
                find_word_with_conj_prop(
                    &ctx_rebound,
                    &new_root,
                    |cd| cd.prop.as_ref().is_some_and(|p| p.neg != Some(false)),
                    false,
                )
                .await?
                .into_iter()
                .map(PrimaryWord::from)
                .collect()
            } else {
                // dict-grammar.lisp:485 (t (find-word-with-pos root "adj-i"))
                match find_word_with_pos(&ctx_rebound, &new_root, &["adj-i"]).await? {
                    WordWithPosRows::Kana(rows) => rows
                        .into_iter()
                        .map(|r| PrimaryWord::from(KaniWordDispatchEnum::Kana(r)))
                        .collect(),
                    WordWithPosRows::Kanji(rows) => rows
                        .into_iter()
                        .map(|r| PrimaryWord::from(KaniWordDispatchEnum::Kanji(r)))
                        .collect(),
                }
            }
        }
    };

    let opts = DefSimpleSuffixOpts {
        stem: 1,
        score: ScoreMod::Single(5),
        connector: "",
        patch: patch_set,
    };
    def_simple_suffix_body(&ctx_rebound, primary_words, root, suf, kf, &opts).await
}

/// Port of `ichiran/dict:suffix-sa` (`dict-grammar.lisp:481`).
///
/// Handles the nominalizing ～さ: concatenates words found as the root's
/// adjective-stem (conj-type 51) conjugation with those found as adj-na.
pub async fn suffix_sa(
    ctx: &KaniranContext,
    root: &str,
    suf: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:483 — (find-word-with-conj-type root +conj-adjective-stem+)
    // dict-errata.lisp:1237 — (defconstant +conj-adjective-stem+ 51)
    let mut primary_words: Vec<PrimaryWord> = find_word_with_conj_type(ctx, root, &[51])
        .await?
        .into_iter()
        .map(PrimaryWord::from)
        .collect();

    // dict-grammar.lisp:484 — (find-word-with-pos root "adj-na")
    let adj_na = match find_word_with_pos(ctx, root, &["adj-na"]).await? {
        WordWithPosRows::Kana(rows) => rows
            .into_iter()
            .map(|r| PrimaryWord::from(KaniWordDispatchEnum::Kana(r)))
            .collect::<Vec<_>>(),
        WordWithPosRows::Kanji(rows) => rows
            .into_iter()
            .map(|r| PrimaryWord::from(KaniWordDispatchEnum::Kanji(r)))
            .collect::<Vec<_>>(),
    };
    // dict-grammar.lisp:482 — (nconc arm-A arm-B)
    primary_words.extend(adj_na);

    // dict-grammar.lisp:481 — (:connector "" :score 2), :stem 0 default.
    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(2),
        connector: "",
        patch: None,
    };
    def_simple_suffix_body(ctx, primary_words, root, suf, kf, &opts).await
}

/// Port of `ichiran/dict:suffix-iadj` (`dict-grammar.lisp:492`).
///
/// Handles i-adjective suffixes (げ/め): looks up the root as an
/// adjective-stem conjugation (conj-type 51).
pub async fn suffix_iadj(
    ctx: &KaniranContext,
    root: &str,
    suffix: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:493 — (find-word-with-conj-type root +conj-adjective-stem+)
    // dict-errata.lisp:1237 — (defconstant +conj-adjective-stem+ 51)
    let primary_words: Vec<PrimaryWord> = find_word_with_conj_type(ctx, root, &[51])
        .await?
        .into_iter()
        .map(PrimaryWord::from)
        .collect();

    // dict-grammar.lisp:492 — (:connector "" :score 1), :stem 0 default.
    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(1),
        connector: "",
        patch: None,
    };
    def_simple_suffix_body(ctx, primary_words, root, suffix, kf, &opts).await
}

/// Port of `ichiran/dict:suffix-garu` (`dict-grammar.lisp:504`).
///
/// Handles ～がる on adjectives: for a root other than な/い/よ, looks up
/// an adjective-stem conjugation, or for a root ending in そ patches it to
/// ～そう and retries via the :sou suffix.
pub async fn suffix_garu(
    ctx: &KaniranContext,
    root: &str,
    suf: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:505 (unless (member root '("な" "い" "よ") …))
    let (primary_words, patch_set): (Vec<PrimaryWord>, Option<(&'static str, &'static str)>) =
        if matches!(root, "な" | "い" | "よ") {
            (Vec::new(), None)
        } else {
            // dict-grammar.lisp:506 (or (find-word-with-conj-type root +conj-adjective-stem+) …)
            // dict-errata.lisp:1237 — (defconstant +conj-adjective-stem+ 51)
            let arm_a = find_word_with_conj_type(ctx, root, &[51]).await?;
            if !arm_a.is_empty() {
                (arm_a.into_iter().map(PrimaryWord::from).collect(), None)
            } else if root.ends_with("そ") {
                // dict-grammar.lisp:507-511 (when (ends-with "そ" root)
                //   (setf patch '("う" . "")) (let ((root (apply-patch root patch))
                //                                   (*suffix-map-temp* nil))
                //     (find-word-with-suffix root :sou)))
                let patch = ("う", "");
                let new_root = apply_patch(root, patch);
                let ctx_inner = ctx.with_suffix_map_temp(None);
                let words = find_word_with_suffix(&ctx_inner, &new_root, &["sou"]).await?;
                (
                    words.into_iter().map(PrimaryWord::from).collect(),
                    Some(patch),
                )
            } else {
                (Vec::new(), None)
            }
        };

    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(0),
        connector: "",
        patch: patch_set,
    };
    def_simple_suffix_body(ctx, primary_words, root, suf, kf, &opts).await
}

/// Port of `ichiran/dict:suffix-ra` (`dict-grammar.lisp:504`).
///
/// Handles the pluralizing ～ら on a root not already ending in ら:
/// looks it up as a pronoun, falling back to the seq-1580640 entry.
pub async fn suffix_ra(
    ctx: &KaniranContext,
    root: &str,
    suffix: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:505 — (unless (alexandria:ends-with-subseq "ら" root) …)
    let primary_words: Vec<PrimaryWord> = if root.ends_with('ら') {
        Vec::new()
    } else {
        // dict-grammar.lisp:506 — (or-as-hiragana 'find-word-with-pos root "pn")
        let finder: OrAsHiraganaFinder<'_> = Arc::new(|word: String| {
            Box::pin(async move {
                let rows = find_word_with_pos(ctx, &word, &["pn"]).await?;
                Ok(match rows {
                    WordWithPosRows::Kana(v) => FindWordRows::Kana(v),
                    WordWithPosRows::Kanji(v) => FindWordRows::Kanji(v),
                })
            })
        });
        let from_pn: Option<Vec<KaniWordDispatchEnum>> = or_as_hiragana(ctx, root, finder)
            .await?
            .map(|rows| match rows {
                OrAsHiraganaRows::Direct(FindWordRows::Kana(v)) => {
                    v.into_iter().map(KaniWordDispatchEnum::Kana).collect()
                }
                OrAsHiraganaRows::Direct(FindWordRows::Kanji(v)) => {
                    v.into_iter().map(KaniWordDispatchEnum::Kanji).collect()
                }
                OrAsHiraganaRows::AsHiragana(v) => {
                    v.into_iter().map(KaniWordDispatchEnum::Proxy).collect()
                }
            });

        // dict-grammar.lisp:507 — (find-word-seq root 1580640).
        // `or` falls through to this when `or-as-hiragana` returned nil.
        let words: Vec<KaniWordDispatchEnum> = match from_pn {
            Some(words) => words,
            None => match find_word_seq(ctx, root, &[1580640]).await? {
                WordSeqRows::Kana(rows) => {
                    rows.into_iter().map(KaniWordDispatchEnum::Kana).collect()
                }
                WordSeqRows::Kanji(rows) => {
                    rows.into_iter().map(KaniWordDispatchEnum::Kanji).collect()
                }
            },
        };
        words.into_iter().map(PrimaryWord::from).collect()
    };

    // dict-grammar.lisp:504 — (:connector "" :score 1), :stem 0 default.
    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(1),
        connector: "",
        patch: None,
    };
    def_simple_suffix_body(ctx, primary_words, root, suffix, kf, &opts).await
}

/// Port of `ichiran/dict:suffix-rashii` (`dict-grammar.lisp:520`).
///
/// Handles ～らしい: pairs words found as the root's past-plain (conj 2)
/// with those found as root+ら conj-type 11.
pub async fn suffix_rashii(
    ctx: &KaniranContext,
    root: &str,
    suf: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:522 (find-word-with-conj-type root 2)
    let group_a = find_word_with_conj_type(ctx, root, &[2]).await?;
    // dict-grammar.lisp:523 (find-word-with-conj-type (concatenate root "ら") 11)
    let root_ra = format!("{}ら", root);
    let group_b = find_word_with_conj_type(ctx, &root_ra, &[11]).await?;

    // dict-grammar.lisp:521 (pair-words-by-conj group_a group_b)
    let buckets = pair_words_by_conj(ctx, &[group_a, group_b]).await?;

    // dict-grammar.lisp:351-354 (when (listp pw) (setf score-base (second pw) pw (first pw)))
    let primary_words: Vec<PrimaryWord> = buckets
        .into_iter()
        .map(|bucket| {
            let mut iter = bucket.into_iter();
            let first = iter
                .next()
                .expect("pair-words-by-conj bucket missing slot 0");
            let second = iter
                .next()
                .expect("pair-words-by-conj bucket missing slot 1");
            let pw = first.expect(
                "suffix-rashii: pair-words-by-conj bucket has nil at slot 0; \
                 upstream (adjoin-word nil w2) raises no-applicable-method",
            );
            match second {
                Some(score_base) => PrimaryWord::WithScoreBase(pw, score_base),
                None => PrimaryWord::Bare(pw),
            }
        })
        .collect();

    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(3),
        connector: "",
        patch: None,
    };
    def_simple_suffix_body(ctx, primary_words, root, suf, kf, &opts).await
}

/// Port of `ichiran/dict:suffix-desu` (`dict-grammar.lisp:525`).
///
/// Handles ～です after a negative: when the root ends in ない or なかった,
/// looks up a word whose conjugation is negated.
pub async fn suffix_desu(
    ctx: &KaniranContext,
    root: &str,
    suf: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:526-527 (or (ends-with "ない" root) (ends-with "なかった" root))
    let primary_words: Vec<PrimaryWord> = if root.ends_with("ない") || root.ends_with("なかった")
    {
        // dict-grammar.lisp:528-529 (find-word-with-conj-prop root
        //   (lambda (cdata) (conj-neg (conj-data-prop cdata))))
        find_word_with_conj_prop(
            ctx,
            root,
            |cd| cd.prop.as_ref().is_some_and(|p| p.neg != Some(false)),
            false,
        )
        .await?
        .into_iter()
        .map(PrimaryWord::from)
        .collect()
    } else {
        Vec::new()
    };

    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Constant(200),
        connector: " ",
        patch: None,
    };
    def_simple_suffix_body(ctx, primary_words, root, suf, kf, &opts).await
}

/// Port of `ichiran/dict:suffix-desho` (`dict-grammar.lisp:541`).
///
/// Handles ～でしょ after a negative: when the root ends in ない, looks
/// up a word whose conjugation is negated.
pub async fn suffix_desho(
    ctx: &KaniranContext,
    root: &str,
    suf: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:542 (ends-with "ない" root)
    let primary_words: Vec<PrimaryWord> = if root.ends_with("ない") {
        // dict-grammar.lisp:543-544 (find-word-with-conj-prop root
        //   (lambda (cdata) (conj-neg (conj-data-prop cdata))))
        find_word_with_conj_prop(
            ctx,
            root,
            |cd| cd.prop.as_ref().is_some_and(|p| p.neg != Some(false)),
            false,
        )
        .await?
        .into_iter()
        .map(PrimaryWord::from)
        .collect()
    } else {
        Vec::new()
    };

    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Constant(300),
        connector: " ",
        patch: None,
    };
    def_simple_suffix_body(ctx, primary_words, root, suf, kf, &opts).await
}

/// Port of `ichiran/dict:suffix-tosuru` (`dict-grammar.lisp:537`).
///
/// Handles ～とする: looks up the root as a volitional (conj-type 9, e.g.
/// 食べよう, 飲もう, 行こう) conjugation.
pub async fn suffix_tosuru(
    ctx: &KaniranContext,
    root: &str,
    suffix: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:538 — (find-word-with-conj-type root 9)
    let primary_words: Vec<PrimaryWord> = find_word_with_conj_type(ctx, root, &[9])
        .await?
        .into_iter()
        .map(PrimaryWord::from)
        .collect();

    // dict-grammar.lisp:537 — (:connector " " :score 3), :stem 0 default.
    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(3),
        connector: " ",
        patch: None,
    };
    def_simple_suffix_body(ctx, primary_words, root, suffix, kf, &opts).await
}

/// Port of `ichiran/dict:suffix-kurai` (`dict-grammar.lisp:540`).
///
/// Handles ～くらい/～ぐらい: looks up the root as a past-plain (た-form,
/// conj-type 2) conjugation.
pub async fn suffix_kurai(
    ctx: &KaniranContext,
    root: &str,
    suffix: &str,
    kf: &KanaText,
) -> Result<Vec<CompoundText>, sqlx::Error> {
    // dict-grammar.lisp:541 — (find-word-with-conj-type root 2)
    let primary_words: Vec<PrimaryWord> = find_word_with_conj_type(ctx, root, &[2])
        .await?
        .into_iter()
        .map(PrimaryWord::from)
        .collect();

    // dict-grammar.lisp:540 — (:connector " " :score 3), :stem 0 default.
    let opts = DefSimpleSuffixOpts {
        stem: 0,
        score: ScoreMod::Single(3),
        connector: " ",
        patch: None,
    };
    def_simple_suffix_body(ctx, primary_words, root, suffix, kf, &opts).await
}

#[cfg(test)]
mod tests;
