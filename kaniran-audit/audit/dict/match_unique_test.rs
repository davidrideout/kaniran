//! Manual fixture-replay runner for `ICHIRAN/DICT:MATCH-UNIQUE`.
//! Source under test: `src/dict/match_unique.rs`.
//!
//! Run with:
//!   cargo run --bin match_unique_test -- \
//!       --path corpus/<corpus_tag>/dict/match_unique.parquet
//!
//! ## Args (2 positional, no keywords)
//!
//! - `args[0]` — suffix class as a Lisp keyword (`":IRU"`, `":SA"`,
//!   `":DESU"`, …). The Rust port stores classes lowercase without
//!   the leading colon ([`super::super::_star_suffix_unique_only_star_::SUFFIX_UNIQUE_ONLY`]
//!   uses `"sa"`, `"desu"`, …), so the runner strips `:` and
//!   lowercases before lookup. Mirrors the same convention used for
//!   `*suffix-cache*` keys.
//! - `args[1]` — `(list <match> …)` where each match is a `KANA-TEXT`
//!   or `KANJI-TEXT` DAO mirror. Smoke parquet (26,122 rows) contains
//!   only those two classes — consistent with the runtime trace
//!   verified in the source-under-test module doc.
//!
//! ## Result shapes (multi-value from postmodern's `query :column`)
//!
//! Lisp `(values …)` flattens to a top-level JSON array:
//!
//! - `[null]` — primary NIL. Either (a) class not in
//!   `*suffix-unique-only*`, (b) `:sa` with empty `seqs` (closure
//!   `(and nil _)` short-circuits, single value), or (c) `:desu`
//!   returning NIL.
//! - `[":KEYWORD"]` — bare-entry hit; primary is the matched keyword
//!   itself (which equals the input suffix class on a bare hit).
//! - `[true]` — `:desu` closure returned T.
//! - `[[seq, …], <count>]` — `:sa` truthy path; primary is the list
//!   of root seqs returned by `(:select 'seq :from 'entry …
//!   :where 'root-p)`, secondary is postmodern's row count.
//! - `[null, 0]` — `:sa` with non-empty `seqs` whose query returned
//!   zero root rows. Primary NIL is observable; secondary 0 is the
//!   postmodern row count, not part of the Rust return contract.
//!
//! ## Comparison policy
//!
//! - `None`  ⇔  `[null]`  *or*  `[null, 0]`. Rust collapses both
//!   subcases of falsy `:sa` into a single `None` because the only
//!   known caller is a predicate (`find-word-suffix` at
//!   `dict-grammar.lisp:705`).
//! - `Some(Bare)`  ⇔  `[":KEYWORD"]` where the keyword **must equal**
//!   the input suffix class. Pinned because the Lisp `find` returns
//!   the matched element by identity — if upstream ever returns a
//!   different class for the same lookup, that's a real divergence.
//! - `Some(Desu)`  ⇔  `[true]`.
//! - `Some(Sa(rows))`  ⇔  `[[seq, …], <count>]`. The seq list is
//!   compared as a sorted set (Postgres' row order is unspecified
//!   without `ORDER BY`; both sides come from the same query but may
//!   permute). The captured count must equal `rows.len()`.

#[path = "../common/mod.rs"]
mod common;

use serde_json::Value;

use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::dict::kani::KaniWordDispatchEnum;
use kaniran_core::dict::match_unique::{match_unique, MatchUniqueResult};

use common::{parse_captured_word, CapturedRow};

const EXPECTED_FQN: &str = "ICHIRAN/DICT:MATCH-UNIQUE";

async fn audit_one(ctx: &KaniranContext, row: &CapturedRow) -> Result<(), String> {
    if row.args.len() != 2 {
        return Err(format!("expected 2 args, got {}", row.args.len()));
    }

    let raw_class = row.args[0]
        .as_str()
        .ok_or_else(|| format!("arg 0 not string keyword: {}", row.args[0]))?;
    let lookup_class = raw_class
        .strip_prefix(':')
        .ok_or_else(|| format!("arg 0 missing leading colon: {}", raw_class))?
        .to_lowercase();

    let arg_matches = row.args[1]
        .as_array()
        .ok_or_else(|| format!("arg 1 not array: {}", row.args[1]))?;
    let matches: Vec<KaniWordDispatchEnum> = arg_matches
        .iter()
        .map(parse_captured_word)
        .collect::<Result<_, _>>()?;

    let actual = match_unique(ctx, &lookup_class, &matches)
        .await
        .map_err(|err| format!("match_unique query: {}", err))?;

    if row.result.is_empty() {
        return Err("result envelope empty".into());
    }
    let primary = &row.result[0];
    let secondary = row.result.get(1);

    compare(&actual, primary, secondary, raw_class)
}

fn compare(
    actual: &Option<MatchUniqueResult>,
    primary: &Value,
    secondary: Option<&Value>,
    raw_class: &str,
) -> Result<(), String> {
    match actual {
        // dict-grammar.lisp:689-693 — `find` miss OR `:sa`/`:desu`
        // closure returning nil. Both Lisp shapes (`[null]` and
        // `[null, 0]`) collapse to `None` per the module doc.
        None => {
            if primary.is_null() {
                Ok(())
            } else {
                Err(format!(
                    "rust=None vs lisp primary={} secondary={:?}",
                    primary, secondary
                ))
            }
        }
        // dict-grammar.lisp:692 — `(cond (t uniq))` returns the matched
        // keyword itself. Identity-pin: the bare-entry result keyword
        // must be the input suffix class.
        Some(MatchUniqueResult::Bare) => {
            let p = primary.as_str().ok_or_else(|| {
                format!("rust=Some(Bare) vs lisp primary (not string): {}", primary)
            })?;
            if p != raw_class {
                return Err(format!(
                    "bare keyword mismatch: rust=Some(Bare) input={} lisp={}",
                    raw_class, p
                ));
            }
            Ok(())
        }
        // dict-grammar.lisp:522-530 — `:desu` closure returns T from
        // `(< len-conjs len-matches)`.
        Some(MatchUniqueResult::Desu) => {
            if matches!(primary, Value::Bool(true)) {
                Ok(())
            } else {
                Err(format!("rust=Some(Desu) vs lisp primary={}", primary))
            }
        }
        // dict-grammar.lisp:486-490 — `:sa` closure returns the
        // non-empty list of root seqs (truthy) plus postmodern's row
        // count as the secondary multi-value.
        Some(MatchUniqueResult::Sa(rows)) => {
            let arr = primary.as_array().ok_or_else(|| {
                format!("rust=Some(Sa) vs lisp primary (not array): {}", primary)
            })?;
            let mut captured: Vec<i32> = arr
                .iter()
                .map(|v| {
                    v.as_i64().map(|n| n as i32).ok_or_else(|| {
                        format!("non-int element in :sa primary array: {}", v)
                    })
                })
                .collect::<Result<_, _>>()?;
            let mut actual_sorted = rows.clone();
            actual_sorted.sort();
            captured.sort();
            if actual_sorted != captured {
                return Err(format!(
                    ":sa seq list mismatch\n  rust (sorted): {:?}\n  lisp (sorted): {:?}",
                    actual_sorted, captured
                ));
            }
            // Postmodern emits `(values list count)` for `query :column`.
            // Pin the count against `rows.len()` whenever it's present.
            if let Some(sec) = secondary {
                let cap_count = sec.as_i64().ok_or_else(|| {
                    format!(":sa secondary not int: {}", sec)
                })?;
                if cap_count as usize != rows.len() {
                    return Err(format!(
                        ":sa row count mismatch: rust={} lisp={}",
                        rows.len(),
                        cap_count
                    ));
                }
            }
            Ok(())
        }
    }
}

#[tokio::main]
async fn main() {
    common::run_async(EXPECTED_FQN, audit_one).await;
}
