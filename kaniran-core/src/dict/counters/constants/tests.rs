use super::*;

// --- _star_special_counters_star_ ---
/// Build-loop regression (CONVENTIONS §6): the registry should
/// hold exactly one entry per `def-special-counter` callsite in
/// upstream `dict-counters.lisp`. Drift here means a duplicate
/// `m.insert` (silently overwriting) or a missing one.
#[test]
fn builds_91_entries_one_per_upstream_callsite() {
    let map = build_special_counters();
    assert_eq!(map.len(), 91, "expected 91 special-counter seqs");
}

/// Pin the iteration shape: a registered fn called with empty
/// readings should still return a valid (possibly source-less)
/// `Vec<CounterArgs>`. Catches the case where a callsite forgot
/// to wrap its output in `vec![...]` or returned a wrong type.
#[test]
fn every_fn_runs_on_empty_readings() {
    let map = build_special_counters();
    for (seq, f) in &map {
        let out = f(&[], &[]);
        assert!(!out.is_empty(), "seq {} returned no entries", seq);
        for a in &out {
            assert!(!a.text.is_empty(), "seq {}: empty text", seq);
            assert!(!a.kana.is_empty(), "seq {}: empty kana", seq);
            // No source resolves against empty readings.
            assert!(
                a.source.is_none(),
                "seq {}: source resolved against empty readings",
                seq
            );
        }
    }
}
