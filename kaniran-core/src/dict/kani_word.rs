//! Top-level dispatcher enums for the `ichiran/dict` word polymorphism.
//!
//! Sidecar (no Lisp FQN). Lisp `word` is the ad-hoc union of the
//! reading and tokenization types that segmentation and scoring
//! generic functions (`get-kana`, `text`, `seq`, `common`, `ord`,
//! `word-type`, `word-conj-data`, ...) dispatch over. The
//! [`KaniWordDispatchEnum`] names that union;
//! [`KaniSimpleTextDispatchEnum`] names the `simple-text` sub-family
//! used by [`super::proxy_text_class::ProxyText::source`].
//!
//! Counter is wrapped through its existing family enum
//! [`super::counter_text_class::Counter`], which already dispatches
//! across the 11 counter-text subclasses.
//!
//! `entry` is **not** a member of this enum. Upstream's gfs
//! (`common`, `get-kana`, `get-text`, `get-kanji`) define methods
//! specialized on `entry`, but every upstream callsite that passes
//! an entry is locally Entry-typed (`entry-digest` at
//! `dict.lisp:67` is the canonical one) — none route through
//! polymorphic dispatch. The Rust port mirrors that: locally-typed
//! callsites invoke `Entry::get_text(ctx)` /
//! `Entry::get_kana(ctx)` directly; the dispatcher enums never
//! carry an entry instance.

use crate::dict::compound_text_class::CompoundText;
use crate::dict::counter_text_class::Counter;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kanji_text_dao::KanjiText;
use crate::dict::proxy_text_class::ProxyText;
use crate::dict::word_type::WordType;

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
    /// the wider [`super::seq::seq`] free fn to the simple-text
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
    /// [`super::get_hint::get_hint`], which dispatches on the wider
    /// enum.
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
    /// internally; the top-level [`super::get_kana::get_kana`]
    /// dispatcher just delegates here for the simple-text arms.
    pub async fn get_kana(
        &self,
        ctx: &crate::conn::kani_context::KaniranContext,
        disable_hints: bool,
    ) -> Result<Option<String>, sqlx::Error> {
        // dict.lisp:80-84 (defmethod get-kana :around ((obj simple-text)))
        // (unless (or *disable-hints* (hintedp obj))
        //    (let ((*disable-hints* t)) (get-hint obj)))
        if !disable_hints && !self.hintedp() {
            let wrapped = self.to_word();
            if let Some(hint_result) =
                super::get_hint::get_hint(ctx, &wrapped, true).await?
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
                match super::best_kana_conj::best_kana_conj(ctx, k).await? {
                    Some(s) => Ok(Some(s)),
                    None => super::get_kanji_kana_old::get_kanji_kana_old(ctx, k).await,
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
