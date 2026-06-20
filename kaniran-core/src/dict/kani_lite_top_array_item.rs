//! Kaniran sidecar (no Lisp FQN). Lite mirror of
//! [`super::path::TopArrayItem`] used inside the
//! `find-best-path` inner loop, with a structurally-shared
//! [`KaniLitePath`] payload.

use std::sync::Arc;

use super::kani_lite_segment_list::KaniLiteSegmentList;
use super::grammar::synergy::Synergy;

#[derive(Debug, Clone)]
pub struct KaniLiteTopArrayItem {
    pub score: i32,
    pub payload: KaniLitePath,
}

/// Lite mirror of [`super::path::PathElement`]. The
/// inner loop builds these; `find-best-path` reconstructs full
/// [`PathElement`]s from them at exit.
///
/// [`PathElement`]: super::path::PathElement
#[derive(Debug, Clone)]
pub enum KaniLitePathElement {
    SegmentList(Arc<KaniLiteSegmentList>),
    Synergy(Synergy),
}

/// Persistent cons-list of path elements, head = the most recently
/// prepended element. This is the structure `(nconc split tail)`
/// (`dict.lisp:1225`) builds upstream: every candidate path shares its
/// tail with the path it extended, so prepending costs one node and
/// cloning costs one refcount bump.
///
/// ```text
/// KaniLitePath(Some(node))      // [seg(の), synergy(noun+prt), seg(本)]
///   node.elem = SegmentList(の)
///   node.next ───────────────►  shared with every sibling path that
///                               extended the same [synergy, seg(本)] tail
/// ```
#[derive(Debug, Clone, Default)]
pub struct KaniLitePath(Option<Arc<KaniLitePathNode>>);

#[derive(Debug)]
pub struct KaniLitePathNode {
    elem: KaniLitePathElement,
    next: KaniLitePath,
}

impl KaniLitePath {
    pub fn nil() -> Self {
        KaniLitePath(None)
    }

    /// Prepend `elem`, sharing `tail` structurally (one node, one
    /// refcount bump).
    pub fn cons(elem: KaniLitePathElement, tail: &KaniLitePath) -> Self {
        KaniLitePath(Some(Arc::new(KaniLitePathNode {
            elem,
            next: tail.clone(),
        })))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_none()
    }

    /// The most recently prepended element (the path's rightmost
    /// segment in sentence order).
    pub fn head(&self) -> Option<&KaniLitePathElement> {
        self.0.as_deref().map(|node| &node.elem)
    }

    /// The shared remainder after `head` — a refcount bump.
    pub fn tail(&self) -> KaniLitePath {
        match self.0.as_deref() {
            Some(node) => node.next.clone(),
            None => KaniLitePath(None),
        }
    }

    /// Walk head→tail (reverse sentence order, same as iterating the
    /// old slice payload front-to-back).
    pub fn iter(&self) -> KaniLitePathIter<'_> {
        KaniLitePathIter { next: self.0.as_deref() }
    }
}

pub struct KaniLitePathIter<'a> {
    next: Option<&'a KaniLitePathNode>,
}

impl<'a> Iterator for KaniLitePathIter<'a> {
    type Item = &'a KaniLitePathElement;

    fn next(&mut self) -> Option<&'a KaniLitePathElement> {
        let node = self.next?;
        self.next = node.next.0.as_deref();
        Some(&node.elem)
    }
}
