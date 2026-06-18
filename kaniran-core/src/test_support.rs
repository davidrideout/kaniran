//! Shared fixtures for the in-crate unit tests.
//!
//! Building a context memory-maps the ~1.5 GB rkyv archive and populates
//! every lookup cache, so it is expensive. Each test used to call
//! `KaniranContext::from_env` on its own; under the rkyv backend the test
//! harness ran those builds once per parallel thread and exhausted memory.
//! The whole unit suite links into one test binary, so a single
//! process-wide `OnceLock` lets every test share one context — the first
//! caller builds it, the rest clone the `Arc`.

use std::sync::{Arc, Mutex, OnceLock};

use crate::conn::kani_context::KaniranContext;

/// The process-wide shared context, built on first use from `DATABASE_URL`
/// / `kaniran.toml`. Concurrent first callers block on the single build,
/// then all share the same `Arc`.
pub(crate) fn shared_ctx() -> Arc<KaniranContext> {
    static CTX: OnceLock<Arc<KaniranContext>> = OnceLock::new();
    CTX.get_or_init(|| {
        KaniranContext::from_env().expect("DATABASE_URL / kaniran.toml required")
    })
    .clone()
}

/// The stable JMdict base sequence(s) a conjugated-form `seq` derives from,
/// sorted and deduped. Synthetic conj seqs renumber per build; their base
/// does not, so tests assert on the base instead.
pub(crate) fn resolve_base_seqs(seq: i32) -> Vec<i32> {
    let ctx = shared_ctx();
    let mut bases: Vec<i32> = crate::dict::conj::select_conjs(&ctx, seq, None)
        .expect("select_conjs")
        .iter()
        .map(|conj| conj.seq_from)
        .collect();
    bases.sort_unstable();
    bases.dedup();
    bases
}

/// Assert that conjugated-form `seq` resolves to `expected` base seqs.
///
/// When `KANI_CAPTURE_BASES` is set the call records `file:line -> bases`
/// to `/tmp/base_capture.txt` instead of asserting — used once to harvest
/// the true bases the engine computes, which are then baked in as
/// `expected`. `#[track_caller]` reports the call site, not this function.
#[track_caller]
pub(crate) fn check_base_seqs(seq: i32, expected: &[i32]) {
    report(resolve_base_seqs(seq), expected, std::panic::Location::caller());
}

/// Like [`check_base_seqs`] for a group of conjugated forms: the sorted,
/// deduped union of every input seq's base seqs. Used where a test pins a
/// vec/set of result seqs — the synthetic sort order isn't stable either,
/// so the base *set* is the right invariant.
#[track_caller]
pub(crate) fn check_base_seq_set(seqs: &[i32], expected: &[i32]) {
    let mut got: Vec<i32> = seqs.iter().flat_map(|seq| resolve_base_seqs(*seq)).collect();
    got.sort_unstable();
    got.dedup();
    report(got, expected, std::panic::Location::caller());
}

/// Assert two part-of-speech lists are equal as sets. Segment POS order
/// isn't stable across archive builds, so compare sorted.
#[track_caller]
pub(crate) fn assert_pos_set(got: &[String], want: &[&str]) {
    let mut got: Vec<&str> = got.iter().map(String::as_str).collect();
    got.sort_unstable();
    let mut want = want.to_vec();
    want.sort_unstable();
    assert_eq!(got, want);
}

fn report(got: Vec<i32>, expected: &[i32], loc: &'static std::panic::Location<'static>) {
    if std::env::var_os("KANI_CAPTURE_BASES").is_some() {
        use std::io::Write;
        static LOCK: Mutex<()> = Mutex::new(());
        let _guard = LOCK.lock().unwrap();
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/base_capture.txt")
            .expect("open base capture file");
        writeln!(file, "{}:{}\t{:?}", loc.file(), loc.line(), got).expect("write capture");
        return;
    }
    assert_eq!(got, expected, "base seqs at {loc}");
}
