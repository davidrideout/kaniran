# Handoff — Rust port of ichiran

Read [`CLAUDE.md`](../../CLAUDE.md) and [`CONVENTIONS.md`](../../CONVENTIONS.md) first for orientation. This doc is the current snapshot.

**Commits since the previous HANDOFF (`08e8ae8`):**
1. `0e28e20` Open ichiran/dict: simple-text + kanji-text + kana-text DAOs
2. `080d9a0` Port ichiran/dict static counter globals + SuffixKind sidecar
3. `3908dad` Macroexpand DSL definers in introspector; lift build_graph gate
4. `840c30e` Port counter-text family (12 symbols) + Counter dispatch enum
5. `c7c89af` Drop CONVENTIONS §4.6/§4.7; restore `:fresh` in-place semantics
6. `52160a2` Add fixture-capture pipeline + replay parser + audit harness
7. `f6ced57` Port 4 dict DAOs (entry/sense/sense-prop/conjugation) + 3 counter fns
8. `50e50d4` Port *suffix-cache* / *suffix-class* + audit async-fn coverage
9. `b28a6ee` Port wave 103 dict caches: *counter-cache*, *is-arch-cache*, *no-conj-data*

**Uncommitted in working tree:** `*init-suffixes-lock*` marked skip + plan regenerated. Reason: subsumed by `OnceLock::get_or_init`'s built-in once-only synchronization on `*suffix-cache*` / `*suffix-class*`. Two files modified — `reverse/scripts/symbols.csv`, `reverse/scripts/PORT_PLAN.md`. Commit when convenient.

---

## tl;dr — current state

| Layer | Status |
|---|---|
| Original Lisp source | Checked in at repo root, untouched. |
| **Symbol / dependency extraction** (md files) | Done. **945 md files** under `reverse/<file>.lisp/` covering 6 introspected kinds + 2 hand-written. |
| **Graph CSVs** (symbols.csv, edges.csv) | **944 symbols** + 3145 edges. |
| **Signatures** (`signatures.json`) | 763 entries (689 fn + 36 macro + 38 gf). Used by `audit-signatures`. |
| **PORT_PLAN.md** | **862 waves, 923 symbols.** |
| **Marked status** | **111 ported / 29 skip / 804 pending** (was 79 / 28 in the previous HANDOFF). |
| **Tracer** (`:ichi-trace` in `ichiran-extractor/trace_capture.lisp`) | End-to-end validated on the 400K-sentence corpus run (per CLAUDE.md). |
| **Rust crate `kaniran-core`** | 111 ports + 5 Rust-only sidecars. Six package directories: `characters/`, `conn/`, `core/`, `dict/` (NEW), `maintenance/`, `numbers/`. **128 tests pass.** |
| **Audit infrastructure** | `audit-signatures` now matches `pub async fn` as well. Currently 9 divergences in `divergences.md`. |

Closed packages: `characters` (61), `conn` (2 + 24 skip), `numbers` (14). Three closed.

`dict/` is open: 32 ported + 3 skip (out of 679 — most of the work remaining is here).

---

## Layout

```
kaniran/
├── Cargo.toml                       # workspace
├── Cargo.lock
├── CLAUDE.md
├── CONVENTIONS.md
├── kaniran-core/
│   └── src/
│       ├── lib.rs                   # `pub mod kani; characters; conn; core; dict; maintenance; numbers;`
│       ├── kani.rs / kani/          # naming.rs + fixture.rs
│       ├── characters/              # 61 ports + 3 sidecars
│       ├── conn/                    # 2 ports + 1 sidecar (KaniranContext)
│       ├── core/                    # 1 port
│       ├── dict/                    # NEW — 32 ports + 1 sidecar (kani_suffix_kind)
│       ├── maintenance/             # 1 port
│       └── numbers/                 # 14 ports + 1 sidecar (kani_num_class)
├── ichiran-extractor/               # bulk fixture-capture pipeline (FastAPI + pooled SBCL workers)
│   └── trace_capture.lisp           # :ichi-trace package
└── reverse/
    └── scripts/
        ├── build_graph.py           # gate lifted (`3908dad`); --signatures-only still bypasses
        ├── query.py                 # audit-signatures handles async fn (`50e50d4`)
        ├── symbols.csv
        ├── edges.csv
        ├── signatures.json
        ├── divergences.md           # committed, rewritten by every audit run
        ├── PORT_PLAN.md
        ├── HANDOFF.md
        └── README.md
```

---

## What got built / changed since the last HANDOFF

### 1. Opened `ichiran/dict` package — 32 ports

The dict tree is now active. Highlights:

- **Static counter globals** (`*counter-accepts*`, `*counter-foreign*`, `*counter-suffixes*`, `*extra-counter-ids*`, `*skip-counter-ids*`) plus the `SuffixKind` sidecar enum.
- **Counter class hierarchy** (12 ports) — `counter-text` base + 10 subclasses + `number-text`. Per-subclass newtype + sub-enum dispatcher per CONVENTIONS §4.7. The `Counter` enum lives in `counter_text_class.rs`.
- **Static doc-only stubs** for the counter dispatcher fns (`get-counter-ids`, `get-counter-readings`, `get-counter-stags`, `get-counter-readings`'s helpers) — async DB-touching DAO fns. These show up as ctx-injection drift in `divergences.md` (see §3 below).
- **DAO row representations** — `entry`, `sense`, `sense-prop`, `simple-text`, `kanji-text`, `kana-text`, `conjugation`. Mostly hand-rolled `FromRow` impls because `state` slots have no DB counterpart.
- **Empty-registry def-conn-var globals** — `*special-counters*`, `*suffix-cache*`, `*suffix-class*`, `*counter-cache*`, `*is-arch-cache*`, `*no-conj-data*`. Same `OnceLock<HashMap>` + `get_or_init(HashMap::new)` pattern; populators are unported (each runs DB queries) so registries return empty until they land.

The empty-registry pattern is now the de facto idiom for any def-conn-var cache whose populator is downstream of the JMdict-schema decision — eight ports use it.

### 2. Convention drops

- **CONVENTIONS §4.6 (`:fresh` keyword) and §4.7 (in-place mutation) dropped** in `c7c89af`. Restored the original semantics where `:fresh` keywords pass through as-is and mutating fns mutate. Reason: codifying these as universal rules made some ports feel artificial; per-port doc-comments handle the divergence cleanly enough on the rare cases.
- **Class-hierarchy convention added as §4.7** (renumbered, not the dropped one) — codifies the per-subclass newtype + sub-enum dispatcher pattern from the counter-text port. See `840c30e`.
- **Macroexpand step in introspector** (`3908dad`) replaced the symbol-walking pass for DSL definers (`def-counter`, `def-simple-suffix`, etc.). The build_graph gate that existed for the `not-a-number` mis-package bug is lifted in this commit.

### 3. Audit handles `pub async fn`

`audit-signatures`' `PUB_FN_NAME` regex previously matched only `pub fn`. Three DAO functions ported in `f6ced57` (`get-counter-ids`, `get-counter-readings`, `get-counter-stags`) are async, and the validator silently couldn't see them — they showed as "no pub fn found".

After `50e50d4`'s regex extension, those three now appear as real arity drift (Rust arity 1/1/2 vs Lisp 0/0/1). The drift is **ctx injection** — DB-touching async fns take a `&KaniranContext` first parameter. This pattern is **not yet codified in CONVENTIONS** — it'll happen organically as more DB-touching fns port. The `divergences.md` ledger is now the audit trail for it.

### 4. Fixture-capture pipeline + replay parser

`52160a2` added the bulk fixture-capture pipeline (FastAPI + pooled SBCL workers, `ichiran-extractor/`) plus the audit harness (`audit_fixtures` example). End-to-end validated on a 400K-sentence corpus run at ~735 sentences/sec, 100K captures/sec, 0 parse errors on replay (per CLAUDE.md). Hand-rolled prin1 reader replaces the prior `lexpr` dependency.

### 5. Current state of `divergences.md` (9 entries)

| Symbol | Drift | Reason |
|---|---|---|
| `characters:geminate` | arity 1 ≠ 2 | dropped `:fresh` (per-port doc-comment, §4.6 was retracted) |
| `characters:join` | arity 2 ≠ 3 | dropped `:key` (one upstream callsite) |
| `characters:rendaku` | arity 2 ≠ 3 | dropped `:fresh` + collapsed `:handakuten` to enum (§4.4) |
| `characters:unrendaku` | arity 1 ≠ 2 | dropped `:fresh` |
| `numbers:group-to-kana` | arity 3 ≠ 2 | split Lisp `:class-to-kana` plist into two table args |
| `numbers:parse-number*` | arity 1 ≠ 3 | dropped `:start :end` for Rust slice idiom |
| `dict:get-counter-ids` | arity 1 ≠ 0 | ctx injection on async DAO fn (uncodified) |
| `dict:get-counter-readings` | arity 1 ≠ 0 | ctx injection on async DAO fn (uncodified) |
| `dict:get-counter-stags` | arity 2 ≠ 1 | ctx injection on async DAO fn (uncodified) |

The 3 ctx-injection entries are the open decision item. The rest are documented.

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
- **Audit-signatures is part of the port-completion checklist** (CONVENTIONS §7).
- **`divergences.md` is the durable record** of every Rust port whose pub-fn surface differs from the captured Lisp lambda list. Committed.
- **Class-hierarchy port shape codified** as CONVENTIONS §4.7: per-subclass newtype + sub-enum dispatcher.

### Resolved this session

- **Empty-registry pattern is the idiom for unported def-conn-var caches:** `OnceLock<HashMap<...>>` + `get_or_init(HashMap::new)`, with concrete value types committed up front. Eight existing ports use it.
- **`build_graph.py` regen gate lifted** in `3908dad`. The introspector's macroexpand pass now records DSL-definer expansions correctly.
- **Audit handles `pub async fn`** in `50e50d4`.

### Still open

1. **JMdict schema** — share ichiran's Postgres schema or design fresh. Required before any populator port (currently 8 def-conn-var caches sit empty waiting on it). Already the gating decision for `find-sticky-positions` / `process-word-info` / `get-original-reading` (visible in `query.py next`).
2. **XML reader** for the initial JMdict.xml corpus load. Candidates: `roxmltree`, `quick-xml`, `xmltree`. JMdict's external DTD entities need a small inlining preprocessing step. Triggered when corpus-load work begins (`dict-load.lisp` chunk).
3. **Port scope** — full port (~944 symbols) vs. romanize/segment public API only (~100 symbols).
4. **Ctx-injection convention.** DB-touching async fns take `&KaniranContext` as the first parameter — visible as 3 ctx-injection drift entries in `divergences.md`. Not codified yet; per-port doc-comments cover it for now. Codify when the second batch of DAO fns lands.

---

## Next in the plan

`query.py next` reports **27 unblocked symbols** — large jump from the previous HANDOFF's 6 because the dict cache ports unblocked the surrounding suffix / counter / conjugation fns.

Top of the list (those without DB-decision dependencies):

| FQN | Kind | File | Notes |
|---|---|---|---|
| `ichiran:get-character-classes` | fn | `romanize.lisp:3` | Pure char-class transform — fully unblocked. |
| `ichiran/custom:normalize-geo` | fn | `dict-custom.lisp:176` | Small string cleanup. |
| `ichiran/custom:as-xml-simple` | fn | `dict-custom.lisp:225` | XML-library decision (#2). |

Most of the other 24 unblocked symbols touch DAOs and are gated on the JMdict-schema decision. See `query.py next` for the full list.

**Going strictly in plan order:** wave 109 (`init-suffixes-running-p`) is skip; wave 110 is `find-word-seq` — a `find-word-with-pos` derivative that hits the kana/kanji DAOs. The next non-skip walkable item depends on which DAO query helpers port first.

**Things an agent can do right now without further blocking decisions:**

1. **Port the two non-DB unblocked leaves** (`ichiran:get-character-classes`, `ichiran/custom:normalize-geo`).
2. **Codify ctx-injection in CONVENTIONS** as §4.something — there are now 3 visible divergence entries; the convention has earned its keep.
3. **Run the tracer against the ichiran test suite** for fixture harvest — the pipeline is built and validated; running it against the real test suite produces the replay corpus.

**Decisions that gate the bulk of remaining work:**
1. **JMdict schema** — required before any populator port.
2. **XML reader** — required when corpus-load work begins.
3. **Port scope** — affects how aggressively to pursue the long tail.

**User's call; don't pick autonomously.**

---

## Resume — first three commands

```sh
# 1. Confirm the Rust crate compiles + tests pass
cargo test -p kaniran-core --lib
# expect: 128 passed

# 2. Confirm divergences.md still matches the working tree (audit is deterministic)
python3 reverse/scripts/query.py audit-signatures
git diff reverse/scripts/divergences.md
# expect: no diff

# 3. See what's unblocked next (27 symbols)
python3 reverse/scripts/query.py next
```

---

## Gotchas (still real + new)

- **Uncommitted skip in working tree.** `*init-suffixes-lock*` was marked skip in this session but not committed. `git status` shows `symbols.csv` + `PORT_PLAN.md` modified.
- **`signatures.json` only covers callable kinds** (fn / macro / gf — 763 entries). Globals, structs, classes, daos, types, conditions are not in the file (no callable surface to audit).
- **`divergences.md` is committed** and rewritten on every `audit-signatures` run. After every port: `git diff reverse/scripts/divergences.md` is the review surface.
- **`audit-signatures --only <pkg>` filters STDOUT only.** The file always reflects the full sweep.
- **Audit catches `pub fn` AND `pub async fn`.** The regex was extended in `50e50d4` after three async DAO fns ported in `f6ced57` showed as "no pub fn found".
- **Empty-registry caches use a `_cache` accessor suffix when the bare name collides.** `*no-conj-data*` accessor is `no_conj_data_cache()` because `no-conj-data` (`dict.lisp:339`) is a homonymous Lisp function that's not yet ported.
- **`run-remote.sh` rsyncs with `--delete --exclude='scripts/'`.** Two hand-written md files (`reverse/characters.lisp/char-class_type.md`, `reverse/numbers.lisp/not-a-number_condition.md`) are at risk on next introspection run. Either commit before re-running, or back them up.
- **Source-walk over-collects on purpose.** Records every ichiran-package symbol the Lisp reader sees in a defining form, including locals. The Python pass filters callees against `symbols.csv` membership.
- **The naming.rs CSV-path test uses `../reverse/scripts/symbols.csv`** (relative to `CARGO_MANIFEST_DIR` = `kaniran-core/`). If layout moves, update.
- **Introspector line numbers can drift from the checked-in `*.lisp` files.** Grep upstream `.lisp` directly when writing new doc-comments.
- **`query.py mark` requires lowercase FQNs.** CSV stores lowercase even though Lisp symbols print uppercase.
- **`query.py next` only counts `ported` callees.** A symbol whose every callee is `skip` will NOT appear in `next`. Scan `PORT_PLAN.md` directly for what's actually walkable in plan order after a wave of skips.
- **Env-var tests in `conn::get_ichiran_connection_env`** mutate process env vars and must serialize on a static `Mutex`.
- **`KaniranContext` ownership.** Each instance owns its `PgPool` and (eventually) its caches. `Clone` is derived (cheap — `PgPool` is internally `Arc<Pool>`-shaped). The `Inner` refactor that consolidates the def-conn-var caches into typed `OnceCell` fields hasn't landed yet — the 8 empty-registry globals all currently use module-local `OnceLock` per memory `feedback_no_oncelock`.
- **Function name vs file stem for trailing `*`.** The file stem for `parse-number*` is `parse_number_star_.rs` (trailing `_` per `kani::naming` collision-prevention rule). The `pub fn` name inside must match. The audit catches mismatches.
