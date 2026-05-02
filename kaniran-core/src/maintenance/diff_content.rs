//! Port of `ichiran/maintenance:diff-content` (`ichiran.lisp:140-147`).
//!
//! Compare two text blobs split on `[\r\n]+` and return a unified diff
//! or a sentinel marking that one side is missing. Used upstream to
//! summarize JMdict entry-content changes between two database
//! snapshots.
//!
//! Divergences from Lisp:
//! - "Missing" inputs are modeled as [`Option<&str>`] rather than
//!   `nil`, separating the absent case from an explicit empty string.
//! - The `(or simple-string (member :gone :new))` return type collapses
//!   to the [`DiffResult`] enum (CONVENTIONS §4.3).
//! - The `&key short` keyword stays a [`bool`]: the parameter name
//!   reads as "short-circuit when one side is absent" at the callsite,
//!   so the polarity is clear without an enum (CONVENTIONS §4.4).
//! - Diff library: [`similar`] is the closest Rust equivalent of
//!   upstream `cl-diff`'s unified-diff renderer. Output text is not
//!   guaranteed byte-identical with cl-diff; tests pin behavior, not
//!   literal output.
//!
//! ## `None` vs `Some("")`
//!
//! Only [`None`] triggers the [`DiffResult::Gone`] / [`DiffResult::New`]
//! sentinels — [`Some("")`] is treated as a present-but-empty input
//! and flows through the diff path. This matches upstream's
//! NULL-vs-empty distinction on the JMdict `entry.content` column:
//! a SQL `NULL` becomes `nil` (sentinel); an empty `TEXT` value stays
//! as `""` and produces a normal (empty-body) diff.
//!
//! In Lisp the distinction is implicit in the `(and old (split-by-regex
//! ...))` truthiness chain — `nil` short-circuits, `""` reaches
//! `split-by-regex` and returns a non-`nil` list. The Rust port makes
//! the boundary explicit at the function signature and in the
//! [`Option::is_none`] checks below.

use crate::characters::split_by_regex::split_by_regex;
use fancy_regex::Regex;
use similar::TextDiff;
use std::sync::OnceLock;

static NEWLINE_RE: OnceLock<Regex> = OnceLock::new();

fn newline_re() -> &'static Regex {
    NEWLINE_RE.get_or_init(|| Regex::new(r"[\r\n]+").expect("newline regex compiles"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffResult {
    Gone,
    New,
    Diff(String),
}

pub fn diff_content(old: Option<&str>, new: Option<&str>, short: bool) -> DiffResult {
    if short && new.is_none() {
        return DiffResult::Gone;
    }
    if short && old.is_none() {
        return DiffResult::New;
    }
    let old_lines = old.map(|s| split_by_regex(newline_re(), s)).unwrap_or_default();
    let new_lines = new.map(|s| split_by_regex(newline_re(), s)).unwrap_or_default();
    let old_refs: Vec<&str> = old_lines.iter().map(String::as_str).collect();
    let new_refs: Vec<&str> = new_lines.iter().map(String::as_str).collect();
    let diff = TextDiff::from_slices(&old_refs, &new_refs);
    DiffResult::Diff(diff.unified_diff().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_gone_when_new_is_missing() {
        assert_eq!(diff_content(Some("anything"), None, true), DiffResult::Gone);
    }

    #[test]
    fn short_new_when_old_is_missing() {
        assert_eq!(diff_content(None, Some("anything"), true), DiffResult::New);
    }

    #[test]
    fn short_false_returns_diff_even_when_one_side_is_missing() {
        // With short=false, the sentinel branch is skipped and a real
        // diff is computed against the empty side.
        match diff_content(Some("hello"), None, false) {
            DiffResult::Diff(_) => {}
            other => panic!("expected Diff, got {other:?}"),
        }
    }

    #[test]
    fn identical_inputs_diff_is_empty_body() {
        // Unified diff of two identical line-lists has no @@ hunks, so
        // the body is empty. Pins the "no changes" path without
        // asserting against any specific diff syntax.
        let d = diff_content(Some("a\nb\nc"), Some("a\nb\nc"), false);
        match d {
            DiffResult::Diff(s) => assert!(s.is_empty(), "expected empty body, got {s:?}"),
            other => panic!("expected Diff, got {other:?}"),
        }
    }

    #[test]
    fn empty_string_is_not_collapsed_to_missing() {
        // Pins the None vs Some("") boundary: an empty string is a
        // present-but-empty input, not a missing one. Even with
        // short=true, the sentinels only fire on None.
        assert!(matches!(
            diff_content(Some(""), Some("hello"), true),
            DiffResult::Diff(_)
        ));
        assert!(matches!(
            diff_content(Some("hello"), Some(""), true),
            DiffResult::Diff(_)
        ));
        // None on the same side still triggers the sentinel.
        assert_eq!(diff_content(None, Some("hello"), true), DiffResult::New);
        assert_eq!(diff_content(Some("hello"), None, true), DiffResult::Gone);
    }

    #[test]
    fn newlines_collapse_via_split_by_regex() {
        // Upstream splits on [\r\n]+ (greedy), so multiple blank lines
        // collapse rather than appearing as empty diff entries.
        // "a\r\n\r\nb" splits to ["a", "b"] — the in-between blank
        // doesn't produce a third line that has to diff away.
        let same = diff_content(Some("a\nb"), Some("a\r\n\r\nb"), false);
        match same {
            DiffResult::Diff(s) => assert!(s.is_empty(), "blank-line collapse failed: {s:?}"),
            other => panic!("expected Diff, got {other:?}"),
        }
    }
}
