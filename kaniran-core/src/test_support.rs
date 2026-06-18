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

use serde_json::Value;

use crate::conn::kani_backend::KaniBackend;
use crate::conn::kani_context::KaniranContext;

/// Conjugated-form entries are stored as text/conjugation rows whose seqs and
/// ids the loader assigns afresh on every build; a JMdict base entry's seq
/// stays below this value, a synthetic conjugated-entry seq at or above it.
/// Tests resolve anything synthetic at runtime instead of pinning the
/// per-build number.
pub(crate) const SYNTHETIC_SEQ_MIN: i32 = 10_000_000;

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

/// Assert seq `actual` is the stable base seq `expected`: directly when
/// `actual` is itself a base seq, or via its conjugation base when `actual`
/// is a synthetic conjugated-entry seq. Lets a test pin a result seq that is
/// stable for base entries and renumbers for conjugated ones with one call.
#[track_caller]
pub(crate) fn check_seq_or_base(actual: i32, expected: i32) {
    if actual < SYNTHETIC_SEQ_MIN {
        assert_eq!(actual, expected, "stable base seq");
    } else {
        check_base_seqs(actual, &[expected]);
    }
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

/// The synthetic conjugated-entry seq whose surface form is `surface` (e.g.
/// "つきはてた" → the past-tense entry of 尽き果てる). The seq renumbers per
/// build, so tests that feed a conjugated entry resolve it from its stable
/// surface instead of hardcoding the number. Panics unless exactly one
/// synthetic entry carries that surface.
pub(crate) fn conj_entry_seq(surface: &str) -> i32 {
    let seqs = conj_entry_seqs(surface);
    assert_eq!(seqs.len(), 1, "surface {surface:?} → synthetic seqs {seqs:?}");
    seqs[0]
}

/// Every synthetic conjugated-entry seq whose surface is `surface`, sorted
/// and deduped. A surface can belong to more than one conjugated entry (two
/// distinct conjugation paths landing on the same spelling); callers that
/// need the unique one use [`conj_entry_seq`], those that disambiguate
/// further use this. Panics when the surface has no synthetic entry.
pub(crate) fn conj_entry_seqs(surface: &str) -> Vec<i32> {
    let ctx = shared_ctx();
    let mut seqs: Vec<i32> = Vec::new();
    if let Ok(rows) = ctx.store.kanji_texts_by_text(surface) {
        seqs.extend(rows.iter().map(|row| row.seq));
    }
    if let Ok(rows) = ctx.store.kana_texts_by_text(surface) {
        seqs.extend(rows.iter().map(|row| row.seq));
    }
    seqs.retain(|seq| *seq >= SYNTHETIC_SEQ_MIN);
    seqs.sort_unstable();
    seqs.dedup();
    assert!(!seqs.is_empty(), "surface {surface:?} has no synthetic entry");
    seqs
}

/// The conjugation-table id for base `seq`'s conjugation whose first prop
/// carries part-of-speech `pos` (e.g. "v1", "v5m"). Only the via-empty rows
/// `select_conjs` surfaces are searched. Conj ids renumber per build, so
/// tests that need to name one resolve it from the stable `pos` instead.
pub(crate) fn conj_id_by_pos(seq: i32, pos: &str) -> i32 {
    let ctx = shared_ctx();
    let rows = crate::dict::conj::select_conjs_and_props(
        &ctx,
        seq,
        None,
        crate::dict::conj::FilterPropsText::None,
    )
    .expect("select_conjs_and_props");
    let mut ids: Vec<i32> = rows
        .iter()
        .filter(|(_, props, _)| props.first().is_some_and(|prop| prop.pos == pos))
        .map(|(conj, _, _)| conj.id)
        .collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 1, "base {seq} pos {pos:?} → conj ids {ids:?}");
    ids[0]
}

/// The conj id of base `seq`'s single via-set (non-null `via`) conjugation
/// row. The via-empty-preferring `select_conjs` hides these, so enumerate the
/// full row set directly. Ids renumber per build.
pub(crate) fn via_set_conj_id(seq: i32) -> i32 {
    let ctx = shared_ctx();
    let rows = ctx.store.conjs_by_seq(seq).expect("conjs_by_seq");
    let mut ids: Vec<i32> = rows
        .iter()
        .filter(|conj| conj.seq_via.is_some())
        .map(|conj| conj.id)
        .collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 1, "base {seq} via-set conj ids {ids:?}");
    ids[0]
}

/// Recursively rewrite every `"seq"` holding a synthetic conjugated-entry seq
/// to `0`, leaving stable base seqs intact.
fn zero_synthetic_seqs(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, val) in map.iter_mut() {
                if key == "seq" {
                    if let Some(seq) = val.as_i64() {
                        if seq >= i64::from(SYNTHETIC_SEQ_MIN) {
                            *val = Value::from(0);
                        }
                    }
                }
                zero_synthetic_seqs(val);
            }
        }
        Value::Array(items) => items.iter_mut().for_each(zero_synthetic_seqs),
        _ => {}
    }
}

/// Assert `got` JSON equals the `expected` JSON literal, ignoring synthetic
/// conjugated-entry seqs (which renumber per build) and object key order;
/// arrays stay ordered. Stable base seqs are still compared.
#[track_caller]
pub(crate) fn assert_json_seq_agnostic(got: &Value, expected: &str, label: &str) {
    let mut got = got.clone();
    let mut expected: Value = serde_json::from_str(expected).expect("parse expected json");
    zero_synthetic_seqs(&mut got);
    zero_synthetic_seqs(&mut expected);
    assert_eq!(got, expected, "{label}");
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
