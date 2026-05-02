# Handoff — Rust port of ichiran

Read `CLAUDE.md` first for orientation. This doc is the current snapshot of where the work is and what is open.

---

## tl;dr — current state

| Layer | Status |
|---|---|
| Original Lisp source | Checked in at repo root, untouched. |
| **Symbol / dependency extraction** (md files) | Done. **945 md files** under `reverse/<file>.lisp/` covering every behavior-bearing form (functions, macros, generics, structs, plain CLOS, DAO classes, globals, plus 1 hand-written deftype and 1 define-condition). |
| **Graph CSVs** (symbols.csv, edges.csv) | Done. **944 symbols** (689 fn, 126 global, 38 gf, 36 macro, 28 class, 14 dao, 11 struct, 1 type, 1 condition) + 2698 edges. `build_graph.py` parses all 6 md kinds. |
| **PORT_PLAN.md** | Done. **923 symbols across 921 waves**, deterministic. Globals come first (leaves), structs/classes next, functions stack into the deeper waves. 2 mutual-recursion groups of 2 symbols each are the only SCCs. |
| **Tracer mechanism** (`:ichi-trace` package in `ichiran-repl.sh`) | **Built and proven via probes against `.103`.** End-to-end: install hooks → exercise functions → JSONL fixtures land on `.103`. Has NOT yet been run for real on `(ichiran/test:run-all-tests)` or a corpus. |
| **Rust crate `kaniran-core`** | Bootstrapped at `crates/kaniran-core/`. Workspace member at repo root. 16 tests pass. Has fixture-replay scaffolding (`kani::fixture` — JSONL + lexpr) and naming convention (`kani::naming` — exhaustively covers all 944 symbols, no collisions). |
| **Actual ports of ichiran functions** | **Zero.** No leaf has been ported yet. |

---

## What's in place — file map

```
crates/kaniran-core/
├── Cargo.toml                       # workspace member
└── src/
    ├── lib.rs                       # declares pub mod kani;
    ├── kani.rs                      # kani:: namespace (kaniran infra, NOT ports)
    └── kani/
        ├── fixture.rs               # JSONL envelope + lexpr-based replay
        └── naming.rs                # FQN → Rust path (single source of truth)

reverse/                             # static analysis output (rsynced from .103)
├── index.md
├── <each>.lisp/                     # 16 of these, one per source file
│   ├── <fn>.md                      # 763 of these
│   ├── <name>_struct.md             # 11
│   ├── <name>_class.md              # 28
│   ├── <name>_dao.md                # 14
│   ├── <name>_global.md             # 126
│   ├── <name>_type.md               # 1   (hand-written: char-class)
│   └── <name>_condition.md          # 1   (hand-written: not-a-number)
└── scripts/
    ├── introspect.lisp              # SBCL pass — 6 kinds; runs on .103
    ├── run-remote.sh                # scp + ssh + rsync wrapper
    ├── build_graph.py               # md → CSVs (handles all 6 kinds)
    ├── query.py                     # leaves / next / plan / mark / stats
    ├── symbols.csv, edges.csv       # generated
    ├── PORT_PLAN.md                 # generated
    └── HANDOFF.md                   # you are here

ichiran-repl.sh                      # local→remote SBCL wrapper
                                     # contains :ichi (REPL helpers) + :ichi-trace (tracer)

CLAUDE.md                            # project orientation, conventions, host info
```

---

## What still has to happen — three questions and a wave

### Decisions still open (none made yet, surfaced before any DB-touching work)

1. **DB layer.** sqlx+tokio (async), diesel (sync), sea-orm (async), or hand-rolled. Affects every `ichiran/dict::*` DAO port.
2. **JMdict schema.** Share ichiran's Postgres schema (so a single DB serves both), or design a fresh one. Sharing is cheaper but couples the projects.
3. **Scope.** Full port (~944 symbols) vs. romanize/segment public API only (~100 symbols, dramatically smaller).

These four were all in the original HANDOFF; one (where the Rust crate lives) is now answered. The other three are unchanged.

### Concrete near-term moves (any one is a clean start)

A. **Run the tracer for real.** `:ichi-trace` works in probes. The next step is invoking it with a real driver — `(ichiran/test:run-all-tests)` plus a corpus loop — and dumping per-symbol fixtures. No more wrapper changes needed; just a long heredoc that installs hooks on the leaf set, runs the driver, dumps `/tmp/fixtures/`, and scp's back. `query.py leaves` gives the install list.

B. **Hand-port the trivial wave.** ~78 leaves classify as TRIVIAL (≤8 lines, no DB, no regex, no recursion). Of those, 13 are auto-generated struct accessors that collapse into 2 Rust files (`KanaRepresentation` + `RmapItem`). About 65 actual hand-port files in this wave — bulk in `ichiran/characters` (11 leaves), `ichiran/dict` (48), and the struct families. Doesn't need fixtures; per-function unit tests with hand-picked inputs are enough.

C. **Stand up the fixture replay runner in Rust.** `kani::fixture` has the JSONL parser and lexpr decoder. What's missing is a generic test harness: `replay_fixture(fn_name, port_fn) -> Result<()>` that reads a JSONL file, parses each line, calls `port_fn` with the parsed args, and asserts the result lexpr-equals the captured value. Once this exists, every ported function gets a one-line test.

D. **Pick one of the three open decisions** and move forward. The DB layer choice unblocks the most subsequent work.

---

## Gotchas (real, found this session)

- **`run-remote.sh` rsyncs with `--delete --exclude='scripts/'`.** Anything under `reverse/` that the introspector did not produce gets wiped on next run. Two hand-written md files (`reverse/characters.lisp/char-class_type.md`, `reverse/numbers.lisp/not-a-number_condition.md`) are at risk. Either commit before re-running, or extend the introspector to cover deftype + define-condition (they're 1 each — small task).
- **`ichiran-repl.sh` HELPERS heredoc is single-quoted bash.** Apostrophes inside it (in comments or strings) terminate the heredoc and bash interprets the rest as commands. Use `does not` instead of `doesn't`. No automated check.
- **SBCL drops into the debugger on unhandled errors and consumes stdin** — wrap snippets sent via the wrapper in `handler-case`, or the script hangs. (Possible future hardening: add `--disable-debugger` to the sbcl invocation in `ichiran-repl.sh:222`. Not done because it's the user's tool.)
- **Build-graph rewrites symbols.csv on every run, resetting `status` to `pending`.** Commit the file before re-generating, or use `query.py mark` to re-mark progress.
- **PORT_PLAN.md ordering is byte-deterministic** across runs (Tarjan + sorted iteration was specifically fixed earlier). If it changes between runs without a CSV change, that's a regression.

## What's not a gotcha but feels like one

- **`*reading-cache*` is the only global that interacts with Postgres** — but it does so lazily, per-key, inside the function `get-readings-cache`. The global itself is just an empty hash table; nothing in it requires a DB connection to construct. Safe to port as `Lazy<Mutex<HashMap<...>>>::default()`. The DB call lives in the function port, not the global.
- **No global anywhere requires the database to initialize.** Verified by walking every defparameter/defvar/defconstant body and every `(setf *var* ...)` form across all 14 source files. Zero DB calls in any initializer.
- **No macros expand into `defclass`/`defstruct` forms.** The 11 + 42 = 53 data definitions found by grep is the complete set; nothing dynamic is hiding behind the DSL macros (`def-simple-suffix` etc.). Those macros only push entries into existing globals.

---

## How to resume — first three commands

```sh
# 1. Confirm graph still parses cleanly
python3 reverse/scripts/build_graph.py
# expect: symbols: 944 -> ...   by kind: fn=689, global=126, gf=38, macro=36, class=28, dao=14, struct=11, type=1, condition=1

# 2. Confirm Rust crate compiles + tests pass
cargo test
# expect: 16 passed

# 3. Confirm tracer wrapper still works against .103
./ichiran-repl.sh <<'LISP'
(ichi-trace:install (quote ichiran/characters:mora-length))
(ichiran/characters:mora-length "ねこ")
(format t "n=~a~%" (ichi-trace:n-captures))
(ichi-trace:uninstall-all)
LISP
# expect: install-many output, n=1, no errors
```

If all three succeed, the inherited infrastructure is healthy and the work is to start porting (move B above) or run captures (move A).

---

## Probe results worth keeping

The tracer design was validated on `.103` end-to-end this session:
- `sb-int:encapsulate` works on `defun` and `defgeneric`.
- `prin1` / `read` round-trip is faithful for primitive shapes (chars, strings including Japanese, NIL, T, keywords, integers, lists, vectors of primitives).
- `multiple-value-list` cleanly captures multi-value returns (e.g., `cl-ppcre:scan` returns 4 values).
- `sb-mop` walks classes and slot definitions reliably.
- `sb-kernel:find-defstruct-description` + `dd-*` accessors give complete struct introspection (slots, defaults, conc-name, custom constructors).
- Postmodern's DAO column metadata is accessible via `slot-value` on `direct-column-slot` instances using internal symbols `pomo::col-type`, `pomo::sql-name`.

These are not assumptions; they were exercised against the live ichiran image.
