`ichiran/characters` is fully ported (61/61). The next wave is a transition point: 19 symbols are unblocked across 7 packages, but most of them touch the database. **Before tackling `ichiran/dict`, the DB-layer decision needs to be made** — see "Open architectural decisions" in [`HANDOFF.md`](./HANDOFF.md).

**Starting state:** the commit that lands items 51–61 (built on top of `18c70ec`). `cargo test -p kaniran-core --lib` should report **49 passed** before you start. If it doesn't, stop and figure out why before doing anything else — the baseline is wrong.

## Read first

1. **[`CONVENTIONS.md`](../../CONVENTIONS.md)** — the canonical rules. Especially relevant for opening a new package directory (§1) and for any new convention that crops up during the transition.
2. **[`HANDOFF.md`](./HANDOFF.md)** — current state, frozen-literal ledger, the open DB-layer decision.
3. **[`CLAUDE.md`](../../CLAUDE.md)** — repo orientation, methodology.
4. The path-translation rules and tests in **[`kaniran-core/src/kani/naming.rs`](../../kaniran-core/src/kani/naming.rs)**. Note: the bare `ichiran:` package maps to `core/`, *not* `ichiran/` — feed any new FQN through `kani::naming::fqn_to_path`.

Auto-memory carries the project-specific feedback rules.

## What's unblocked

`python3 reverse/scripts/query.py next` reports 19 ready symbols. Grouped:

| Package | Count | Notes |
|---|---:|---|
| `ichiran` (bare) | 1 | `process-iteration-characters` — pure kana logic; opens a **new package directory** (`core/`). |
| `ichiran/custom` | 2 | `as-xml-simple`, `normalize-geo` — dict-custom utilities. |
| `ichiran/dict` | 12 | `find-word`, `find-substring-words`, `find-word-seq`, `find-word-with-pos`, `find-words-seqs`, `find-sticky-positions`, `add-reading`, `get-candidates`, `get-kanji-kana-old`, `process-hints`, `process-word-info`, `remove-hiragana-nokanji`, `sense-exists-p`. **Most touch Postgres.** |
| `ichiran/kanji` | 2 | `get-original-reading`, `get-reading-alternatives` — likely DB-touching. |
| `ichiran/maintenance` | 1 | `diff-content` — probably pure string utility. |

## Recommended path

### Phase 1 — non-DB leaves (clears 2-4 symbols, keeps momentum without committing to a DB layer)

1. **`ichiran:process-iteration-characters`** (`romanize.lisp:7`). Bare-`ichiran` package, so this opens `kaniran-core/src/core/` — first new package directory since the port started. Verify path with `kani::naming::fqn_to_path`. Add `pub mod core;` to `lib.rs` and a fresh `core/mod.rs`. Function itself processes iteration characters (`ゝゞヽヾ`) — kana-only.
2. **`ichiran/maintenance:diff-content`** (`ichiran.lisp:138`). Likely a string-diff helper. Read the Lisp body to confirm it's free of DB calls before writing the port. If it pulls from a DAO, defer.
3. **`ichiran/custom:as-xml-simple`** (`dict-custom.lisp:225`). Read the body first — name suggests XML emission, which would be pure, but `custom` DAOs are wired through this package. If it touches `defclass :metaclass dao-class` instances, defer to phase 3.

For each: read upstream callers via `grep -n '<name>' *.lisp` to confirm the API shape, write the port file, add the `pub mod` line, run `cargo test -p kaniran-core`, then `query.py mark`.

### Phase 2 — DB-layer decision (blocks the rest)

The `ichiran/dict` leaves dominate the next ~700 symbols and almost all of them are DAOs or DAO-querying functions. Three concrete options:

- **`sqlx`** (async, query macros, compile-time-checked SQL against a live DB). Strongest correctness guarantees, but requires Postgres at compile time and the macro plumbing complicates `kaniran-core`'s standalone-publish goal.
- **`diesel`** (sync, type-safe ORM, mature). Closest to what ichiran's Postmodern looks like API-wise. Sync simplifies the port; trades off integration with async Rust ecosystems.
- **`sea-orm`** (async, ActiveRecord-shaped, leans on `sqlx` underneath). Comfortable migration path from a Rails-shaped mental model; less common in published crates than `sqlx`.

Either form must be **feature-gated** so `kaniran-core` can publish standalone (per `project_kaniran_core_standalone_intent.md` in auto-memory).

This decision is the user's call — don't pick one autonomously. Surface the trade-offs and wait.

### Phase 3 — fixture harvest before `ichiran/dict`

With `ichiran/characters` complete and the next big package being `ichiran/dict` (679 symbols, lots of search/match logic), this is the natural moment to run the `:ichi-trace` tracer against `(ichiran/test:run-all-tests)` and harvest fixtures for the functions ported so far. The fixture-replay infra in `kani::fixture` is already wired. Doing it now means dict ports can be fixture-driven from day one.

Tracer is in `ichiran-repl.sh` as the `:ichi-trace` package — see [`CLAUDE.md`](../../CLAUDE.md) "Tracer / sniffer". Don't create a separate `trace_capture.lisp`.

### Phase 4 — `ichiran/dict` proper

Only after phase 2's decision lands. By then, fixture corpus from phase 3 covers most of `characters/`; replay-driven porting kicks in.

## Done means (per phase)

- Phase 1: 2–4 ports landed, `cargo test` green, `query.py mark` + `query.py plan --out PORT_PLAN.md` regenerated, HANDOFF updated.
- Phase 2: DB decision recorded in `HANDOFF.md` "Decisions still open" with the chosen crate and the migration sketch (sync/async, schema-share-vs-fresh, feature flag layout). Cargo.toml updated with the new optional dep behind a feature.
- Phase 3: tracer run successfully completed against `(ichiran/test:run-all-tests)`; fixture JSONL files committed under a path that `kani::fixture` can find; at least one ported function gets a fixture-replay test as a smoke-check.
- Phase 4: this file regenerated for the first dict batch.

## Don'ts

- Don't pick a DB layer autonomously. Flag the choices and ask.
- Don't open `ichiran/dict` ports before phase 2 lands — every one will need rework.
- Don't re-derive `*abnormal-chars*` opportunistically; it remains in the staleness ledger as its own future work.
- Don't hand-edit `PORT_PLAN.md` or `symbols.csv` — `query.py mark` / `query.py plan`.
- Don't run `build_graph.py` casually; it wipes statuses.
- Don't introduce new conventions ad-hoc — extend CONVENTIONS.md if you genuinely find a gap.
- Don't claim a function is "non-DB" without grepping its body for `query`, `select-dao`, `with-connection`, etc. — DB calls hide in helper layers.
