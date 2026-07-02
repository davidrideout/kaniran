//! Rust-only sidecar within the `ichiran/characters` port module.
//! Has no Lisp counterpart.
//!
//! Span-tracking variant of [`crate::characters::kana::normalize`] for the
//! v2 serializer's verbatim-text contract: v2 tokens quote the original
//! input, but segmentation offsets live in the normalized stream, so the
//! serializer needs to map any normalized char range back to the input
//! chars it came from. The rewrites that shift char counts (`、` → `", "`,
//! `か゛` → `が`) happen in the ngram pass; the per-char substitution phase
//! is 1:1, so its indices need no map of their own.

use std::sync::LazyLock;

use crate::characters::constants::PUNCTUATION_MARKS;
use crate::characters::helpers::dakuten_join;
use crate::characters::kana::{to_normal_char, NormalizationContext};
use crate::characters::kani_ngram_scanner::KaniNgramScanner;

/// The source-char range behind each normalized char, from
/// [`KaniNgramScanner::simplify_spanned`]. `source_range` turns a normalized
/// char range into the input char range it was rewritten from.
///
/// ```text
/// kani_normalize_spanned("あ、い", Default)
///   → ("あ, い", spans [(0,1), (1,2), (1,2), (2,3)])
/// spans.source_range(1, 3) == (1, 2)   // ", " came from the 、
/// ```
#[derive(Debug, Clone)]
pub struct KaniNormalizedSpans {
    per_char: Vec<(usize, usize)>,
    source_char_len: usize,
}

impl KaniNormalizedSpans {
    /// Chars of the normalized string.
    pub fn normalized_len(&self) -> usize {
        self.per_char.len()
    }

    /// Map the normalized char range `[start, end)` to the input char range
    /// it came from. An empty range maps to an empty range at the
    /// corresponding position.
    pub fn source_range(&self, start: usize, end: usize) -> (usize, usize) {
        debug_assert!(start <= end && end <= self.per_char.len());
        if start >= end {
            let at = self
                .per_char
                .get(start)
                .map_or(self.source_char_len, |(from, _)| *from);
            return (at, at);
        }
        let from = self.per_char[start].0;
        let to = self.per_char[end - 1].1;
        (from, to)
    }
}

/// [`crate::characters::kana::normalize`] with provenance: returns the same
/// normalized string plus the [`KaniNormalizedSpans`] mapping back into the
/// input. The scanner tables mirror `normalize`'s statics — KANA context is
/// `dakuten_join()` alone, DEFAULT prepends `*punctuation-marks*` (keep in
/// sync with `characters/kana.rs`).
pub fn kani_normalize_spanned(
    input: &str,
    context: NormalizationContext,
) -> (String, KaniNormalizedSpans) {
    // Phase 1 is a per-char 1:1 substitution: indices pass through untouched.
    let phase1: String = input
        .chars()
        .map(|ch| to_normal_char(ch, context).unwrap_or(ch))
        .collect();

    static KANA_CONTEXT_SCANNER: LazyLock<KaniNgramScanner> =
        LazyLock::new(|| KaniNgramScanner::new(dakuten_join()));
    static DEFAULT_CONTEXT_SCANNER: LazyLock<KaniNgramScanner> = LazyLock::new(|| {
        let combined: Vec<(&str, &str)> = PUNCTUATION_MARKS
            .iter()
            .copied()
            .chain(dakuten_join().iter().map(|(from, to)| (from.as_str(), to.as_str())))
            .collect();
        KaniNgramScanner::new(&combined)
    });
    let scanner = match context {
        NormalizationContext::Kana => &*KANA_CONTEXT_SCANNER,
        NormalizationContext::Default => &*DEFAULT_CONTEXT_SCANNER,
    };

    let (normalized, per_char) = scanner.simplify_spanned(&phase1);
    let spans = KaniNormalizedSpans {
        per_char,
        source_char_len: input.chars().count(),
    };
    (normalized, spans)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::characters::kana::normalize;

    #[test]
    fn spanned_normalize_equals_normalize() {
        // The v2 serializer maps offsets computed against romanize*'s own
        // normalize output, so the two must agree byte for byte. Covers the
        // punctuation expansion (、 。), a decomposed dakuten pair (か゛,
        // U+309B), and full-width ASCII (１０), in both contexts.
        for input in ["はい、そうです。", "か\u{309b}った", "１０本ください。", "plain"] {
            for context in [NormalizationContext::Default, NormalizationContext::Kana] {
                assert_eq!(kani_normalize_spanned(input, context).0, normalize(input, context));
            }
        }
    }

    #[test]
    fn expansion_maps_both_output_chars_to_the_source_char() {
        // 、 (1 char) becomes ", " (2 chars): both map back to char 1.
        let (normalized, spans) = kani_normalize_spanned("あ、い", NormalizationContext::Default);
        assert_eq!(normalized, "あ, い");
        assert_eq!(spans.source_range(0, 1), (0, 1)); // あ
        assert_eq!(spans.source_range(1, 3), (1, 2)); // ", " ← 、
        assert_eq!(spans.source_range(3, 4), (2, 3)); // い
    }

    #[test]
    fn contraction_maps_the_output_char_to_the_source_pair() {
        // か + standalone dakuten mark ゛ (2 chars) join into が (1 char).
        let (normalized, spans) = kani_normalize_spanned("か\u{309b}た", NormalizationContext::Default);
        assert_eq!(normalized, "がた");
        assert_eq!(spans.source_range(0, 1), (0, 2)); // が ← か゛
        assert_eq!(spans.source_range(1, 2), (2, 3)); // た
        assert_eq!(spans.normalized_len(), 2);
    }
}
