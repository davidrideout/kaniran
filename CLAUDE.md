# CLAUDE.md — kaniran

## What this repo is

A **Rust fork of [ichiran](https://github.com/tshatrov/ichiran)** (Japanese morphology / segmentation / romanization, Common Lisp + PostgreSQL). The original Lisp source is checked in at the repo root for reference; the port itself does not live there yet.

The upstream `README.md` is ichiran's original README. Don't treat it as documentation for this fork.

## Where the port work happens

Everything related to the port is under `reverse/`.

```
reverse/
  *.lisp/                # auto-generated md files for upstream ichiran symbols.
                         # Suffixes carry the kind:
                         #   <name>.md           = function / macro / generic
                         #   <name>_struct.md    = defstruct
                         #   <name>_class.md     = defclass (plain CLOS)
                         #   <name>_dao.md       = defclass :metaclass dao-class
                         #   <name>_global.md    = defparameter / defvar / defconstant
                         #   <name>_type.md      = deftype
                         #   <name>_condition.md = define-condition
  index.md               # generated table of contents
  scripts/
    introspect.lisp      # SBCL introspector (6-kind capture); run on the ichiran host.
    run-remote.sh        # scp+ssh+rsync wrapper. NB: rsync --delete --exclude='scripts/'
                         # wipes any non-introspector md (e.g. hand-written ones).
    build_graph.py       # parses all 6 md kinds into symbols.csv + edges.csv.
    query.py             # graph queries: leaves, plan, deps, dependents, mark, stats, ...
    symbols.csv          # one row per symbol (sorted by fqn for diff stability).
    edges.csv            # directed dependency edges (resolved=0 => external/builtin).
    PORT_PLAN.md         # canonical topologically-sorted port order (commit this).
    HANDOFF.md           # current state + open decisions + next moves.
    README.md            # detailed usage for everything in scripts/.

crates/kaniran-core/     # The Rust port crate. Workspace member at repo root.
  src/
    lib.rs               # declares pub mod kani; (and future per-package port modules)
    kani.rs              # kani:: namespace — kaniran infra, NOT ports
    kani/
      fixture.rs         # JSONL fixture envelope + lexpr-based replay parsing
      naming.rs          # Lisp FQN -> Rust path (single source of truth, exhaustively tested)
  Cargo.toml

ichiran-repl.sh          # local->remote SBCL wrapper. Pre-loads ichiran on .103 plus:
                         #   :ichi package — REPL helpers (score, words, full-trace, etc.)
                         #   :ichi-trace package — sb-int:encapsulate-based fixture recorder
                         # The tracer lives HERE, not in a separate trace_capture.lisp.
```

## Port methodology

**Leaf-up port with trace-driven golden tests.**

1. Treat the call graph (extracted from md files) as a DAG and topologically sort it (`query.py plan`).
2. Port the leaves first, then the next layer, and so on. Mutually-recursive components are ported as a unit (the planner identifies them via Tarjan's SCC).
3. For each Lisp function being ported, capture **real (args, result) fixtures** by running the original ichiran with `sb-int:encapsulate` hooks during its existing test suite + a Japanese corpus driver. The Rust port replays those fixtures as `#[test]`s — equivalence is verified, not asserted.

`reverse/scripts/PORT_PLAN.md` is the agreed sequence. As waves are completed, run `query.py mark fqn1 fqn2 ... --status ported` and regenerate the plan to see what's now unblocked (`query.py next`).

## Key facts about the codebase

- **944 symbols** across 16 source files (10 packages). Breakdown: 689 fn, 126 global, 38 gf, 36 macro, 28 class, 14 dao, 11 struct, 1 type, 1 condition.
- **923 symbols / 921 waves** in PORT_PLAN.md (excludes ichiran/maintenance and ichiran/test).
- **2 real strongly-connected components** (4 symbols total) — must port as a unit. Cycles are tiny.
- **36 macro leaves** — most are DSL definers in `dict-grammar.lisp` / `dict-split.lisp` / `dict-counters.lisp`. They don't translate as macros; the DATA they register lives in `_global.md` files (e.g. `*suffix-list*` is the populated registry for `def-simple-suffix` callsites).
- **Ichiran/dict** is the bulk of the codebase. The hard part.
- **No global requires the database to initialize.** `*reading-cache*` is the only global that interacts with Postgres, and it does so lazily per-key inside `get-readings-cache`. The cache itself starts empty.
- **No macros expand to defclass/defstruct.** Everything dynamic in the codebase is data registered into existing globals.
- **About 78 leaves classify as TRIVIAL** (≤8 lines, no DB, no regex, no recursion) and can be hand-ported without fixtures. Of those, 13 collapse into 2 Rust files (defstruct families).

## Remote ichiran host

A working ichiran install runs on **`david@192.168.1.103:/home/david/storage/ichiran`** with PostgreSQL configured. Used to:

- Re-run `introspect.lisp` when upstream changes (via `run-remote.sh`).
- Drive `sb-int:encapsulate` capture for fixture generation (planned).

Connect with `ssh david@192.168.1.103`. SBCL is at `/usr/bin/sbcl`, version 2.2.9. Quicklisp is set up at `~/quicklisp`. ichiran's deps include `jsown` (handy for JSONL output), `lisp-unit` (test framework), `postmodern` (Postgres).

The driver entrypoint is `(ichiran/test:run-all-tests)`.

## Working conventions

- **Rust port coding/naming conventions live in [`CONVENTIONS.md`](./CONVENTIONS.md).** Read it before adding or editing port files — it covers file layout, doc-comment requirements, the rules for translating Lisp shapes (multi-value returns, `&key` keywords, in-place mutation, tagged cons cells) into idiomatic Rust APIs, the testing policy (logic not data), and the workflow steps below in concrete form. The single source of truth for FQN→Rust path translation is the module-doc on [`kaniran-core/src/kani/naming.rs`](./kaniran-core/src/kani/naming.rs); CONVENTIONS.md and HANDOFF.md both defer to it.
- **Don't edit upstream `*.lisp` files at the repo root.** They're checked in for reference / introspection input. Treat as read-only.
- **`PORT_PLAN.md` is the source of truth for porting order.** Regenerate (don't hand-edit) via `query.py plan --out reverse/scripts/PORT_PLAN.md`. It's deterministic across runs (Tarjan + sorted set iteration); re-running on the same CSVs produces a byte-identical file.
- **Mark progress in `symbols.csv`'s `status` column** (`pending` → `ported`, `wip`, `skip`, etc.). `query.py mark` does this round-trip-safely. Pair `--status skip` (or any off-the-books status) with `--reason "..."` — the reason lands in the CSV's `reason` column and surfaces in the PORT_PLAN.md badge.
- **Re-running `build_graph.py` resets `status` to `pending` and `reason` to empty for every row** (it overwrites the CSV). Commit before regenerating, or back up.
- **Use `query.py` over hand-grepping the md files.** The dependency analysis is non-trivial (cycles, unresolved external refs) and the script handles it correctly.

## Tracer / sniffer

Built. Lives in `ichiran-repl.sh` as the `:ichi-trace` Common Lisp package — NOT in a separate `trace_capture.lisp`. Embedded in the wrapper because every wrapper invocation already loads ichiran on `.103`, so the tracer comes pre-loaded.

API (all under `ichi-trace:`):
- `install fqn` / `install-many fqns` — wrap target functions with `sb-int:encapsulate` recorders.
- `clear` / `n-captures` / `captures` — inspect.
- `dump-jsonl path` / `dump-per-symbol dir` — write fixtures.
- `uninstall` / `uninstall-all` — clean removal (fully reversible; doesn't touch source or fasls).

Invariants the implementation respects:
- Re-entrance guard via `*in-recorder*` (prevents loops when traced fns call other traced fns).
- Primitive-shape gate on args and result — non-readable shapes (closures, hash-tables, classes) get logged to `*skipped*` rather than recorded.
- Fully-qualified function names in JSONL (`ICHIRAN/CHARACTERS:MORA-LENGTH`).
- `*print-readably*` bound during prin1 so captured strings round-trip via Rust's `lexpr::from_str`.

Status: proven end-to-end via probes. Has NOT yet been run against `(ichiran/test:run-all-tests)` for a real fixture sweep — that's the obvious next step when actual port work begins.

## Common commands

```sh
# regenerate the dependency CSVs from the md files (gated — see build_graph.py header)
python3 reverse/scripts/build_graph.py

# regenerate signatures.json without touching the gated CSVs
python3 reverse/scripts/build_graph.py --signatures-only

# see what to port next
python3 reverse/scripts/query.py leaves               # current leaves
python3 reverse/scripts/query.py plan                 # full topological order
python3 reverse/scripts/query.py next                 # unblocked by completed waves

# graph queries
python3 reverse/scripts/query.py deps <fqn> [--deep]
python3 reverse/scripts/query.py dependents <fqn> [--deep]

# mark progress (round-trip safe — just rewrites symbols.csv)
python3 reverse/scripts/query.py mark <fqn>... --status ported

# stats
python3 reverse/scripts/query.py stats

# regenerate canonical plan after marking a wave
python3 reverse/scripts/query.py plan --out reverse/scripts/PORT_PLAN.md

# audit ported pub fns vs captured Lisp lambda lists — run after each port
# (always rewrites reverse/scripts/divergences.md — commit if it changes)
python3 reverse/scripts/query.py audit-signatures                # full sweep + rewrite
python3 reverse/scripts/query.py audit-signatures --only <pkg>   # scope STDOUT only
python3 reverse/scripts/query.py audit-signatures --no-write     # don't touch the file
```

`audit-signatures` is part of the **port-completion checklist** (CONVENTIONS §7) alongside `cargo check` and `cargo test`. It cross-references each ported `pub fn` against the Lisp lambda list captured in `signatures.json` and flags arity drift, dropped keywords, missing pub fns, and extra public functions sharing a port file (the failure mode that produced the original `_with` split).

The committed artifact is **`reverse/scripts/divergences.md`** — sorted by FQN, deterministic across runs, designed to diff cleanly. After every port: `git diff reverse/scripts/divergences.md` is the review surface. New entries are either intentional (cite CONVENTIONS §4.4/§4.6/etc. and commit) or port bugs (fix and re-run until they disappear).

## Things you might think are true but aren't

- ❌ "There are 102 cycles in the graph." (Naive layer-walk artifact; Tarjan finds **2** real SCCs covering 4 symbols.)
- ❌ "Macros are unportable." (Most of the 36 macro leaves dissolve into Rust data tables or idioms; only ~6 need real thought.)
- ❌ "build_graph.py preserves status or reason across runs." (It rewrites the file, resetting both. Commit first or re-mark via `query.py mark --reason ...`.)
- ❌ "Plan ordering shifts between runs." (Fixed earlier — Tarjan uses sorted set iteration; output is byte-identical.)
- ❌ "The Rust crate is TBD." (It exists at `crates/kaniran-core/` with a working naming convention and fixture-replay infra. Bootstrapped, not populated.)
- ❌ "Globals get loaded from the database at startup." (Verified false — every defparameter/defvar/defconstant initializer is in-memory only. Only `*reading-cache*` interacts with Postgres, and it does so lazily inside the function `get-readings-cache`.)
- ❌ "We need to build a separate `trace_capture.lisp`." (The tracer is already in `ichiran-repl.sh` as the `:ichi-trace` package. Don't duplicate it.)
- ❌ "`reverse/` only covers functions." (Now covers 6 kinds — fn/macro/gf + struct/class/dao/global, plus 1 hand-written deftype and 1 define-condition.)
