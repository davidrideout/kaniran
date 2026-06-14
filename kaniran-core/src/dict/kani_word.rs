//! Top-level dispatcher enums for the `ichiran/dict` word polymorphism.
//!
//! Sidecar (no Lisp FQN). Lisp `word` is the ad-hoc union of the
//! reading and tokenization types that segmentation and scoring
//! generic functions (`get-kana`, `text`, `seq`, `common`, `ord`,
//! `word-type`, `word-conj-data`, ...) dispatch over.
//! [`KaniWordDispatchEnum`] names that union;
//! [`KaniSimpleTextDispatchEnum`] names the `simple-text` sub-family.
//! `entry` is not a member: every upstream entry callsite is locally
//! Entry-typed rather than polymorphic.

use crate::dict::text_classes::CompoundText;
use crate::dict::counters::classes::Counter;
use crate::dict::dao::KanaText;
use crate::dict::dao::KanjiText;
use crate::dict::text_classes::ProxyText;
use crate::dict::accessors::WordType;

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
    /// the wider [`super::counters::methods::seq`] free fn to the simple-text
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
    /// [`super::split::hint::get_hint`], which dispatches on the wider
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
    /// internally; the top-level [`super::accessors::get_kana`]
    /// dispatcher just delegates here for the simple-text arms.
    pub fn get_kana(
        &self,
        ctx: &crate::conn::kani_context::KaniranContext,
    ) -> Result<Option<String>, crate::conn::KaniDbError> {
        // dict.lisp:80-84 (defmethod get-kana :around ((obj simple-text)))
        // (unless (or *disable-hints* (hintedp obj))
        //    (let ((*disable-hints* t)) (get-hint obj)))
        if !ctx.disable_hints && !self.hintedp() {
            let wrapped = self.to_word();
            // dict.lisp:82 (let ((*disable-hints* t)) (get-hint obj))
            let ctx2 = ctx.with_disable_hints(true);
            if let Some(hint_result) =
                super::split::hint::get_hint(&ctx2, &wrapped)?
            {
                return Ok(Some(hint_result));
            }
        }
        // dict.lisp:84 (call-next-method) — primary methods
        self.primary_get_kana(ctx)
    }

    /// The "call-next-method" body of [`Self::get_kana`] — the
    /// per-subclass primary method bodies for kanji-text /
    /// kana-text / proxy-text.
    fn primary_get_kana(
        &self,
        ctx: &crate::conn::kani_context::KaniranContext,
    ) -> Result<Option<String>, crate::conn::KaniDbError> {
        match self {
            // dict.lisp:111-115 (defmethod get-kana ((obj kanji-text)))
            // (let ((bk (best-kana-conj obj))) (if (eql bk :null) (get-kanji-kana-old obj) bk))
            // best_kana_conj returning :null falls through to
            // get_kanji_kana_old, which may itself return None
            // (no sibling kana_text row) — upstream `(text nil)`
            // would raise no-applicable-method; Rust surfaces
            // it as Ok(None).
            Self::Kanji(k) => {
                match super::readings::best_kana_conj(ctx, k)? {
                    Some(s) => Ok(Some(s)),
                    None => super::readings::get_kanji_kana_old(ctx, k),
                }
            }
            // dict.lisp:150-151 (defmethod get-kana ((obj kana-text))) — (text obj)
            Self::Kana(k) => Ok(Some(k.text.to_string())),
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
