# CLAUDE.md — kaniran

## What this repo is

A **Rust port of [ichiran](https://github.com/tshatrov/ichiran)** (Japanese morphology / segmentation / romanization, originally Common Lisp + PostgreSQL). kaniran is the product and lives in the workspace crates (`kaniran-core`, `kaniran-cli`, `kaniran-audit`). The original Lisp is **not** in this repo — it lives on the remote ichiran host (see below) and upstream on GitHub; what's tracked here is the introspected symbol metadata under `bookkeeping/reverse/`.

`README.md` is kaniran's own (install + CLI usage). Licensed MIT (`LICENSE`); workspace version `0.1.0`.

## How it was built, and where it's going

kaniran started as a **strict, behavior-preserving transliteration**: replicate ichiran exactly within Rust constraints — same shapes, same outputs, no improvements — verified function-by-function against fixtures captured from the running original. That was deliberate. It pins correctness before anything else, and "mirror exactly, no improvements" was largely a guardrail to keep the port honest while the foundation was laid.

That foundation now exists. From here kaniran is meant to grow **beyond** ichiran — performance, ergonomics, features — not stay a frozen mirror. So:

- **Existing ported code stays faithful.** Don't "improve" a transliterated function in place without a reason — its captured fixtures assert ichiran-equivalence, and silent drift there is a bug. Coding/naming conventions for that code are in `bookkeeping/CONVENTIONS.md`.
- **New work is not bound by "no improvements."** Deliberate divergence from ichiran is allowed and expected when it serves kaniran's goals. Make it intentional and documented, not accidental.

When unsure whether something is "faithful-port" territory or "next-level kaniran" territory, ask.

## Repository layout

The shipped product is the Rust workspace; everything used to plan and verify the port lives under `bookkeeping/` and is not part of the release.

```
kaniran-core/            # The Rust port (the library). Workspace member.
  data/                  # CSVs vendored + embedded via include_str! (romaji-map, kwpos, conj)
  src/
    lib.rs               # declares the per-package modules (characters, core, dict, conn,
                         #   custom, kanji, numbers, maintenance)
                         # Rust-only types/values (no Lisp counterpart) take a kani_/Kani prefix.
  Cargo.toml
kaniran-cli/             # Port of ichiran/cli — the `kaniran-cli` binary.
kaniran-audit/           # Internal bulk fixture-replay audit runners (one bin per fn). Ignorable.

bookkeeping/             # Planning + fixture tooling. Not shipped; not needed to build/use kaniran.
  CONVENTIONS.md         # Rust port coding/naming conventions.
  ichiran-repl.sh        # Helper to REPL into the remote ichiran host.
  reverse/
    *.lisp/              # auto-generated md files for upstream ichiran symbols, by kind:
                         #   <name>.md / _struct / _class / _dao / _global / _type / _condition
    index.md             # generated table of contents
    scripts/
      introspect.lisp    # SBCL introspector (6-kind capture); run on the ichiran host.
      run-remote.sh      # scp+ssh+rsync wrapper. NB: rsync --delete --exclude='scripts/'
                         #   wipes any non-introspector md (e.g. hand-written ones).
      build_graph.py     # parses all 6 md kinds into symbols.csv + edges.csv.
      query.py           # graph queries: leaves, plan, deps, dependents, mark, stats, ...
      symbols.csv        # one row per symbol (sorted by fqn for diff stability).
      edges.csv          # directed dependency edges (resolved=0 => external/builtin).
      slot_types.csv     # hand-curated STRUCT.SLOT -> TARGET edges (introspector blindspot).
      PORT_PLAN.md       # canonical topologically-sorted port order (commit this).
      divergences.md     # audit-signatures output (commit when it changes).
      README.md          # detailed usage for everything in scripts/.
  ichiran-extractor/     # Bulk fixture-capture pipeline (FastAPI + pooled SBCL workers on .103).
                         #   The :ichi-trace package lives in
                         #   bookkeeping/ichiran-extractor/trace_capture.lisp.
```

(Filename `PORT_PLAN.md` and the CSV/CLI status string `ported` are the original spellings, kept to avoid a separate rename sweep.)

## Port methodology — how the faithful foundation is built and verified

**Leaf-up porting with trace-driven golden tests.**

1. Treat the call graph (extracted from the md files) as a DAG and topologically sort it (`query.py plan`).
2. Port the leaves first, then the next layer, and so on. Mutually-recursive components are ported as a unit (the planner identifies them via Tarjan's strongly-connected-components pass).
3. For each Lisp function, capture **real (args, result) fixtures** by running the original ichiran with `sb-int:encapsulate` hooks during its existing test suite + a Japanese corpus driver. The Rust port replays those fixtures as `#[test]`s — equivalence is verified, not asserted.

`bookkeeping/reverse/scripts/PORT_PLAN.md` is the agreed sequence. As waves are completed, run `query.py mark fqn1 fqn2 ... --status ported` and regenerate the plan to see what's now unblocked (`query.py next`).

## Key facts about the ichiran source

(These describe upstream ichiran, which lives on the remote host / GitHub, not in this repo.)

- **944 symbols** across 16 source files (10 packages). Breakdown: 689 fn, 126 global, 38 gf, 36 macro, 28 class, 14 dao, 11 struct, 1 type, 1 condition.
- **923 symbols / 921 waves** in `PORT_PLAN.md` (excludes ichiran/maintenance and ichiran/test).
- **2 real strongly-connected components** (4 symbols total) — must port as a unit. Cycles are tiny.
- **36 macro leaves** — most are DSL definers in `dict-grammar.lisp` / `dict-split.lisp` / `dict-counters.lisp`. They don't translate as macros; the DATA they register lives in `_global.md` files (e.g. `*suffix-list*` is the populated registry for `def-simple-suffix` callsites).
- **ichiran/dict** is the bulk of the codebase. The hard part.
- **No global requires the database to initialize.** `*reading-cache*` is the only global that interacts with Postgres, and it does so lazily per-key inside `get-readings-cache`. The cache itself starts empty.
- **No macros expand to defclass/defstruct.** Everything dynamic in the codebase is data registered into existing globals.
- **About 78 leaves classify as TRIVIAL** (≤8 lines, no DB, no regex, no recursion) and can be hand-ported without fixtures. Of those, 13 collapse into 2 Rust files (defstruct families).

## Remote ichiran host

A working ichiran install runs on a remote host (`<user>@<ichiran-host>`, ichiran checked out under `<ichiran-dir>`) with PostgreSQL configured — the actual address/path live in local notes, not in this repo. Used to:

- Re-run `introspect.lisp` when upstream changes (via `run-remote.sh`).
- Drive `sb-int:encapsulate` capture for fixture generation via `bookkeeping/ichiran-extractor/`.

Connect over `ssh`. SBCL is at `/usr/bin/sbcl`, version 2.2.9. Quicklisp is set up at `~/quicklisp`. ichiran's deps include `jsown` (handy for JSONL output), `lisp-unit` (test framework), `postmodern` (Postgres).

The driver entrypoint is `(ichiran/test:run-all-tests)`.

## Working conventions

- **Rust port coding/naming conventions live in [`bookkeeping/CONVENTIONS.md`](./bookkeeping/CONVENTIONS.md).** Read it before adding or editing ported files — it covers file layout, doc-comment requirements, the rules for translating Lisp shapes (multi-value returns, `&key` keywords, in-place mutation, tagged cons cells) into Rust APIs, the testing policy (logic not data), and the workflow steps below in concrete form. The FQN→Rust-path substitution rules are spelled out in CONVENTIONS.md §3 (a Python copy lives in `bookkeeping/ichiran-extractor/fetch_extractor.py`'s `fqn_to_path`).
- **The only Lisp in this repo is kaniran's own extraction tooling** under `bookkeeping/ichiran-extractor/` (and `introspect.lisp`). Upstream ichiran source is not checked in — read it on the remote host or GitHub.
- **`PORT_PLAN.md` is the source of truth for port order.** Regenerate (don't hand-edit) via `query.py plan --out bookkeeping/reverse/scripts/PORT_PLAN.md`. It's deterministic across runs (Tarjan + sorted set iteration); re-running on the same CSVs produces a byte-identical file.
- **Mark progress in `symbols.csv`'s `status` column** (`pending` → `ported`, `wip`, `skip`, etc.). `query.py mark` does this round-trip-safely. Pair `--status skip` (or any off-the-books status) with `--reason "..."` — the reason lands in the CSV's `reason` column and surfaces in the `PORT_PLAN.md` badge.
- **Track the fixture-replay workstream in `extracted` / `audited` columns** alongside the main status:
  - `query.py extracted <fqn>... --corpus <tag>` tags fns whose parquet fixtures have been captured. Use `<short_desc>_<yyyy_mm_dd>` for canonical diverse-corpus runs (e.g. `splits_2026_05_09`, `segmenter_2026_05_09`); use a descriptive label for one-offs (`init-suffixes`, `recapture`, etc.).
  - `query.py audited <fqn>... --pass P --total T` tags pass-rate from a `audit_fixtures` run against the captured parquet. The command refuses if `extracted` is empty (audit must follow extraction). Convention: leave `audited` blank for one-off probes and synthetic corpora — it's reserved for canonical diverse-corpus pass-rates.
  - Populator ports verified via cache-cardinality cross-check (`cache_inspect`) leave both columns empty — they're a different verification path.
  - Both columns render as extra `*[extracted: …]*` / `*[audited P/T]*` badges in `PORT_PLAN.md` next to the main status badge.
- **`build_graph.py` preserves all four state columns** (`status`, `reason`, `extracted`, `audited`) across rebuilds — none of them are introspector output. A symbol that disappears between runs loses its state (no row to hold it); a new symbol starts blank.
- **Slot-type edges are hand-curated in `bookkeeping/reverse/scripts/slot_types.csv`** to repair the introspector's blindspot for `t`-typed defstruct/defclass slots. Each row asserts `STRUCT.SLOT` holds `TARGET`, and emits a `STRUCT → TARGET` edge with origin `slot-type`. `build_graph.py` validates that the struct, slot, and target all exist and aborts on any typo. Add a row when you discover an unported type held in a slot — without it, the topo sort can rank the holder before the held type.
- **Use `query.py` over hand-grepping the md files.** The dependency analysis is non-trivial (cycles, unresolved external refs) and the script handles it correctly.

## Tracer / sniffer

Built. Lives in `bookkeeping/ichiran-extractor/trace_capture.lisp` as the `:ichi-trace` Common Lisp package. Loaded by every SBCL worker the FastAPI pool starts.

API (all under `ichi-trace:`):
- `install fqn` / `install-many fqns` — wrap target functions with `sb-int:encapsulate` recorders.
- `clear` / `n-captures` / `captures` / `drain` — inspect / consume.
- `uninstall` / `uninstall-all` — clean removal (fully reversible; doesn't touch source or fasls).

Invariants the implementation respects:
- Re-entrance guard via `*in-recorder*` (only fires around the recorder's bookkeeping; inner installed callees ARE captured).
- Primitive-shape gate on args and result — non-readable shapes (closures, hash-tables, classes) get logged to `*skipped*` rather than recorded.
- Fully-qualified function names in JSONL (`ICHIRAN/CHARACTERS:MORA-LENGTH`).
- `*print-readably*` bound during prin1 so captured strings round-trip via the Rust replay parser.
- Characters tagged as `(:CHAR <codepoint>)` lists pre-print so `#\HIRAGANA_LETTER_HA` style names never appear in captures (no Unicode-name table needed on the Rust side).

Status: end-to-end run validated 2026-05-04 — full 400K-sentence extraction at ~735 sentences/sec, 100K captures/sec, 0 parse errors on replay.

## Common commands

```sh
# regenerate the dependency CSVs from the md files (gated — see build_graph.py header)
python3 bookkeeping/reverse/scripts/build_graph.py

# regenerate signatures.json without touching the gated CSVs
python3 bookkeeping/reverse/scripts/build_graph.py --signatures-only

# see what to port next
python3 bookkeeping/reverse/scripts/query.py leaves           # current leaves
python3 bookkeeping/reverse/scripts/query.py plan             # full topological order
python3 bookkeeping/reverse/scripts/query.py next             # unblocked by completed waves

# graph queries
python3 bookkeeping/reverse/scripts/query.py deps <fqn> [--deep]
python3 bookkeeping/reverse/scripts/query.py dependents <fqn> [--deep]

# mark progress (round-trip safe — just rewrites symbols.csv)
python3 bookkeeping/reverse/scripts/query.py mark <fqn>... --status ported

# tag fixture-replay workstream state (orthogonal to the main status)
python3 bookkeeping/reverse/scripts/query.py extracted <fqn>... --corpus tatoeba
python3 bookkeeping/reverse/scripts/query.py audited   <fqn>... --pass P --total T

# stats
python3 bookkeeping/reverse/scripts/query.py stats

# regenerate canonical plan after marking a wave
python3 bookkeeping/reverse/scripts/query.py plan --out bookkeeping/reverse/scripts/PORT_PLAN.md

# audit ported pub fns vs captured Lisp lambda lists — run after each port
# (always rewrites bookkeeping/reverse/scripts/divergences.md — commit if it changes)
python3 bookkeeping/reverse/scripts/query.py audit-signatures              # full sweep + rewrite
python3 bookkeeping/reverse/scripts/query.py audit-signatures --only <pkg> # scope STDOUT only
python3 bookkeeping/reverse/scripts/query.py audit-signatures --no-write   # don't touch the file
```

`audit-signatures` is part of the **port-completion checklist** (CONVENTIONS §7) alongside `cargo check` and `cargo test`. It cross-references each ported `pub fn` against the Lisp lambda list captured in `signatures.json` and flags arity drift, dropped keywords, missing pub fns, and extra public functions sharing a port file (the failure mode that produced the original `_with` split).

The committed artifact is **`bookkeeping/reverse/scripts/divergences.md`** — sorted by FQN, deterministic across runs, designed to diff cleanly. After every port: `git diff bookkeeping/reverse/scripts/divergences.md` is the review surface. New entries are either intentional (cite CONVENTIONS §4.4/§4.6/etc. and commit) or port bugs (fix and re-run until they disappear).

## Things you might think are true but aren't

- ❌ "kaniran must mirror ichiran forever." (No — see *How it was built, and where it's going*. Faithful mirroring was the porting discipline that pins correctness; existing ported code stays faithful, but kaniran is meant to evolve past ichiran with intentional, documented changes.)
- ❌ "There are N cycles in the graph." (Tarjan finds the real strongly-connected components — current count and member size are in the `# Port plan — …` header at the top of `PORT_PLAN.md`. Most are tiny; a handful are real type-recursion clusters surfaced by `slot_types.csv`.)
- ❌ "Macros are unportable." (Most of the 36 macro leaves dissolve into Rust data tables or idioms; only ~6 need real thought.)
- ❌ "build_graph.py resets status / reason on every run." (It used to. Now it preserves all four state columns — `status`, `reason`, `extracted`, `audited` — across rebuilds. A symbol that disappears between runs loses its state; new symbols start with the parse_md defaults.)
- ❌ "Plan ordering shifts between runs." (Fixed earlier — Tarjan uses sorted set iteration; output is byte-identical.)
- ❌ "kaniran-core is just a bootstrap stub." (It's substantially populated — many waves ported — alongside a working CLI (`kaniran-cli`) and audit harness (`kaniran-audit`).)
- ❌ "Globals get loaded from the database at startup." (Verified false — every defparameter/defvar/defconstant initializer is in-memory only. Only `*reading-cache*` interacts with Postgres, and it does so lazily inside the function `get-readings-cache`.)
- ❌ "We need to build a separate `trace_capture.lisp`." (It already exists at `bookkeeping/ichiran-extractor/trace_capture.lisp`.)
- ❌ "`bookkeeping/reverse/` only covers functions." (It covers 6 kinds — fn/macro/gf + struct/class/dao/global, plus 1 hand-written deftype and 1 define-condition.)
- ❌ "The introspector knows what each defstruct slot holds." (Slot types are `t` in CL; the introspector reports them as such. `bookkeeping/reverse/scripts/slot_types.csv` is the hand-curated repair: each row asserts `STRUCT.SLOT → TARGET` and `build_graph.py` emits the corresponding `slot-type` edge so the topo sort treats holders as depending on what they hold. Add a row when you discover a missing one.)
