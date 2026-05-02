# Handoff — Rust port of ichiran

Read [`CLAUDE.md`](../../CLAUDE.md) and [`CONVENTIONS.md`](../../CONVENTIONS.md) first for orientation. This doc is the current snapshot.

**Baseline commit:** the commit that lands items 51–61 (built on top of `18c70ec`). Run `git log --oneline` to find it. The *previous* batch — items 41–50 plus the `*dakuten-join*` derivation — is `18c70ec`.

---

## tl;dr — current state

| Layer | Status |
|---|---|
| Original Lisp source | Checked in at repo root, untouched. |
| **Symbol / dependency extraction** (md files) | Done. **945 md files** under `reverse/<file>.lisp/` covering 6 introspected kinds + 2 hand-written. |
| **Graph CSVs** (symbols.csv, edges.csv) | Done. **944 symbols** (689 fn, 126 global, 38 gf, 36 macro, 28 class, 14 dao, 11 struct, 1 type, 1 condition) + 2698 edges. |
| **PORT_PLAN.md** | Regenerated. **923 symbols across 921 waves**. **61 marked ported** — `ichiran/characters` is fully complete. |
| **Tracer** (`:ichi-trace` package in `ichiran-repl.sh`) | Built and proven via probes. Has not yet been run on `(ichiran/test:run-all-tests)` or a corpus. |
| **Rust crate `kaniran-core`** | 61 ports + 2 Rust-only sidecars + 1 sidecar method (`KanaClass::lisp_name`). **49 tests pass.** |
| **Conventions doc** | [`CONVENTIONS.md`](../../CONVENTIONS.md) at repo root — single source for coding/naming rules; both this file and [`CLAUDE.md`](../../CLAUDE.md) defer to it. |

`ichiran/characters` is now **61 of 61 symbols ported (100%)** — closes out the package. 19 symbols across 7 other packages are unblocked for the next wave (see "Next in the plan" below).

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

### 11 function ports — closes out `ichiran/characters`

| # | Symbol | Rust shape | Notable choice |
|---|---|---|---|
| 51 | `kanji-prefix` | `pub fn kanji_prefix(&str) -> String` | Cached `OnceLock<Regex>` for `^.*[kanji]`. Empty string when no kanji (mirrors `(or scan "")`). |
| 52 | `long-vowel-modifier-p` | `pub fn long_vowel_modifier_p(KanaClass, char) -> bool` | Uses the new `KanaClass::lisp_name()` sidecar method. Predicate-only (CONVENTIONS §4.1). |
| 53 | `match-diff` | `pub fn match_diff(&str, &str) -> Option<(Vec<MatchSegment>, usize)>` | Inline `MatchSegment { Equal, Diff }` enum (avoids name collision with `fancy_regex::Match`). Char-position throughout. Empty inputs return `None`. |
| 54 | `mora-length` | `pub fn mora_length(&str) -> usize` | Modifier set inlined as `&str` literal; per-char `.contains`. |
| 55 | `simplify-ngrams` | `pub fn simplify_ngrams<S, T>(&str, &[(S, T)]) -> String` | Generic over `AsRef<str>` so both `*punctuation-marks*` (`&[(&str, &str)]`) and `dakuten_join()` (`&Vec<(String, String)>`) work without conversion. Per-call regex (caller-driven, unbounded keys). |
| 56 | `normalize` | `pub fn normalize(&str, NormalizationContext) -> String` | Two-pass: char-by-char `to_normal_char` then `simplify_ngrams`. Default-context map combines `*punctuation-marks*` with `dakuten_join()`. `:fresh` dropped. |
| 57 | `rendaku` | `pub fn rendaku(&str, Voicing) -> String` | `&key handakuten` → 2-variant `Voicing { Dakuten, Handakuten }` (CONVENTIONS §4.4). `:fresh` dropped. |
| 58 | `safe-subseq` | `pub fn safe_subseq(&str, usize, Option<usize>) -> Option<String>` | Char-position bounds check; `&optional end` → `Option<usize>`. |
| 59 | `sequential-kanji-positions` | `pub fn sequential_kanji_positions(&str, usize) -> Vec<usize>` | Cached zero-width lookahead. Returns char positions (CONVENTIONS §4.5). |
| 60 | `unrendaku` | `pub fn unrendaku(&str) -> String` | Introduces `pub(super) fn transpose(char, KanaClass, KanaClass) -> Option<char>` reused by `rendaku`. `:fresh` dropped. |
| 61 | `voice-char` | `pub fn voice_char(KanaClass) -> KanaClass` | `(gethash cc h cc)` collapses to `unwrap_or(cc)` when input/output are same-typed (CONVENTIONS §4.2 example). |

### `KanaClass::lisp_name()` method on the sidecar

Added an inherent method `pub fn lisp_name(&self) -> &'static str` to `KanaClass` in `kani_kana_class.rs`. One arm per of the 87 variants; returns the upstream Lisp keyword's `(string :keyword)` form (`Ka` → `"KA"`, `PlusYa` → `"+YA"`, `LongVowel` → `"LONG-VOWEL"`, `IterV` → `"ITER-V"`). `long_vowel_modifier_p` is the only current consumer; the method is also useful debug-output and is the natural place to land any future "upstream symbol form" needs.

### Test count 34 → 49 (+15)

- +2 `kanji_prefix` (greedy-prefix behavior, empty-when-no-kanji)
- +5 `match_diff` (empty → None, equal-strings, single-char Diff, shared-prefix-then-Diff, CJK char-position alignment with non-trivial score)
- +2 `safe_subseq` (char vs byte slicing, out-of-range / inverted bounds)
- +2 `sequential_kanji_positions` (lookahead semantics on adjacent kanji, non-adjacency rejection)
- +2 `simplify_ngrams` (runtime `dakuten_join()` integration, empty-map no-op)
- +2 `normalize` (Default mode end-to-end through both phases, Kana mode preserving punctuation)

No tests added for `voice_char`, `mora_length`, `long_vowel_modifier_p`, `rendaku`, `unrendaku` — they're either single-line lookups, thin wrappers around already-tested machinery, or pure data-driven (CONVENTIONS §6).

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
3. **Port scope** — full port (~944 symbols) vs. romanize/segment public API only (~100 symbols).

The repo is intended as a multi-crate workspace; future siblings (`kaniran-cli`, `kaniran-demo`) will live at the repo root.

---

## Next in the plan

`ichiran/characters` is closed out. The next wave (`query.py next`) reports **19 unblocked symbols**, distributed:

- **`ichiran` (bare package)** — `process-iteration-characters` (`romanize.lisp:7`). Pure kana logic; opens a new package directory (`core/`).
- **`ichiran/custom`** — `as-xml-simple`, `normalize-geo` (2). XML / dict-custom utilities.
- **`ichiran/dict`** — 11 leaves (`find-word`, `find-substring-words`, `find-word-seq`, `find-word-with-pos`, `find-words-seqs`, `find-sticky-positions`, `add-reading`, `get-candidates`, `get-kanji-kana-old`, `process-hints`, `process-word-info`, `remove-hiragana-nokanji`, `sense-exists-p`). Most of these touch the database — DB-layer decision is the prerequisite.
- **`ichiran/kanji`** — `get-original-reading`, `get-reading-alternatives` (2). Likely DB-touching.
- **`ichiran/maintenance`** — `diff-content` (1). String-diff utility.

Recommended order:

1. **Take the non-DB leaves first to keep moving while the DB-layer decision settles**:
   - `ichiran:process-iteration-characters` (kana-only)
   - `ichiran/maintenance:diff-content` (probably string-only)
   - `ichiran/custom:as-xml-simple` if it's pure XML emission, defer if it queries
2. **Make the DB-layer decision** before anything in `ichiran/dict`. Three concrete options to evaluate: `sqlx` (async, query macros, compile-time-checked), `diesel` (sync, type-safe ORM), `sea-orm` (async, ActiveRecord-shaped). The choice ripples through every DAO and into `kaniran-core`'s public surface.
3. Either `ichiran/numbers` (13 symbols, leaf math, no DB) or `ichiran/conn` (26 symbols, DB plumbing — only after the DB-layer decision) opens up larger as more leaves get ported.

The tracer (`:ichi-trace`) is still built and proven via probes but **has not yet been run against `(ichiran/test:run-all-tests)` or a Japanese corpus.** With `ichiran/characters` now closed, this is a natural inflection point to run a fixture-harvest sweep before tackling `ichiran/dict` — many of those functions need real-world Japanese text to verify equivalence.

---

## Resume — first three commands

```sh
# 1. Confirm the Rust crate compiles + tests pass
cargo test -p kaniran-core
# expect: 49 passed

# 2. See what's unblocked next (mix of packages — see "Next in the plan")
python3 reverse/scripts/query.py next | head

# 3. Confirm the package counts
python3 reverse/scripts/query.py stats
# expect: ichiran/characters 0 61 (100% complete)
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
