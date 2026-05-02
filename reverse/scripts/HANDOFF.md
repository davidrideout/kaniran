# Handoff — Rust port of ichiran

Read [`CLAUDE.md`](../../CLAUDE.md) and [`CONVENTIONS.md`](../../CONVENTIONS.md) first for orientation. This doc is the current snapshot.

---

## tl;dr — current state

| Layer | Status |
|---|---|
| Original Lisp source | Checked in at repo root, untouched. |
| **Symbol / dependency extraction** (md files) | Done. **945 md files** under `reverse/<file>.lisp/` covering 6 introspected kinds + 2 hand-written. |
| **Graph CSVs** (symbols.csv, edges.csv) | Done. **944 symbols** (689 fn, 126 global, 38 gf, 36 macro, 28 class, 14 dao, 11 struct, 1 type, 1 condition) + 2698 edges. |
| **PORT_PLAN.md** | Regenerated. **923 symbols across 921 waves**. **50 marked ported** (items 1–50, all in `ichiran/characters`). |
| **Tracer** (`:ichi-trace` package in `ichiran-repl.sh`) | Built and proven via probes. Has not yet been run on `(ichiran/test:run-all-tests)` or a corpus. |
| **Rust crate `kaniran-core`** | 50 ports + 2 Rust-only sidecars. **34 tests pass.** |
| **Conventions doc** | [`CONVENTIONS.md`](../../CONVENTIONS.md) at repo root — single source for coding/naming rules; both this file and [`CLAUDE.md`](../../CLAUDE.md) defer to it. |

`ichiran/characters` is now **50 of 61 symbols ported** (82%) — all 30 globals, the `char-class` deftype, the `hash-from-list` macro (doc-only), and 18 functions.

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
│       └── characters/
│           ├── mod.rs
│           ├── char_class_type.rs
│           ├── kani_kana_class.rs                    # sidecar: KanaClass enum (87 mora/modifier tags)
│           ├── kani_char_class_bare_scanners.rs      # sidecar: cached bare regex per CharClass
│           ├── _star_*_star_.rs       # 30 ported defparameter/defvar globals
│           │                          # (*dakuten-join* now derives via OnceLock — see below)
│           ├── as_hiragana.rs                        # function ports
│           ├── as_katakana.rs
│           ├── basic_split.rs                        # introduces SegmentKind { Misc, Word }
│           ├── collect_char_class.rs
│           ├── consecutive_char_groups.rs            # char-position offsets, regression-pinned
│           ├── count_char_class.rs
│           ├── dakuten_join.rs                       # builder used by *dakuten-join* derivation
│           ├── destem.rs
│           ├── geminate.rs
│           ├── get_char_class.rs                     # gethash-with-default → Option<KanaClass>
│           ├── hash_from_list_macro.rs               # doc-only macro stub (CONVENTIONS §4.8)
│           ├── join.rs
│           ├── kanji_cross_match.rs
│           ├── kanji_mask.rs                         # cached scanner
│           ├── kanji_match.rs                        # collapsed to bool
│           ├── kanji_regex.rs                        # builds per-word fancy_regex::Regex
│           ├── split_by_regex.rs
│           ├── test_word.rs
│           └── to_normal_char.rs
└── reverse/                          # introspection output + scripts (unchanged)
```

---

## What got built / changed this session

### 9 new function ports + 1 doc-only macro stub — second wave of `ichiran/characters` leaves

| # | Symbol | Rust shape | Notable choice |
|---|---|---|---|
| 41 | `dakuten-join` | `pub fn dakuten_join(&HashMap<KanaClass, KanaClass>, char) -> Vec<(String, String)>` | Flat plist `(in1 out1 in2 out2 ...)` lifted to paired `Vec<(String, String)>` — the only consumer expects pairs. |
| 42 | `destem` | `pub fn destem(&str, usize, CharClass) -> String` | Routes through cached `char_class_bare_scanners()`. Char-position semantics. `&optional :kana` dropped — both call-sites pass the class explicitly. |
| 43 | `geminate` | `pub fn geminate(&str) -> String` | `:fresh` dropped, always allocates (CONVENTIONS §4.6). |
| 44 | `get-char-class` | `pub fn get_char_class(char) -> Option<KanaClass>` | `(gethash k h k)` self-as-default → `Option<T>` (CONVENTIONS §4.2). |
| 45 | `hash-from-list` | doc-only `hash_from_list_macro.rs` | CONVENTIONS §4.8: macro defines hashtable globals, all 6 call-sites are or will be direct Rust `HashMap`s. Nothing to translate at the macro level. |
| 46 | `join` | `pub fn join<S: AsRef<str>>(&str, &[S]) -> String` | `&key key` dropped (single caller pre-maps). Generic bound handles `&str`/`String` callers. |
| 47 | `kanji-cross-match` | `pub fn kanji_cross_match(&str, &str, &str) -> Option<String>` | Char-position semantics. Upstream's nil-arithmetic latent crash collapsed to `None`. Inline `first_mismatch_chars` helper. |
| 48 | `kanji-mask` | `pub fn kanji_mask(&str) -> String` | Uses `*kanji-regex*` as a `(?:...)+`-wrapped scanner, cached via `OnceLock`. |
| 49 | `kanji-regex` | `pub fn kanji_regex(&str) -> fancy_regex::Regex` | Builds per-word; non-kanji chars escaped via `fancy_regex::escape`. No cache (caller-driven, unbounded keys). 2 behavioral tests. |
| 50 | `kanji-match` | `pub fn kanji_match(&str, &str) -> bool` | Position-or-nil collapsed to `bool` (CONVENTIONS §4.1). |

### `*dakuten-join*` ledger entry resolved

`_star_dakuten_join_star_.rs` now exposes `pub fn dakuten_join() -> &'static Vec<(String, String)>`, deriving via `OnceLock` from `dakuten_join(dakuten_hash(), '゛') ++ dakuten_join(handakuten_hash(), '゜')` — exact upstream construction. The captured introspector literal is preserved as a private `INTROSPECTED` constant inside the test module; the regression test sorts both sides before comparing because SBCL's `hash-table-alist` iteration order is implementation-defined. The frozen-literal ledger row for `*dakuten-join*` has been removed — only `*abnormal-chars*` remains.

`_star_punctuation_marks_star_.rs` had a doc-comment intra-link to the removed `DAKUTEN_JOIN` static; updated to point at the new `dakuten_join()` function.

### Test count 31 → 34 (+3)

- +1 `_star_dakuten_join_star_::derived_value_matches_introspected_literal_under_sort` (52-pair regression — pins the new `OnceLock` derivation against the captured value, sort-tolerant).
- +2 `kanji_regex` behavioral pins: pure-kanji word collapses to `^.+$` and accepts any non-empty reading; non-kanji chars stay literal.

No tests added for `dakuten_join` (the function), `destem`, `geminate`, `get_char_class`, `join`, `kanji_cross_match`, `kanji_mask`, `kanji_match` — they're either thin wrappers or routed-through cached infra whose behavior is verified upstream of the call (regex compile tests, scanner-cache coverage, fancy-regex's own test suite). `hash-from-list` is doc-only and has no body to test.

---

## Frozen-literal divergences (staleness ledger)

Per CONVENTIONS §3.4 and §5.3 — globals built at load time from other globals/functions we haven't ported yet are captured as Rust literals; the doc-comment on each must list (a) the upstream construction expression, (b) the dependencies that would invalidate the value, and (c) what to do once construction is portable.

Current instances:

| File | Upstream construction | Stales when these change |
|---|---|---|
| `_star_abnormal_chars_star_.rs` | `(concatenate 'string "...full-width ASCII..." *half-width-kana*)` at `characters.lisp:106-109` | `*half-width-kana*` (item 13, ported). All inputs ported — ready to flip to a `format!`-derivation parallel to `*normal-chars*` whenever picked up. |

`*dakuten-join*` was here last session — it now derives via the ported `dakuten_join` function and is no longer frozen.

---

## Conventions

The detailed rules now live in [`CONVENTIONS.md`](../../CONVENTIONS.md). Pointers retained for fast reference:

- **Canonical FQN → Rust path mapping:** `kaniran-core/src/kani/naming.rs` module-doc + tests there.
- **One file per ported Lisp symbol;** `kani_<name>.rs` for Rust-only sidecars; `_star_<name>_star_.rs` for `defparameter`-style globals; `<name>_<kind>.rs` for typed Lisp kinds.
- **Tests guard logic and integration, not literal data.** Pinning a derived value against a captured introspector literal is fine; pinning hand-typed data against itself is not.
- **Two enums in `characters/`:** `KanaClass` (87 variants in `kani_kana_class.rs` — sidecar, the closed mora/modifier tag set) and `CharClass` (9 variants in `char_class_type.rs` — port of the `char-class` deftype). Variant sets don't overlap.

---

## Decisions still open (unchanged)

1. **DB layer** — sqlx+tokio (async), diesel (sync), sea-orm (async), or hand-rolled. Affects every `ichiran/dict::*` DAO port. Heavier deps should be feature-gated since `kaniran-core` is intended to publish standalone.
2. **JMdict schema** — share ichiran's Postgres schema, or design a fresh one.
3. **Port scope** — full port (~944 symbols) vs. romanize/segment public API only (~100 symbols).

The repo is intended as a multi-crate workspace; future siblings (`kaniran-cli`, `kaniran-demo`) will live at the repo root.

---

## Next in the plan

`ichiran/characters` items 51–61 remain (11 symbols) — closes out the package. Within that:

- **51 `kanji-prefix`**, **54 `mora-length`**, **58 `safe-subseq`**, **59 `sequential-kanji-positions`**, **61 `voice-char`** — trivial.
- **52 `long-vowel-modifier-p`** — small; needs a `KanaClass::lisp_name()` method on the sidecar (the only consumer needing the upstream keyword name as a string).
- **55 `simplify-ngrams`** — thin alternation-based replacement; needs to accept both `&[(&str, &str)]` (for `*punctuation-marks*`) and the runtime `Vec<(String, String)>` from `dakuten_join()` — generic over `AsRef<str>`.
- **56 `normalize`**, **57 `rendaku`**, **60 `unrendaku`** — small, drop `:fresh`, return `String` (CONVENTIONS §4.6); `rendaku`'s `:handakuten` keyword becomes a 2-variant enum (CONVENTIONS §4.4).
- **53 `match-diff`** — the recursive optimal-alignment algorithm; only genuinely complex item left in this package. Multi-value return + recursion + char-position semantics. Needs behavioral pinning.

Once items 51–61 are done, `ichiran/characters` is fully ported (61/61) and the next package — likely `ichiran/conn` (small, 27 symbols, mostly DB plumbing) or `ichiran/numbers` (13 symbols, leaf math) — opens up. `ichiran/dict` (679 symbols) is the bulk of the remaining work and the natural place for the DB-layer decision to be made.

The tracer (`:ichi-trace`) is still built and proven via probes but **has not yet been run against `(ichiran/test:run-all-tests)` or a Japanese corpus.** Function ports so far have been hand-translated from the Lisp source rather than fixture-replayed. Once `match-diff` lands (the only non-trivial remaining function in characters), or before starting `ichiran/dict`, running the tracer over the test suite to harvest fixtures becomes the natural next infrastructure step.

---

## Resume — first three commands

```sh
# 1. Confirm the Rust crate compiles + tests pass
cargo test -p kaniran-core
# expect: 34 passed

# 2. Confirm graph still parses cleanly (only if you didn't just port — this resets statuses)
# python3 reverse/scripts/build_graph.py
# WARNING: rewrites symbols.csv from md files and resets every status to pending.
#          Commit symbols.csv first. Don't run unless you re-ran introspection.

# 3. See what's unblocked next
python3 reverse/scripts/query.py next | head
# Expect: items 51–61 of ichiran/characters appear (depending on cross-package edges).
```

---

## Gotchas (still real)

- **`run-remote.sh` rsyncs with `--delete --exclude='scripts/'`.** Two hand-written md files (`reverse/characters.lisp/char-class_type.md`, `reverse/numbers.lisp/not-a-number_condition.md`) are at risk on next introspection run. Either commit before re-running, or extend the introspector.
- **`ichiran-repl.sh` HELPERS heredoc is single-quoted bash.** Apostrophes inside it (in comments or strings) terminate the heredoc and bash interprets the rest as commands.
- **`build_graph.py` rewrites symbols.csv on every run, resetting `status` to `pending`.** Commit before re-generating, or use `query.py mark` to re-mark.
- **The naming.rs CSV-path test uses `../reverse/scripts/symbols.csv`** (relative to `CARGO_MANIFEST_DIR`, which is `kaniran-core/`). If the workspace layout moves, that path follows.
- **Introspector line numbers can drift from the checked-in `*.lisp` files.** The introspector runs against the ichiran image on `.103`; if upstream moved, citations from `symbols.csv` may not match the file-at-rest in this repo. When writing new port doc-comments, grep `characters.lisp` directly to get a current line number rather than copying from the CSV.
