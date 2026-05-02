# Handoff — Rust port of ichiran

Read [`CLAUDE.md`](../../CLAUDE.md) and [`CONVENTIONS.md`](../../CONVENTIONS.md) first for orientation. This doc is the current snapshot.

**Baseline commit:** `124c2dc` — `Port characters.lisp items 51-61; close out ichiran/characters`. Phase-1 of the next-wave (this session) is on top of that, uncommitted in the working tree. Run `git log --oneline` for the full port history.

---

## tl;dr — current state

| Layer | Status |
|---|---|
| Original Lisp source | Checked in at repo root, untouched. |
| **Symbol / dependency extraction** (md files) | Done. **945 md files** under `reverse/<file>.lisp/` covering 6 introspected kinds + 2 hand-written. |
| **Graph CSVs** (symbols.csv, edges.csv) | Done. **944 symbols** (689 fn, 126 global, 38 gf, 36 macro, 28 class, 14 dao, 11 struct, 1 type, 1 condition) + 2698 edges. |
| **PORT_PLAN.md** | Regenerated. **921 waves, 923 symbols**. **63 marked ported** — `ichiran/characters` complete + first 2 next-wave leaves. |
| **Tracer** (`:ichi-trace` package in `ichiran-repl.sh`) | Built and proven via probes. Has not yet been run on `(ichiran/test:run-all-tests)` or a corpus. |
| **Rust crate `kaniran-core`** | 63 ports + 2 Rust-only sidecars + 1 sidecar method (`KanaClass::lisp_name`). Three package directories: `characters/`, `core/`, `maintenance/`. **60 tests pass.** |
| **Conventions doc** | [`CONVENTIONS.md`](../../CONVENTIONS.md) at repo root — single source for coding/naming rules; both this file and [`CLAUDE.md`](../../CLAUDE.md) defer to it. |

`ichiran/characters` remains 61/61 (100%). Phase-1 of the next wave landed two leaves: `ichiran:process-iteration-characters` (opens `core/`) and `ichiran/maintenance:diff-content` (opens `maintenance/`). 17 symbols across 5 packages are still unblocked, but most need the open DB-layer decision.

---

## Layout

```
kaniran/                              # repo root — workspace Cargo.toml here
├── Cargo.toml                        # workspace-only
├── Cargo.lock
├── CLAUDE.md                         # repo orientation, methodology
├── CONVENTIONS.md                    # canonical coding/naming rules — read before editing port files
├── kaniran-core/                     # the core crate
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── kani.rs                   # kaniran infra (NOT a port)
│       ├── kani/
│       │   ├── fixture.rs            # JSONL replay
│       │   └── naming.rs             # FQN → path; canonical for the path mapping rule. Covers all 9 kinds.
│       ├── core/                     # NEW: bare `ichiran` package; renamed to avoid shadowing crate root
│       │   ├── mod.rs
│       │   └── process_iteration_characters.rs   # NEW (62): inline CcItem enum (KanaClass | char)
│       ├── maintenance/              # NEW: `ichiran/maintenance` package (DB-free leaves only so far)
│       │   ├── mod.rs
│       │   └── diff_content.rs       # NEW (63): DiffResult enum, similar-crate unified diff
│       └── characters/
│           ├── mod.rs
│           ├── char_class_type.rs
│           ├── kani_kana_class.rs                    # sidecar: KanaClass enum + lisp_name() method
│           ├── kani_char_class_bare_scanners.rs      # sidecar: cached bare regex per CharClass
│           ├── _star_*_star_.rs       # 30 ported defparameter/defvar globals
│           ├── as_hiragana.rs                        # function ports
│           ├── as_katakana.rs
│           ├── basic_split.rs
│           ├── collect_char_class.rs
│           ├── consecutive_char_groups.rs
│           ├── count_char_class.rs
│           ├── dakuten_join.rs
│           ├── destem.rs
│           ├── geminate.rs
│           ├── get_char_class.rs
│           ├── hash_from_list_macro.rs               # doc-only macro stub
│           ├── join.rs
│           ├── kanji_cross_match.rs
│           ├── kanji_mask.rs
│           ├── kanji_match.rs
│           ├── kanji_prefix.rs                       # NEW (51): cached scanner
│           ├── kanji_regex.rs
│           ├── long_vowel_modifier_p.rs              # NEW (52): predicate using lisp_name()
│           ├── match_diff.rs                         # NEW (53): MatchSegment enum, recursive
│           ├── mora_length.rs                        # NEW (54)
│           ├── normalize.rs                          # NEW (56): two-pass over to_normal_char + simplify_ngrams
│           ├── rendaku.rs                            # NEW (57): Voicing enum
│           ├── safe_subseq.rs                        # NEW (58): char-position bounds-checked
│           ├── sequential_kanji_positions.rs         # NEW (59): zero-width lookahead
│           ├── simplify_ngrams.rs                    # NEW (55): generic AsRef<str> map
│           ├── split_by_regex.rs
│           ├── test_word.rs
│           ├── to_normal_char.rs
│           ├── unrendaku.rs                          # NEW (60): introduces transpose() helper
│           └── voice_char.rs                         # NEW (61): unwrap_or fallback
└── reverse/                          # introspection output + scripts (unchanged)
```

---

## What got built / changed this session

### Phase-1 of the next wave — 2 non-DB leaves, 2 new package directories

The session-1 plan (see `NEXT_PROMPT.md`) was to take the 2-4 leaves that don't depend on the unresolved DB-layer decision. `as-xml-simple` was triaged out because it builds an XML DOM that flows directly into `ichiran/dict::load-entry` (DAO consumers `municipality` and `ward`); it belongs with the XML-library decision in phase 4, not phase 1.

| # | Symbol | Rust shape | Notable choice |
|---|---|---|---|
| 62 | `process-iteration-characters` (bare `ichiran`) | `pub fn process_iteration_characters(&[CcItem]) -> Vec<CcItem>` | First file under `core/` (bare-package directory). Inline `CcItem { Class(KanaClass), Char(char) }` enum (CONVENTIONS §4.3) models the dual-shape items from `*char-class-hash*`'s default-as-self lookup. `IterV` voicing only fires on `Class(_)` items — `Char(_)` previas pass through to mirror upstream's hash-miss fallback. |
| 63 | `diff-content` (`ichiran/maintenance`) | `pub fn diff_content(Option<&str>, Option<&str>, bool) -> DiffResult` | First file under `maintenance/`. `DiffResult { Gone, New, Diff(String) }` collapses upstream's `(or simple-string (member :gone :new))` (CONVENTIONS §4.3). `Option<&str>` makes the "absent vs. empty" distinction explicit. `&key short` stays a `bool` — the parameter name reads naturally at the callsite (CONVENTIONS §4.4). Diff library: `similar` (closest Rust equivalent of upstream `cl-diff`'s unified output); output text not byte-identical with cl-diff, tests pin behavior not literal text. |

### New workspace dependency

`similar = "2"` added to `[workspace.dependencies]` and pulled into `kaniran-core` as a regular (non-feature-gated) dep. If `ichiran/maintenance` grows further and the dep becomes a candidate for separation, feature-gating it under a `maintenance` feature is the cheap migration.

### Test count 49 → 60 (+11)

- +6 `process_iteration_characters` (iter-at-start drops, iter repeats prev, iter-v voices Sa→Za, run-of-iters all reference original prev, iter-v on unvoiceable A falls through, char prev passes through iter-v unchanged)
- +5 `diff_content` (short=true → Gone when new is None, short=true → New when old is None, short=false returns Diff even with one missing side, identical inputs produce empty body, `[\r\n]+` collapses runs of newlines so `"a\r\n\r\nb"` matches `"a\nb"`)

---

## Frozen-literal divergences (staleness ledger)

Per CONVENTIONS §3.4 and §5.3 — globals built at load time from other globals/functions we haven't ported yet are captured as Rust literals.

Current instances:

| File | Upstream construction | Stales when these change |
|---|---|---|
| `_star_abnormal_chars_star_.rs` | `(concatenate 'string "...full-width ASCII..." *half-width-kana*)` at `characters.lisp:106-109` | `*half-width-kana*` (item 13, ported). All inputs ported — ready to flip to a `format!`-derivation parallel to `*normal-chars*` whenever picked up. |

Unchanged from the previous session — `*dakuten-join*` is no longer here (derives via the ported `dakuten_join`).

---

## Conventions

The detailed rules live in [`CONVENTIONS.md`](../../CONVENTIONS.md). Pointers retained for fast reference:

- **Canonical FQN → Rust path mapping:** `kaniran-core/src/kani/naming.rs` module-doc + tests there.
- **One file per ported Lisp symbol;** `kani_<name>.rs` for Rust-only sidecars; `_star_<name>_star_.rs` for `defparameter`-style globals; `<name>_<kind>.rs` for typed Lisp kinds.
- **Tests guard logic and integration, not literal data.**
- **Sidecar methods (e.g. `KanaClass::lisp_name`) live on the data type's sidecar file, not in a separate utils module** (CONVENTIONS §1).
- **Two enums in `characters/`:** `KanaClass` (87 variants in `kani_kana_class.rs`) and `CharClass` (9 variants in `char_class_type.rs`). Variant sets don't overlap.

---

## Decisions still open

1. **DB layer** — sqlx+tokio (async), diesel (sync), sea-orm (async), or hand-rolled. Affects every `ichiran/dict::*` DAO port, and `ichiran/conn`, and a meaningful slice of `ichiran/kanji`. Heavier deps should be feature-gated since `kaniran-core` is intended to publish standalone. **This decision now blocks most of the next-wave work** — see below.
2. **JMdict schema** — share ichiran's Postgres schema, or design a fresh one.
3. **XML reader** for the initial JMdict.xml corpus load. Upstream's `as-xml-simple` synthesizes XML only to feed it into the same-process loader, so a Rust port skips the writer entirely and constructs the loaded shape directly — `as-xml-simple` itself doesn't need a counterpart. The reader, however, must parse real JMdict.xml (one-shot at corpus-load time). Candidates: `roxmltree` (read-only DOM, fast), `quick-xml` event reader (fast, no DOM), `xmltree` (mutable DOM, slower). JMdict's external DTD entities (`&n;` etc.) are not auto-resolved by any of these — the standard workaround is a small entity-inlining preprocessing step using the DTD's internal-subset mechanism. Library-agnostic.
4. **Port scope** — full port (~944 symbols) vs. romanize/segment public API only (~100 symbols).

The repo is intended as a multi-crate workspace; future siblings (`kaniran-cli`, `kaniran-demo`) will live at the repo root.

---

## Next in the plan

Phase 1 complete. `query.py next` now reports **17 unblocked symbols**, distributed:

- **`ichiran/custom`** — `as-xml-simple`, `normalize-geo` (2). XML / dict-custom utilities. `as-xml-simple` is gated on the XML library decision (#3 above) and is consumed exclusively by DAO methods; `normalize-geo` is a small string-cleanup helper that's likely portable independently.
- **`ichiran/dict`** — 11 leaves (`find-word`, `find-substring-words`, `find-word-seq`, `find-word-with-pos`, `find-words-seqs`, `find-sticky-positions`, `add-reading`, `get-candidates`, `get-kanji-kana-old`, `process-hints`, `process-word-info`, `remove-hiragana-nokanji`, `sense-exists-p`). Most of these touch the database — DB-layer decision is the prerequisite.
- **`ichiran/kanji`** — `get-original-reading`, `get-reading-alternatives` (2). Likely DB-touching.
- **`ichiran/maintenance`** — 12 remaining symbols, all DB-touching. Wait for phase 2.

Recommended order:

1. **Phase 2 — make the DB-layer decision** before anything in `ichiran/dict`. Three concrete options to evaluate: `sqlx` (async, query macros, compile-time-checked), `diesel` (sync, type-safe ORM), `sea-orm` (async, ActiveRecord-shaped). The choice ripples through every DAO and into `kaniran-core`'s public surface. Decisions #1 (DB), #2 (schema), #3 (XML libs) cluster naturally — opening any one effectively opens the others. **User's call; do not pick autonomously.**
2. **Phase 3 — fixture harvest.** With `characters` closed and ports about to enter the bulk-of-the-codebase `dict` package, this is the natural moment to run `:ichi-trace` against `(ichiran/test:run-all-tests)` and harvest fixtures for the 63 ported functions. The fixture-replay infra in `kani::fixture` is wired and waiting.
3. **Phase 4 — `ichiran/dict` proper**, fixture-driven from day one.

A non-DB tangent that doesn't need decisions: `ichiran/numbers` (13 symbols, leaf math, no DB). Could fill time during phase 2 if the DB decision takes a while.

---

## Resume — first three commands

```sh
# 1. Confirm the Rust crate compiles + tests pass
cargo test -p kaniran-core --lib
# expect: 60 passed

# 2. See what's unblocked next (mix of packages — see "Next in the plan")
python3 reverse/scripts/query.py next | head

# 3. Confirm the package counts
python3 reverse/scripts/query.py stats
# expect: ichiran/characters 0 61, ichiran 1 ported, ichiran/maintenance 1 ported
```

---

## Gotchas (still real)

- **`run-remote.sh` rsyncs with `--delete --exclude='scripts/'`.** Two hand-written md files (`reverse/characters.lisp/char-class_type.md`, `reverse/numbers.lisp/not-a-number_condition.md`) are at risk on next introspection run. Either commit before re-running, or extend the introspector.
- **`ichiran-repl.sh` HELPERS heredoc is single-quoted bash.** Apostrophes inside it (in comments or strings) terminate the heredoc and bash interprets the rest as commands.
- **`build_graph.py` rewrites symbols.csv on every run, resetting `status` to `pending`.** Commit before re-generating, or use `query.py mark` to re-mark.
- **The naming.rs CSV-path test uses `../reverse/scripts/symbols.csv`** (relative to `CARGO_MANIFEST_DIR`, which is `kaniran-core/`). If the workspace layout moves, that path follows.
- **Introspector line numbers can drift from the checked-in `*.lisp` files.** The introspector runs against the ichiran image on `.103`; if upstream moved, citations from `symbols.csv` may not match the file-at-rest in this repo. When writing new port doc-comments, grep `characters.lisp` directly to get a current line number rather than copying from the CSV.
- **`query.py mark` requires lowercase FQNs** (e.g. `ichiran/characters:voice-char`, not `ICHIRAN/CHARACTERS:VOICE-CHAR`). The CSV stores lowercase even though Lisp symbols print uppercase.
- **Closures in `fancy_regex::Regex::replace_all`** must implement the `Replacer` trait. The simplest signature that works is `|caps: &fancy_regex::Captures| -> String { ... }` — annotating the return type explicitly avoids a type-inference dead-end.
