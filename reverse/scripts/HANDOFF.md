# Handoff — Rust port of ichiran

Read [`CLAUDE.md`](../../CLAUDE.md) and [`CONVENTIONS.md`](../../CONVENTIONS.md) first for orientation. This doc is the current snapshot.

**Baseline commits this session (3, on top of `d2b0d1a`):**
1. transliterate and mirror without improvements `ichiran/numbers` — 14 symbols, 35 new tests, package now closed.
2. Add signature-drift audit + introspector-bug guard — `signatures.json`, `query.py audit-signatures`, committed `divergences.md`.
3. This HANDOFF update.

`ichiran-repl.sh` remains untracked (intentional — local wrapper, has the `.103` host hardcoded).

---

## tl;dr — current state

| Layer | Status |
|---|---|
| Original Lisp source | Checked in at repo root, untouched. |
| **Symbol / dependency extraction** (md files) | Done. **945 md files** under `reverse/<file>.lisp/` covering 6 introspected kinds + 2 hand-written. |
| **Graph CSVs** (symbols.csv, edges.csv) | Done. **944 symbols** + 3145 edges. |
| **Signatures** (`signatures.json`) | NEW this session. 763 entries (689 fn + 36 macro + 38 gf) keyed by FQN, with lambda list and declared ftype. Used by `audit-signatures`. |
| **PORT_PLAN.md** | Regenerated. **872 waves, 923 symbols.** |
| **Marked status** | **79 ported** (was 65) + **28 skip** (unchanged). |
| **Tracer** (`:ichi-trace` in `ichiran-repl.sh`) | Built, proven via probes. Has not yet been run on `(ichiran/test:run-all-tests)`. |
| **Rust crate `kaniran-core`** | 79 ports + 4 Rust-only sidecars. Five package directories: `characters/`, `conn/`, `core/`, `maintenance/`, **`numbers/`** (new). **102 tests pass.** |
| **Audit infrastructure** | NEW. `reverse/scripts/divergences.md` is committed and deterministic; `query.py audit-signatures` rewrites it on every run. Currently lists 6 divergences — 4 documented in port doc-comments (CONVENTIONS §4.4 / §4.6), 2 new from the numbers port. |

`ichiran/numbers` is now closed: **14 ported / 0 pending / 0 skip.** Same shape as `ichiran/conn` (closed) and `ichiran/characters` (closed). Three packages fully ported.

---

## Layout

```
kaniran/
├── Cargo.toml                       # workspace
├── Cargo.lock
├── CLAUDE.md
├── CONVENTIONS.md                   # §7 now lists audit-signatures alongside cargo check / cargo test
├── kaniran-core/
│   └── src/
│       ├── lib.rs                   # `pub mod kani; characters; conn; core; maintenance; numbers;`
│       ├── kani.rs / kani/          # naming.rs + fixture.rs
│       ├── characters/              # 61 ports + 3 sidecars
│       ├── conn/                    # 2 ports + 1 sidecar (KaniranContext)
│       ├── core/                    # 1 port
│       ├── maintenance/             # 1 port
│       └── numbers/                 # NEW — 14 ports + 1 sidecar (kani_num_class)
│           ├── mod.rs
│           ├── kani_num_class.rs                       # NumClass enum (:jd / :p / :ad)
│           ├── _star_*_star_.rs                        # 7 globals
│           ├── number_to_kanji.rs / number_to_kana.rs
│           ├── parse_number.rs / parse_number_star_.rs # note trailing _ on file stem (* in Lisp name)
│           ├── group_to_kana.rs / num_sandhi.rs
│           └── not_a_number_condition.rs
└── reverse/
    └── scripts/
        ├── build_graph.py           # gated for full regen; --signatures-only bypasses
        ├── query.py                 # NEW subcommand: audit-signatures
        ├── symbols.csv
        ├── edges.csv
        ├── signatures.json          # NEW — 763 callable signatures
        ├── divergences.md           # NEW — committed audit output
        ├── PORT_PLAN.md
        ├── HANDOFF.md
        └── README.md
```

---

## What got built / changed this session

### 1. Ported `ichiran/numbers` (14 symbols)

Full leaf-up port of the Japanese number system. No DB, no regex, no I/O — pure transformation.

| Symbol | Rust shape | Notes |
|---|---|---|
| 7 globals (`*digit-kanji-default*`, etc.) | `pub const` slices + 1 `OnceLock<HashMap>` | `*char-number-class-hash*` derives lazily from `*char-number-class*`. Test pins to introspector-captured count of 42 entries. |
| `number-to-kanji` | `fn(n, digits, powers, one_sen) -> String` | Single 4-arg fn matching the Lisp keyword surface. |
| `parse-number` | `fn(&str) -> Result<u64, NotANumber>` | Returns `Err` for any glyph not in the lookup table. |
| `parse-number*` | `fn(&[(NumClass, u8)]) -> u64` | Drops Lisp `:start :end` for Rust slice idiom. **Surfaces in divergences.md.** |
| `num-sandhi` | `fn(c1, v1, c2, v2, s1, s2) -> String` | Was `defgeneric`; collapsed to single `fn` with `match` over `(prev, cur)` quadruple. 6-arg signature preserved. |
| `group-to-kana` | `fn(group, digit_table, power_table) -> String` | Lisp's combined `:class-to-kana` plist split into two table args. **Surfaces in divergences.md.** |
| `number-to-kana` | `fn(n, sep: Option<char>, kanji_method: impl Fn(u64) -> String) -> NumberToKanaOutput` | Returns enum — `Joined(String)` or `Groups(Vec<String>)` matching Lisp's `:separator nil` branch. |
| `NotANumber` (condition) | `pub struct NotANumber { text, reason }` with `thiserror::Error` | |

35 new tests, every one verified against the live Lisp on `.103` via REPL pinning (number-to-kanji, parse-number, number-to-kana with both space and `*kana-hint-space*`/U+200B separator).

Test count: 65 → **102** (+37; some pre-existing tests were also unaffected so the audit math doesn't perfectly equal 35).

### 2. Discovered an introspector bug — `not-a-number` mis-packaged

`symbols.csv` had `ichiran:not-a-number,not-a-number,ichiran,...`, but `(find-symbol "NOT-A-NUMBER" (find-package :ichiran/numbers))` on the live Lisp returns `ICHIRAN/NUMBERS:NOT-A-NUMBER`. The introspector emitted the wrong package field for this one symbol. **Corrected the row in place** to `ichiran/numbers:not-a-number,not-a-number,ichiran/numbers,...` so the FQN→Rust path translator (`kani::naming`) routes the file to `numbers/` instead of `core/`.

### 3. Added `build_graph.py` regen guard

To prevent silent re-introduction of the `not-a-number` row when someone next runs `build_graph.py`, the full regen path now raises a `RuntimeError` at startup with a multi-line explanation (the bug, how to verify, how to unblock by fixing `introspect.lisp`).

`--signatures-only` flag bypasses the guard — safe because signatures only cover callable kinds (fn / macro / gf), and the bug only affects the `not-a-number` condition (skipped).

### 4. Added `signatures.json` extractor in `build_graph.py`

763 entries (689 fn + 36 macro + 38 gf). Each entry: `{name, package, file, line, kind, lambda_list, ftype, docstring}`. Pulled from the `## Inputs` and `## Outputs` blocks of the md files — no introspector change needed; the lambda list was already captured.

```sh
python3 reverse/scripts/build_graph.py --signatures-only   # safe, ungated
```

### 5. Added `query.py audit-signatures` subcommand

Cross-references every ported `pub fn` against the captured Lisp lambda list. Always runs the full sweep, **always rewrites `reverse/scripts/divergences.md`** (deterministic, sorted by FQN, designed to diff cleanly when committed).

Catches:
- Arity drift (e.g. dropped keyword).
- Missing `pub fn <expected_name>` (e.g. file exists but the function is named differently).
- Extra `pub fn` siblings in the same port file (the `_with` failure mode).
- Lambda-parse fallbacks (introspector returned no lambda list AND no parseable ftype).

Macros are checked for file-existence only (CONVENTIONS §4.8 — most macros port to doc-only files).

Flags:
- `--only <pkg>` — filter STDOUT only; the file always reflects the full sweep.
- `--no-write` — suppress file rewrite (rare).

Wired into CONVENTIONS §7 (verify step) and CLAUDE.md (Common commands).

### 6. Current state of `divergences.md` (6 entries, all real)

| Symbol | Drift | Reason |
|---|---|---|
| `characters:geminate` | arity 1 ≠ 2 | dropped `:fresh` per CONVENTIONS §4.6 (always allocate) |
| `characters:join` | arity 2 ≠ 3 | dropped `:key` (one upstream callsite, easy to do at call site) |
| `characters:rendaku` | arity 2 ≠ 3 | dropped `:fresh` (§4.6) + collapsed `:handakuten` to enum (§4.4) |
| `characters:unrendaku` | arity 1 ≠ 2 | dropped `:fresh` (§4.6) |
| `numbers:group-to-kana` | arity 3 ≠ 2 | split Lisp `:class-to-kana` plist into two table args |
| `numbers:parse-number*` | arity 1 ≠ 3 | dropped `:start :end` for Rust slice idiom |

The first 4 are codified conventions (apply across all ports). The last 2 are local judgment calls in the numbers port; documented in their port doc-comments. None are bugs — but they're now visible in version control, so future drift / silent regression is catchable via `git diff reverse/scripts/divergences.md`.

---

## Frozen-literal divergences (staleness ledger)

Per CONVENTIONS §3.4 and §5.3.

| File | Upstream construction | Stales when these change |
|---|---|---|
| `_star_abnormal_chars_star_.rs` | `(concatenate 'string "...full-width ASCII..." *half-width-kana*)` at `characters.lisp:106-109` | `*half-width-kana*` (item 13, ported). All inputs ported — ready to flip to a `format!`-derivation parallel to `*normal-chars*` whenever picked up. |

Unchanged this session.

---

## Decisions

### Resolved (carried over)

- **DB layer:** sqlx + tokio (async). Connection on `KaniranContext` sidecar. Multi-DB usage = "construct another `KaniranContext`."
- **Connection URL format:** standard Postgres URL via `DATABASE_URL` env var.
- **`ichiran/conn` package shape:** closed (2 ported, 24 skipped).

### Resolved this session

- **Audit-signatures is part of the port-completion checklist** (CONVENTIONS §7).
- **`divergences.md` is the durable record** of every Rust port whose `pub fn` surface differs from the captured Lisp lambda list. Committed.
- **`build_graph.py` full regen is gated** until the introspector's package field is fixed.

### Still open

1. **JMdict schema** — share ichiran's Postgres schema or design fresh. Pairs with the DB decision; sqlx works either way but DAO ports look different. Will become unavoidable when the first DAO port lands (wave 86: `ichiran/dict:conjugation`, then `simple-text` / `kana-text` / `kanji-text` at 87–89).
2. **XML reader** for the initial JMdict.xml corpus load. Candidates: `roxmltree`, `quick-xml`, `xmltree`. JMdict's external DTD entities need a small inlining preprocessing step. Triggered when corpus-load work begins (`dict-load.lisp` chunk).
3. **Port scope** — full port (~944 symbols) vs. romanize/segment public API only (~100 symbols).

The repo is intended as a multi-crate workspace; future siblings (`kaniran-cli`, `kaniran-demo`) will live at the repo root.

---

## Next in the plan

`query.py next` reports **6 unblocked symbols** (was 7; `num-sandhi` came off the list when `numbers/` closed):

| FQN | Kind | File | DB-decision dependent? |
|---|---|---|---|
| `ichiran:get-character-classes` | fn | `romanize.lisp:3` | No — pure char-class transform. |
| `ichiran/custom:as-xml-simple` | fn | `dict-custom.lisp:225` | XML-library decision (#2 in open list). |
| `ichiran/custom:normalize-geo` | fn | `dict-custom.lisp:176` | No — small string cleanup. |
| `ichiran/dict:find-sticky-positions` | fn | `dict.lisp:990` | Touches DAOs — needs JMdict-schema decision. |
| `ichiran/dict:process-word-info` | fn | `dict.lisp:1417` | Touches DAOs — needs JMdict-schema decision. |
| `ichiran/kanji:get-original-reading` | fn | `kanji.lisp:308` | Touches DAOs — needs JMdict-schema decision. |

**Going strictly in plan order:** the next non-skipped item after the conn block + numbers package is **wave 73: `ichiran/dict:*counter-cache*`** (still — numbers porting was a parallel package, not in the strict-order critical path). Each of the three DB-backed dict caches becomes a typed `OnceCell<T>` field on `KaniranContext` once its builder ports. Builders need both the JMdict-schema decision and the DAOs they query.

**Three things an agent can do right now without further blocking decisions:**

1. **Port the two non-DB unblocked leaves** (`ichiran:get-character-classes`, `ichiran/custom:normalize-geo`). Two more files, same shape as previous-session leaves.
2. **Run the tracer against the ichiran test suite and harvest fixtures** (`(ichi-trace:install-many '(...))` then `(ichiran/test:run-all-tests)` then `(ichi-trace:dump-per-symbol "/tmp/fixtures/")`). Doesn't touch the Rust crate — pure data harvest for later replay.
3. **Fix the introspector** (`reverse/scripts/introspect.lisp` — symbol's package should come from `(package-name (symbol-package sym))`, not whatever it's currently using), then re-run on `.103`, then delete the `build_graph.py` gate. Unblocks future regenerations of `symbols.csv` from updated upstream.

**Decisions that gate further progress:**
1. **JMdict schema** — required before any DAO port (waves 86+).
2. **XML reader** — required when corpus-load work begins.
3. **Port scope** — affects how aggressive the JMdict + DAO decisions need to be.

**User's call; don't pick autonomously.**

---

## Resume — first three commands

```sh
# 1. Confirm the Rust crate compiles + tests pass
cargo test -p kaniran-core --lib
# expect: 102 passed

# 2. Confirm divergences.md still matches the working tree (audit is deterministic)
python3 reverse/scripts/query.py audit-signatures
git diff reverse/scripts/divergences.md
# expect: no diff. If divergent, the `pub fn` surface drifted since the last commit.

# 3. See what's unblocked next (6 symbols — see "Next in the plan")
python3 reverse/scripts/query.py next
```

To inspect the audit's record:

```sh
cat reverse/scripts/divergences.md
```

To inspect signatures.json for any specific port:

```sh
python3 -c "import json; d=json.load(open('reverse/scripts/signatures.json')); print(json.dumps(d['ichiran/numbers:number-to-kanji'], indent=2, ensure_ascii=False))"
```

---

## Gotchas (still real + new)

- **`build_graph.py` full regen is gated.** Running it without `--signatures-only` raises a `RuntimeError`. The gate explains the introspector's `not-a-number` package mis-recording and how to unblock. `--signatures-only` works fine.
- **`signatures.json` only covers callable kinds** (fn / macro / gf — 763 entries). Globals, structs, classes, daos, types, conditions are not in the file (they have no callable surface to audit).
- **`divergences.md` is committed** and rewritten on every `audit-signatures` run. After every port: `git diff reverse/scripts/divergences.md` is the review surface. New entries are either intentional (cite CONVENTIONS) or port bugs (fix and re-run until they vanish).
- **`audit-signatures --only <pkg>` filters STDOUT only.** The file always reflects the full sweep; do not interpret a partial-stdout run as authorizing a partial file.
- **`run-remote.sh` rsyncs with `--delete --exclude='scripts/'`.** Two hand-written md files (`reverse/characters.lisp/char-class_type.md`, `reverse/numbers.lisp/not-a-number_condition.md`) are at risk on next introspection run. Either commit before re-running, or back them up.
- **The introspector's package field is currently wrong for `not-a-number`** (records `ichiran` instead of `ichiran/numbers`). The CSV row was hand-corrected; the next full `build_graph.py` regen would re-introduce the wrong row, which is why the gate exists. Fix the introspector before any regen.
- **Source-walk over-collects on purpose.** Records every ichiran-package symbol the Lisp reader sees in a defining form, including locals. The Python pass filters callees against `symbols.csv` membership; non-top-level names land in the "dropped" counter on rebuild.
- **Source-walk doesn't see macro expansions.** If a DAO reference only appears inside a macro that expands into a `select-dao` call, neither pass catches it. Most ichiran macros are DSL-style data registrars, so impact is minor in practice.
- **`ichiran-repl.sh` HELPERS heredoc is single-quoted bash.** Apostrophes inside it terminate the heredoc.
- **The naming.rs CSV-path test uses `../reverse/scripts/symbols.csv`** (relative to `CARGO_MANIFEST_DIR` = `kaniran-core/`). If layout moves, update.
- **Introspector line numbers can drift from the checked-in `*.lisp` files.** Grep upstream `.lisp` directly when writing new doc-comments.
- **`query.py mark` requires lowercase FQNs.** CSV stores lowercase even though Lisp symbols print uppercase.
- **`query.py next` only counts `ported` callees.** A symbol whose every callee is `skip` will NOT appear in `next`. Scan `PORT_PLAN.md` directly for what's actually walkable in plan order after a wave of skips.
- **Env-var tests in `conn::get_ichiran_connection_env`** mutate process env vars and must serialize on a static `Mutex`. New tests need to use the `with_env` helper.
- **`load-settings` no longer fires implicitly on file-load.** Library contract is "caller constructs `KaniranContext::from_env()` before invoking any DB-touching API."
- **`KaniranContext` ownership.** Each instance owns its `PgPool` and (eventually) its caches. `Clone` is derived (cheap — `PgPool` is internally `Arc<Pool>`-shaped).
- **Function name vs file stem for trailing `*`.** The file stem for `parse-number*` is `parse_number_star_.rs` (trailing `_` per `kani::naming` collision-prevention rule). The `pub fn` name inside must match: `pub fn parse_number_star_(...)`. The audit catches a mismatch — the symptom is `no `pub fn parse_number_star_` (found: ['parse_number_star'])`.
