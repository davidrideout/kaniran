//! Port of `ichiran/dict:translate-hints` (`dict-split.lisp:899-902`).
//!
//! Re-projects every `(kind, pos)` entry in `hints` through the
//! alignment `matched`. Entries whose position overshoots the
//! alignment (where [`super::translate_hint_position`] returns
//! `None`) drop out of the result — the upstream `if new-pos
//! collect` only collects on non-nil.

use super::kani::KaniHintKind;
use super::kani::KaniMatchPart;
use super::translate_hint_position::translate_hint_position;

pub fn translate_hints(
    matched: &[KaniMatchPart],
    hints: &[(KaniHintKind, usize)],
) -> Vec<(KaniHintKind, usize)> {
    hints
        .iter()
        .filter_map(|(kind, pos)| {
            translate_hint_position(matched, *pos).map(|new_pos| (*kind, new_pos))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Empty hints → empty result.
    #[test]
    fn empty_hints_empty_result() {
        let m = [KaniMatchPart::Atom(3)];
        assert!(translate_hints(&m, &[]).is_empty());
    }

    /// Empty alignment + non-empty hints → empty result (every
    /// hint overshoots).
    #[test]
    fn empty_alignment_drops_all() {
        let hints = [(KaniHintKind::Mod, 1), (KaniHintKind::Space, 2)];
        assert!(translate_hints(&[], &hints).is_empty());
    }

    /// Identity walk over a pure Atom: every position passes
    /// through with the same index, hint kind preserved.
    #[test]
    fn atom_identity_passthrough() {
        let m = [KaniMatchPart::Atom(5)];
        let hints = [(KaniHintKind::Mod, 1), (KaniHintKind::Space, 3)];
        assert_eq!(
            translate_hints(&m, &hints),
            vec![(KaniHintKind::Mod, 1), (KaniHintKind::Space, 3)]
        );
    }

    /// Overshoot drops, in-range survives — output order matches
    /// the surviving subset of input order.
    #[test]
    fn overshoot_filters_out() {
        let m = [KaniMatchPart::Atom(2)];
        let hints = [
            (KaniHintKind::Mod, 1),
            (KaniHintKind::Space, 5),
            (KaniHintKind::Mod, 2),
        ];
        assert_eq!(
            translate_hints(&m, &hints),
            vec![(KaniHintKind::Mod, 1), (KaniHintKind::Mod, 2)]
        );
    }

    /// Walks a mixed Atom/Pair alignment — the position semantics
    /// follow [`translate_hint_position`] (verified there).
    #[test]
    fn mixed_alignment_projection() {
        let m = [
            KaniMatchPart::Atom(2),
            KaniMatchPart::Pair(2, 3),
            KaniMatchPart::Atom(1),
        ];
        let hints = [
            (KaniHintKind::Mod, 1),   // atom-interior: 1
            (KaniHintKind::Space, 3), // pair-interior: 3
            (KaniHintKind::Mod, 4),   // pair-trailing: 5
        ];
        assert_eq!(
            translate_hints(&m, &hints),
            vec![
                (KaniHintKind::Mod, 1),
                (KaniHintKind::Space, 3),
                (KaniHintKind::Mod, 5),
            ]
        );
    }
}
