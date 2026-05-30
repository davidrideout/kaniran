//! Port of `ichiran:process-iteration-characters` (`romanize.lisp:9-15`).
//!
//! Expands iteration markers in a sequence of character-class items.
//! `Iter` (ゝヽ) repeats the previous item; `IterV` (ゞヾ) emits the
//! voiced form of the previous item. The "previous" slot only updates
//! on a non-iteration item, so a run of iteration markers all expand
//! to the same source. An iteration marker at the start of the list
//! (no previous item) drops out silently — matching the upstream's
//! conditional-collect behavior.
//!
//! Input items are [`CcItem`]s — the per-character lookups produced by
//! `get-character-classes` (a [`KanaClass`] when the glyph is recognized
//! kana, else the plain [`char`]).
//!
//! Voicing for `IterV` only fires when the previous item is a
//! [`KanaClass`]. The upstream `voice-char` falls through unchanged on
//! characters (its hash lookup misses and the default-as-self idiom
//! returns the input), so a [`CcItem::Char`] previous is emitted
//! as-is, preserving Lisp behavior.

use super::kani_cc_item::CcItem;
use crate::characters::kana_class::KanaClass;
use crate::characters::voicing::voice_char;

pub fn process_iteration_characters(cc_list: &[CcItem]) -> Vec<CcItem> {
    let mut out = Vec::with_capacity(cc_list.len());
    let mut prev: Option<CcItem> = None;
    for &cc in cc_list {
        match cc {
            CcItem::Class(KanaClass::Iter) => {
                if let Some(p) = prev {
                    out.push(p);
                }
            }
            CcItem::Class(KanaClass::IterV) => {
                if let Some(p) = prev {
                    out.push(match p {
                        CcItem::Class(k) => CcItem::Class(voice_char(k)),
                        CcItem::Char(_) => p,
                    });
                }
            }
            other => {
                out.push(other);
                prev = Some(other);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(c: KanaClass) -> CcItem {
        CcItem::Class(c)
    }

    #[test]
    fn iter_at_start_emits_nothing() {
        // No prev yet — both markers drop out, no panic.
        let result = process_iteration_characters(&[k(KanaClass::Iter), k(KanaClass::IterV)]);
        assert_eq!(result, vec![]);
    }

    #[test]
    fn iter_repeats_previous_item() {
        // さゝ — Sa, then Iter expands to a second Sa.
        let result = process_iteration_characters(&[k(KanaClass::Sa), k(KanaClass::Iter)]);
        assert_eq!(result, vec![k(KanaClass::Sa), k(KanaClass::Sa)]);
    }

    #[test]
    fn iter_v_voices_previous_kana() {
        // さゞ — Sa, then IterV expands to the voiced form Za.
        let result = process_iteration_characters(&[k(KanaClass::Sa), k(KanaClass::IterV)]);
        assert_eq!(result, vec![k(KanaClass::Sa), k(KanaClass::Za)]);
    }

    #[test]
    fn run_of_iters_all_reference_same_source() {
        // prev only updates on a non-iteration item, so さゝゝゝ all
        // expand from the original Sa, not from each emitted copy.
        let result = process_iteration_characters(&[
            k(KanaClass::Sa),
            k(KanaClass::Iter),
            k(KanaClass::Iter),
            k(KanaClass::Iter),
        ]);
        assert_eq!(
            result,
            vec![
                k(KanaClass::Sa),
                k(KanaClass::Sa),
                k(KanaClass::Sa),
                k(KanaClass::Sa),
            ]
        );
    }

    #[test]
    fn iter_v_after_unvoiceable_kana_falls_through() {
        // A → IterV: A has no voiced counterpart in *dakuten-hash*,
        // so voice_char returns A unchanged.
        let result = process_iteration_characters(&[k(KanaClass::A), k(KanaClass::IterV)]);
        assert_eq!(result, vec![k(KanaClass::A), k(KanaClass::A)]);
    }

    #[test]
    fn char_prev_passes_through_iter_v_unchanged() {
        // Upstream's voice-char hash misses on a raw char and returns
        // it as-is; Rust mirrors that branch by emitting the char.
        let result = process_iteration_characters(&[CcItem::Char('!'), k(KanaClass::IterV)]);
        assert_eq!(result, vec![CcItem::Char('!'), CcItem::Char('!')]);
    }
}
