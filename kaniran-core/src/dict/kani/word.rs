//! Kaniran sidecars — Rust-only types with no Lisp counterpart.
//!
//! Folds six per-symbol sidecars into one module: the word-polymorphism
//! dispatcher enums plus five small token/tag enums shared across the
//! `ichiran/dict` port (hint kind, suffix kind, conj-form token,
//! split-part token, match-part).
//!
//! - [`KaniWordDispatchEnum`] / [`KaniSimpleTextDispatchEnum`] —
//!   top-level dispatchers for the `ichiran/dict` word polymorphism.
//!   Lisp `word` is the ad-hoc union of the reading and tokenization
//!   types over which segmentation / scoring generic functions
//!   (`get-kana`, `text`, `seq`, `common`, `ord`, `word-type`,
//!   `word-conj-data`, ...) dispatch. `entry` is **not** a member:
//!   upstream gfs define methods on `entry`, but every upstream
//!   callsite passing an entry is locally Entry-typed
//!   (`entry-digest` at `dict.lisp:67` is canonical) — none route
//!   through polymorphic dispatch.
//!
//! - [`KaniMatchPart`] — closed two-variant enum for the heterogeneous
//!   match list consumed by [`crate::dict::split::hint::translate_hint_position`] /
//!   [`crate::dict::split::hint::translate_hints`]. CL feeds these as the raw output
//!   of `match-diff` (atoms or `(s1 s2)` pairs — `characters.lisp:326`)
//!   and `match-readings` (bare strings or `(kanji-char reading-text)`
//!   pairs — `kanji.lisp:296`); callers only ever read length, so the
//!   variants carry pre-computed character counts.
//!
//! - [`KaniHintKind`] — closed enum for the `(:space :mod)` keyword
//!   tags of the kana-hint system; used inline by the upstream as
//!   `*hint-char-map*` keys (`dict-split.lisp:816`) and as the first
//!   element of each emitted hint pair.
//!
//! - [`SuffixKind`] — closed enum for the counter suffix tags
//!   (`:kan`, `:kango`, `:chuu`); REPL-verified exhaustive against
//!   `:accepts` values in the populated `*counter-cache*`.
//!
//! - [`FormToken`] / [`ConjForm`] — typed match cells for the
//!   conjugation-form data registered in `*skip-conj-forms*` /
//!   `*weak-conj-forms*` and consumed by `test-conj-prop`. The CL form
//!   uses untyped lists with heterogeneous `pos` / `conj-type` / `:any`
//!   / `T` / `NIL` / `:null` cells; the Rust enum pins the closed
//!   vocabulary so the data tables can be `pub static` and the matcher
//!   dispatches without runtime type checks.
//!
//! - [`SplitPart`] — heterogeneous parts list element pushed by
//!   `def-simple-split` (`dict-split.lisp:13`). `calc-score`
//!   (`dict.lisp:940-974`) discriminates by `(member :score split)` /
//!   `(member :pscore split)` before iterating the word elements.
//!   Failed lookups are pushed as Lisp `nil` and modeled as
//!   `Option<SplitPart>` at the call site.

use crate::dict::compound_text_class::CompoundText;
use crate::dict::counters::classes::Counter;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kanji_text_dao::KanjiText;
use crate::dict::proxy_text_class::ProxyText;
use crate::dict::word_type::WordType;

// =========================================================================
// KaniMatchPart
// =========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KaniMatchPart {
    Atom(usize),
    Pair(usize, usize),
}

// =========================================================================
// KaniHintKind
// =========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KaniHintKind {
    Space,
    Mod,
}

// =========================================================================
// KaniSuffixKind
// =========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SuffixKind {
    Kan,
    Kango,
    Chuu,
}

// =========================================================================
// FormToken / ConjForm
// =========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormToken {
    Any,
    Int(i32),
    Str(&'static str),
    Bool(bool),
    DbNull,
}

#[derive(Debug, Clone, Copy)]
pub enum ConjForm {
    /// `(conj-type neg fml)` — matched against `prop.conj_type`,
    /// `prop.neg`, `prop.fml`.
    Triple(FormToken, FormToken, FormToken),
    /// `(pos conj-type neg fml)` — matched against `prop.pos`,
    /// `prop.conj_type`, `prop.neg`, `prop.fml`.
    Quadruple(FormToken, FormToken, FormToken, FormToken),
}

// =========================================================================
// KaniSimpleTextDispatchEnum / KaniWordDispatchEnum
// =========================================================================

#[derive(Debug, Clone)]
pub enum KaniSimpleTextDispatchEnum {
    Kanji(KanjiText),
    Kana(KanaText),
    Proxy(ProxyText),
}

impl KaniSimpleTextDispatchEnum {
    /// Family-level dispatcher for `seq` (cross-family gf). Mirrors
    /// the `Counter::get_kana` pattern from CONVENTIONS §4.7
    /// ("A sibling enum in the base file dispatches"): this narrows
    /// the wider [`crate::dict::counters::dispatchers::seq`] free fn to the simple-text
    /// subset so split / synergy callers can borrow a
    /// `&KaniSimpleTextDispatchEnum` without round-tripping through
    /// [`KaniWordDispatchEnum`].
    pub fn seq(&self) -> i32 {
        let mut current = self;
        loop {
            match current {
                Self::Kanji(k) => return k.seq,
                Self::Kana(k) => return k.seq,
                Self::Proxy(p) => current = &p.source,
            }
        }
    }

    /// Family-level dispatcher for `true-text`. See [`Self::seq`].
    pub fn true_text(&self) -> &str {
        let mut current = self;
        loop {
            match current {
                Self::Kanji(k) => return &k.text,
                Self::Kana(k) => return &k.text,
                Self::Proxy(p) => current = &p.source,
            }
        }
    }

    /// Family-level dispatcher for `word-type`. See [`Self::seq`].
    pub fn word_type(&self) -> WordType {
        let mut current = self;
        loop {
            match current {
                Self::Kanji(_) => return WordType::Kanji,
                Self::Kana(_) => return WordType::Kana,
                Self::Proxy(p) => current = &p.source,
            }
        }
    }

    /// `simple-text.hintedp` slot (`dict.lisp:69-76`) — the
    /// re-entrance flag the `:around get-kana` method checks
    /// (`dict.lisp:80-84`). Proxy delegates to the wrapped source.
    pub fn hintedp(&self) -> bool {
        match self {
            Self::Kanji(k) => k.state.hintedp,
            Self::Kana(k) => k.state.hintedp,
            Self::Proxy(p) => p.state.hintedp,
        }
    }

    /// Clone-wrap into [`KaniWordDispatchEnum`] for callers that
    /// need the wider type. Used by [`Self::get_kana`] to invoke
    /// [`crate::dict::split::hint::get_hint`], which dispatches on the
    /// wider enum.
    pub fn to_word(&self) -> KaniWordDispatchEnum {
        match self {
            Self::Kanji(k) => KaniWordDispatchEnum::Kanji(k.clone()),
            Self::Kana(k) => KaniWordDispatchEnum::Kana(k.clone()),
            Self::Proxy(p) => KaniWordDispatchEnum::Proxy(p.clone()),
        }
    }

    /// Family-level `get-kana` for the simple-text family,
    /// covering the `:around` hint dispatch (`dict.lisp:80-84`)
    /// followed by the family's primary methods
    /// (`dict.lisp:111-115` for kanji-text,
    /// `dict.lisp:150-151` for kana-text,
    /// `dict.lisp:552` slot reader for proxy-text). Per
    /// CONVENTIONS §4.7, each family handles its own `:around`
    /// internally; the top-level [`crate::dict::get_kana::get_kana`]
    /// dispatcher just delegates here for the simple-text arms.
    pub async fn get_kana(
        &self,
        ctx: &crate::conn::kani_context::KaniranContext,
    ) -> Result<Option<String>, sqlx::Error> {
        // dict.lisp:80-84 (defmethod get-kana :around ((obj simple-text)))
        // (unless (or *disable-hints* (hintedp obj))
        //    (let ((*disable-hints* t)) (get-hint obj)))
        if !ctx.disable_hints && !self.hintedp() {
            let wrapped = self.to_word();
            // dict.lisp:82 (let ((*disable-hints* t)) (get-hint obj))
            let ctx2 = ctx.with_disable_hints(true);
            if let Some(hint_result) =
                crate::dict::split::hint::get_hint(&ctx2, &wrapped).await?
            {
                return Ok(Some(hint_result));
            }
        }
        // dict.lisp:84 (call-next-method) — primary methods
        self.primary_get_kana(ctx).await
    }

    /// The "call-next-method" body of [`Self::get_kana`] — the
    /// per-subclass primary method bodies for kanji-text /
    /// kana-text / proxy-text.
    async fn primary_get_kana(
        &self,
        ctx: &crate::conn::kani_context::KaniranContext,
    ) -> Result<Option<String>, sqlx::Error> {
        match self {
            // dict.lisp:111-115 (defmethod get-kana ((obj kanji-text)))
            // (let ((bk (best-kana-conj obj))) (if (eql bk :null) (get-kanji-kana-old obj) bk))
            // best_kana_conj returning :null falls through to
            // get_kanji_kana_old, which may itself return None
            // (no sibling kana_text row) — upstream `(text nil)`
            // would raise no-applicable-method; Rust surfaces
            // it as Ok(None).
            Self::Kanji(k) => {
                match crate::dict::best_kana_conj::best_kana_conj(ctx, k).await? {
                    Some(s) => Ok(Some(s)),
                    None => crate::dict::get_kanji_kana_old::get_kanji_kana_old(ctx, k).await,
                }
            }
            // dict.lisp:150-151 (defmethod get-kana ((obj kana-text))) — (text obj)
            Self::Kana(k) => Ok(Some(k.text.clone())),
            // dict.lisp:552 (kana :reader get-kana :initarg :kana) on proxy-text
            Self::Proxy(p) => Ok(Some(p.kana.clone())),
        }
    }
}

#[derive(Debug, Clone)]
pub enum KaniWordDispatchEnum {
    Kanji(KanjiText),
    Kana(KanaText),
    Proxy(ProxyText),
    Compound(CompoundText),
    Counter(Counter),
}

// =========================================================================
// SplitPart
// =========================================================================

#[derive(Debug, Clone)]
pub enum SplitPart {
    Word(KaniWordDispatchEnum),
    Score,
    PScore,
}
