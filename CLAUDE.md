# CLAUDE.md — kaniran

## What this repo is

A **Rust fork of [ichiran](https://github.com/tshatrov/ichiran)** (Japanese morphology / segmentation / romanization, Common Lisp + PostgreSQL). The original Lisp source is checked in at the repo root for reference; the port itself does not live there yet.

The upstream `README.md` is ichiran's original README. Don't treat it as documentation for this fork.

## Where the port work happens

Everything related to the port is under `reverse/`.

```
reverse/
  *.lisp/                # auto-generated md files: one per defun/defmacro/defgeneric in upstream ichiran.
  index.md               # generated table of contents
  scripts/
    introspect.lisp      # original SBCL introspector (produces the md files); run on the ichiran host.
    run-remote.sh        # scp+ssh wrapper for introspect.lisp against the remote host.
    build_graph.py       # parses md files into symbols.csv + edges.csv.
    query.py             # graph queries: leaves, plan, deps, dependents, mark, stats, ...
    symbols.csv          # one row per symbol (sorted by fqn for diff stability).
    edges.csv            # directed call edges (resolved=0 => external/builtin, ignored by traversal).
    PORT_PLAN.md         # canonical topologically-sorted port order (commit this).
    README.md            # detailed usage for everything in scripts/.
```

The Rust crate(s) will live somewhere outside `reverse/` (TBD — not yet started). `reverse/` itself is purely the analysis + planning layer.

## Port methodology

**Leaf-up port with trace-driven golden tests.**

1. Treat the call graph (extracted from md files) as a DAG and topologically sort it (`query.py plan`).
2. Port the leaves first, then the next layer, and so on. Mutually-recursive components are ported as a unit (the planner identifies them via Tarjan's SCC).
3. For each Lisp function being ported, capture **real (args, result) fixtures** by running the original ichiran with `sb-int:encapsulate` hooks during its existing test suite + a Japanese corpus driver. The Rust port replays those fixtures as `#[test]`s — equivalence is verified, not asserted.

`reverse/scripts/PORT_PLAN.md` is the agreed sequence. As waves are completed, run `query.py mark fqn1 fqn2 ... --status ported` and regenerate the plan to see what's now unblocked (`query.py next`).

## Key facts about the codebase

- **763 symbols** across 16 source files (10 packages).
- **271 leaves** — functions/macros/generics with no internal callees. Bulk of immediately-portable work.
- **2 real strongly-connected components** (4 symbols total) — must port as a unit. Despite earlier intuition, cycles are tiny.
- **36 macro leaves** — most are DSL definers in `dict-grammar.lisp` / `dict-split.lisp` / `dict-counters.lisp` that register data into global tables. They don't translate as macros — they collapse to literal Rust data tables (≈600 callsites total to transcribe, automatable from `macroexpand-1` dumps).
- **Ichiran/dict** alone is 585 / 763 symbols — 77% of the codebase. The hard part.
- **Pure leaves** (no DB) live in `characters.lisp` (22) and `numbers.lisp` (2). Best starting point — fixtures capture without needing the Postgres setup.

## Remote ichiran host

A working ichiran install runs on **`david@192.168.1.103:/home/david/storage/ichiran`** with PostgreSQL configured. Used to:

- Re-run `introspect.lisp` when upstream changes (via `run-remote.sh`).
- Drive `sb-int:encapsulate` capture for fixture generation (planned).

Connect with `ssh david@192.168.1.103`. SBCL is at `/usr/bin/sbcl`, version 2.2.9. Quicklisp is set up at `~/quicklisp`. ichiran's deps include `jsown` (handy for JSONL output), `lisp-unit` (test framework), `postmodern` (Postgres).

The driver entrypoint is `(ichiran/test:run-all-tests)`.

## Working conventions

- **Don't edit upstream `*.lisp` files at the repo root.** They're checked in for reference / introspection input. Treat as read-only.
- **`PORT_PLAN.md` is the source of truth for porting order.** Regenerate (don't hand-edit) via `query.py plan --out reverse/scripts/PORT_PLAN.md`. It's deterministic across runs (Tarjan + sorted set iteration); re-running on the same CSVs produces a byte-identical file.
- **Mark progress in `symbols.csv`'s `status` column** (`pending` → `ported`, `wip`, `skip`, etc.). `query.py mark` does this round-trip-safely.
- **Re-running `build_graph.py` resets `status` to `pending` for every row** (it overwrites the CSV). Commit before regenerating, or back up.
- **Use `query.py` over hand-grepping the md files.** The dependency analysis is non-trivial (cycles, unresolved external refs) and the script handles it correctly.

## Tracer / sniffer (planned, not yet built)

A future `reverse/scripts/trace_capture.lisp` will:

1. Install `sb-int:encapsulate` recorders on a target list of FQNs (probably the current `query.py leaves`).
2. Run `(ichiran/test:run-all-tests)` + a Tatoeba-style corpus driver.
3. Dedupe `(fn, prin1-args)` pairs, dump JSONL fixtures to `reverse/scripts/fixtures/<package>/<symbol>.jsonl`.
4. Self-test (Layer 1) at install time + replay-equivalence (Layer 2) after capture, before declaring fixtures trustworthy.

The probe in `/tmp/probe.lisp` (run during design) confirmed: `sb-int:encapsulate` works on both `defun` and `defgeneric`; `prin1-to-string` round-trips faithfully for the leaf types we care about (chars, strings, lists, conses, NIL, keywords, integers); `jsown` is already loaded; `ichiran/test:run-all-tests` is a viable driver.

The sniffer **does not modify ichiran source or its compiled fasls** — it's a runtime function-cell swap, fully reversible within the SBCL image.

## Common commands

```sh
# regenerate the dependency CSVs from the md files
python3 reverse/scripts/build_graph.py

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
```

## Things you might think are true but aren't

- ❌ "There are 102 cycles in the graph." (That number came from a naive layer-walk; Tarjan finds **2** real SCCs.)
- ❌ "Macros are unportable." (Most of the 36 macro leaves dissolve into Rust data tables or idioms; only ~6 need real thought.)
- ❌ "build_graph.py preserves status across runs." (It rewrites the file. Commit first.)
- ❌ "Plan ordering shifts between runs." (Fixed earlier — Tarjan now uses sorted set iteration; output is byte-identical across `PYTHONHASHSEED` values.)
