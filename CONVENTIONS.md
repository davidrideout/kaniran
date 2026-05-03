# CONVENTIONS.md — kaniran Rust port

Read this **before** adding or editing port files. It captures the rules that have accumulated as the port has progressed; following them keeps the tree internally consistent and lets fixture-replay testing work without per-file translation logic.

For repo overview, methodology, and remote-host info, see [`CLAUDE.md`](./CLAUDE.md). This file is specifically about how a port file should look, what to name it, and how its API should differ from the Lisp original (and when not to).

---

## 1. Where things live

```
kaniran-core/src/
  lib.rs                       # `pub mod kani;` and one `pub mod <package>;` per ported package
  kani.rs   /  kani/           # crate-only infra (naming, fixture replay) — NOT a Lisp port
  characters/                  # mirror of `ichiran/characters` package
    mod.rs                     # `pub mod <stem>;` for every file in the dir
    <name>.rs                  # one ported symbol per file
    <name>_<kind>.rs           # for typed Lisp kinds (struct/class/dao/type/condition/macro)
    _star_<name>_star_.rs      # for ports of `defparameter` / `defvar` / `defconstant`
    kani_<name>.rs             # Rust-only sidecar (no Lisp counterpart)
```

- One Lisp symbol per Rust file. Don't co-locate two symbols, even if both are tiny.
- Don't introduce per-package "helpers" or "utils" modules. If a helper is needed by multiple ports, it goes on the relevant data type as a method (e.g. `KanaClass::lisp_name()`), not in a shared util file.
- `kani/` is reserved for original Rust infrastructure with no Lisp counterpart (the FQN translator, fixture-replay envelope, etc.). Don't put port files there.
- Adding a file requires a `pub mod <stem>;` line in the package's `mod.rs`. The `kani::naming` coverage tests will not catch missing `mod` declarations — `cargo check` will.

---

## 2. Naming (Lisp FQN → Rust path)

**Single source of truth: [`kaniran-core/src/kani/naming.rs`](./kaniran-core/src/kani/naming.rs).** The module-doc there spells out the rules; the tests there pin them against every FQN in `reverse/scripts/symbols.csv`.

Summary for the common cases:

| Lisp FQN | Kind | Rust path |
|---|---|---|
| `ICHIRAN/CHARACTERS:AS-HIRAGANA` | fn / gf | `characters/as_hiragana.rs` |
| `ICHIRAN/CHARACTERS:*ABNORMAL-CHARS*` | global | `characters/_star_abnormal_chars_star_.rs` |
| `ICHIRAN/CHARACTERS:CHAR-CLASS` | type | `characters/char_class_type.rs` |
| `ICHIRAN/DICT:KANJI` | dao | `dict/kanji_dao.rs` |
| `ICHIRAN/DICT:DEF-COUNTER` | macro | `dict/def_counter_macro.rs` |
| `ICHIRAN:JOIN-PARTS` | fn (bare `ichiran` package) | `core/join_parts.rs` |

Substitution rules in the file stem: lowercase, `*` → `_star_`, `+` → `_plus_`, `-` → `_`. Leading/trailing `_` is preserved (some symbols differ only in trailing `-`). Don't try to remember edge cases — feed any FQN through `kani::naming::fqn_to_path` and trust the result.

**`kani_<name>.rs` sidecars.** Rust-only types/values that don't exist in the Lisp use a `kani_` filename prefix (no kind suffix, never appear in `symbols.csv`). Current example: `characters/kani_kana_class.rs` holds `KanaClass`, the closed enum of mora/modifier tags that the Lisp uses inline as `:KA`, `:+YA`, etc. without a named type.

---

## 3. Doc-comment requirements

Every port file's first item is a `//!` module doc-comment. Required content:

1. **Citation:** `Port of \`ichiran/<pkg>:<name>\` (\`<file>.lisp:<line>\`).` Always link to the upstream source; line numbers come from `reverse/<file>.lisp/<name>*.md`.
2. **Behavior summary** in 2–5 sentences. Describe what it does, not how it does it.
3. **Divergences from Lisp**, when present. If the Rust signature collapses a multi-value return to `Option<T>`, replaces a `&key` keyword with a `bool`, or returns a new `String` instead of mutating, **say so in the doc-comment.** A future port author needs to understand why the API drifted.
4. **For frozen-literal globals** (data captured as a Rust literal because its construction inputs aren't ported yet): list (a) the upstream construction expression, (b) the dependencies whose change would invalidate the value, and (c) what to do once the construction logic itself is ported. See the staleness ledger in [`reverse/scripts/HANDOFF.md`](./reverse/scripts/HANDOFF.md#frozen-literal-divergences-staleness-ledger).

Do **not** write doc-comments that:
- Restate the function name (`/// Compute mora length` for `pub fn mora_length`).
- Describe internals that a reader can see in 5 lines of code.
- Make claims about callers ("used by X", "called from the Y flow") — those rot the moment the codebase shifts.

---

## 4. Translating Lisp shapes to Rust APIs

The Lisp uses idioms (multi-value returns, plist keywords, in-place mutation, tagged cons cells) that don't translate 1:1 to idiomatic Rust. Codified decisions follow. **Apply these mechanically — don't relitigate them per file.**

### 4.1. Predicate-only callers → `bool`

If every caller of a function uses its return value purely as a truthiness check, return `bool`. The Lisp's actual return (often a position from `ppcre:scan` or `T` from a hash-table presence flag) is incidental.

Verify by grepping callers before collapsing. Concretely:

```rust
// Lisp: (defun test-word (word char-class) ... (ppcre:scan regex word))
// Callers: (if (test-word w :kana) ...)  — pure predicate
pub fn test_word(word: &str, char_class: CharClass) -> bool { ... }
```

If even one caller uses the position, return the richer type and let predicate-callers do `result.is_some()`.

### 4.2. `(gethash key table key)` (default-as-self) → `Option<T>`

The Lisp idiom `(gethash k h k)` returns `k` when the key is missing — useful so the caller can chain. In Rust, return `Option<T>` and let the caller fall back to the input they already have. Concretely:

```rust
// Lisp: (gethash cc *dakuten-hash* cc)  — returns voiced class or cc itself
pub fn voice_char(cc: KanaClass) -> KanaClass {
    dakuten_hash().get(&cc).copied().unwrap_or(cc)
}
```

Don't model "either a class or the input char" with an enum. The fallback information is already in the caller's hand.

### 4.3. Closed `(:tag . value)` shapes → Rust enum

When the Lisp uses a fixed set of tagged cons cells — `(:misc . str)` / `(:word . str)`, or returning `(values :found data)` / `(values :not-found nil)` — port to a Rust enum. Define the enum **inline in the port file** unless the same shape appears in multiple ports.

```rust
// basic_split.rs
pub enum SegmentKind { Misc, Word }
pub fn basic_split(s: &str) -> Vec<(SegmentKind, String)> { ... }
```

The variant set must be exhaustive — if there's any chance a future Lisp callsite adds a new tag, leave a `// TODO: more tags` comment and lean toward `String` for the tag.

### 4.4. Binary `&key` keywords → 2-variant enum

A `&key` keyword that's only ever set to one specific value or absent (`:context :kana` or unspecified) becomes a 2-variant enum in Rust. Define the enum **inline in the port file** (or in a sibling sidecar if multiple ports share it). Multi-valued keywords (e.g. `&key (start 0) (end (length str))`) take their natural type.

```rust
// Lisp: (to-normal-char char &key context)  — context is :kana or absent
pub enum NormalizationContext { Default, Kana }
pub fn to_normal_char(c: char, context: NormalizationContext) -> Option<char> { ... }

// Call sites — the choice is named, not encoded in a polarity:
to_normal_char(c, NormalizationContext::Default);
to_normal_char(c, NormalizationContext::Kana);
```

Use `bool` only when the parameter's polarity reads clearly at the call site without consulting the function signature (`recursive: true`, `case_sensitive: false` — the parameter name disambiguates by itself). When in doubt, prefer the enum: `to_normal_char(c, true)` doesn't tell a reader what `true` means; the enum form does. The slight extra ceremony at the definition is paid back at every call site.

### 4.5. Position offsets → character positions

Lisp `cl-ppcre` and `subseq` index by **character** (= code point in SBCL). Rust `&str` indexing is by byte. Port-level functions that return or accept positions use **character offsets** to match upstream — fixture replay against Lisp captures works without translation, and Rust callers can use `chars().nth()` / `char_indices()` to slice.

```rust
// consecutive_char_groups.rs — start, end, and returned (s, e) are all char offsets
pub fn consecutive_char_groups(class: CharClass, s: &str, start: usize, end: usize)
    -> Vec<(usize, usize)> { ... }
```

Convert to bytes internally (via `s.char_indices()`) when calling the regex engine; convert back before returning. Add a regression test that pins multi-byte behavior — without it, a future "optimization" to byte offsets passes silently in ASCII-only tests.

### 4.6. `:fresh` flag and in-place mutation → return `String`

Several Lisp functions accept `&key fresh` to control whether they mutate the input string or copy it first. The Rust port **always** allocates a new `String` and returns it. Drop the `fresh` parameter entirely. Document the divergence in the doc-comment.

```rust
// Lisp: (geminate txt &key fresh)  — replaces last char with っ in-place or in a copy
pub fn geminate(txt: &str) -> String { ... }
```

Callers that relied on in-place mutation (rare — most use the return value) need updating when their containing function is ported. That's a port-time problem, not an API problem.

### 4.7. Multi-value returns

Lisp `(values a b)` patterns:
- **Two values where the second is rarely used:** drop the second value. Prefer simple types.
- **`(values match-data score)` (genuine pair):** return a tuple `(MatchData, usize)` or a struct. Don't use `Result` — both values are valid.
- **Optional return + presence flag:** `Option<T>` collapses both.

Document which path was taken if it's not obvious.

### 4.8. Macros

Most Lisp macros in this codebase are either (a) DSL definers that register data into existing globals (`def-simple-suffix`, `def-counter`) — the **data** is already captured in the relevant `*global*`, so the macro itself has nothing to translate; or (b) syntactic helpers (`hash-from-list`) whose call-sites are already directly ported.

For these, create the `_macro` file with a doc-only body explaining the situation and pointing at where the equivalent data/code lives. Don't try to write a Rust macro that mimics the Lisp expansion — that's almost always the wrong tool.

A small minority of macros (~6 per the `reverse/` analysis) genuinely encode logic that needs a Rust translation. Those go in the `_macro` file as a regular function or a `macro_rules!` block, with the doc-comment explaining why one was chosen over the other.

---

## 5. Globals

`defparameter` / `defvar` / `defconstant` ports live in `_star_<name>_star_.rs` (see §2). Three patterns:

### 5.1. Plain literal data

```rust
pub static FOO: &str = "literal";
pub static BAR: &[(KanaClass, &str)] = &[ (KanaClass::A, "あア"), ... ];
```

Use this when the value is a true compile-time constant (no derivation, no other globals depended on, no allocation).

### 5.2. Lazy derivation via `OnceLock`

```rust
use std::sync::OnceLock;

static CACHE: OnceLock<HashMap<KanaClass, KanaClass>> = OnceLock::new();

pub fn dakuten_hash() -> &'static HashMap<KanaClass, KanaClass> {
    CACHE.get_or_init(|| { ... })
}
```

Use this when:
- The value depends on other ported globals (build it from them — don't hand-copy).
- The type isn't `const`-constructible (`HashMap`, `fancy_regex::Regex`, `String`).
- There's a `format!`-style derivation (e.g. `*basic-split-regex*` from four constituents).

`OnceLock` is preferred over `lazy_static!` / `once_cell::Lazy` because it's in `std`. Pin the build output with a regression test against the value the introspector captured — this catches drift in inputs.

### 5.3. Frozen literals (when construction inputs aren't ported yet)

When the upstream builds a global from a function or other globals you can't yet derive in Rust, capture the introspected value as a literal in `_star_<name>_star_.rs`. Doc-comment requirements per §3.4. Add an entry to the staleness ledger in `HANDOFF.md`.

Convert to a derivation as soon as the inputs are ported — don't leave dead literals lying around once the construction logic exists in Rust.

---

## 6. Tests

**Test logic, not data.**

- ✅ Build-loop regressions: "this hash has 173 entries when built from `*all-characters*`" guards the loop, not the data.
- ✅ Integration: "every regex string compiles under fancy-regex".
- ✅ Behavioral pinning for non-obvious cases: char vs byte offsets in `consecutive_char_groups`, the alternation logic in `basic_split`.
- ❌ `assert_eq!(LIST.len(), 87)` against a hand-typed list in the same file — the data **is** the spec; downstream code finds errors in it.
- ❌ Asserting literal contents in iteration order against data declared in the same file.

Tests live in `#[cfg(test)] mod tests { ... }` at the bottom of the port file. Don't make a separate `tests/` directory under `kaniran-core/` — that's reserved for crate-level integration tests (none exist yet).

For frozen-literal globals: a "matches introspected value" test pinning the output to the value `reverse/scripts/introspect.lisp` captured. This is the staleness alarm.

---

## 7. Workflow

1. **Before porting**, run `python3 reverse/scripts/query.py deps <fqn>` to confirm prerequisites are ported. The plan is leaf-up — porting out of order means stub deps you'll have to revisit.
2. **Write the port file** following §1–§6. Add the `pub mod <stem>;` line to `mod.rs`.
3. **Verify**:
   - `cargo check -p kaniran-core` — catches missing `mod` declarations.
   - `cargo test -p kaniran-core` — catches behavioral regressions in the tests you wrote.
   - `python3 reverse/scripts/query.py audit-signatures` — cross-checks each ported `pub fn` against the captured Lisp lambda list (`signatures.json`) and flags arity drift, dropped keywords, missing pub fns, and extra public functions in the same file. **Always runs the full sweep** and **always rewrites `reverse/scripts/divergences.md`** (deterministic, sorted by FQN — diffs cleanly). Use `--only <pkg>` to scope the *stdout* output; the file always reflects the full sweep. Use `--no-write` to suppress the file rewrite (rare).

   **All three must pass before claiming the port is done.** `cargo check` catches the mod declaration, `cargo test` catches behavior, `audit-signatures` catches API-shape drift like the `_with` split that prompted its existence.

   **`reverse/scripts/divergences.md` is committed.** After a port, `git diff reverse/scripts/divergences.md` is the review surface. New entries should be either (a) intentional, citing CONVENTIONS (§4.4 enum collapse, §4.6 dropped `:fresh`, etc.) — commit alongside the port; or (b) a port bug — fix and re-run until the entry disappears.
4. **Mark progress**: `python3 reverse/scripts/query.py mark <fqn>... --status ported`. This rewrites the `status` column of `symbols.csv` in place.
5. **Regenerate the plan**: `python3 reverse/scripts/query.py plan --out reverse/scripts/PORT_PLAN.md`. The plan is byte-deterministic across runs on the same CSVs.
6. **Don't run `python3 reverse/scripts/build_graph.py` casually** — it overwrites `symbols.csv` from the md files and resets every `status` cell to `pending`. Only run it after re-running `introspect.lisp` against an updated upstream, and commit `symbols.csv` first.
7. **Update `HANDOFF.md`** when introducing a new convention, a frozen-literal entry, or a non-trivial divergence. Don't update it for routine ports.

Don't edit `PORT_PLAN.md` by hand. Don't edit upstream `*.lisp` files at the repo root — they're the introspection input, treat as read-only.

---

## 8. Imports and exposure

- Use `super::` paths within a package directory; `crate::` only when crossing packages.
- Pub-export from each port file directly (`pub fn foo`, `pub static FOO`). **Don't** add re-exports in `mod.rs` — every user imports via the canonical path. This keeps the mapping back to the Lisp FQN one-to-one.
- Don't add `pub use` shortcuts unless the Lisp package itself exports the name and a downstream port needs it via the package-level path.

---

## 9. Don'ts (catch-all)

- **No upstream `*.lisp` edits.** They're checked in for introspection input. Read-only.
- **No hand-editing `PORT_PLAN.md` or `symbols.csv`** — use `query.py mark` / `query.py plan`.
- **No backwards-compat shims, deprecated aliases, or `// removed` comments.** This is a fresh port; delete cleanly.
- **No speculative abstractions.** If two ports share five lines, write five lines twice. Wait for a third before extracting.
- **No mocked dependencies in tests** — when tests need state (e.g. compiled regexes, hash tables), use the real ones via `OnceLock`. The tracer / fixture-replay infra in `kani/` exists for the cases where real Lisp results are needed.
- **No `unsafe` without explicit justification** in a `// SAFETY:` comment. None of the current port surface needs it.
- **No `unwrap()` on user-controllable input.** `expect()` with a message is fine for invariants the codebase enforces (e.g. "char_class is in *char-scanners*" — the table covers every `CharClass` variant and a test asserts so). Prefer `?` for plumbing through `Option`/`Result`.
- **No `#[allow(...)]` to silence warnings without a comment** explaining why the warning is wrong here.

---

## 10. When in doubt

- API shape: re-read §4 and grep upstream callers (`grep -n '<name>' *.lisp | grep -v '^<file>.lisp'`) to see how the return value is consumed.
- File path: feed the FQN to `kani::naming::fqn_to_path`.
- Whether to test something: §6. If the test would only catch a hand-typed data mistake, skip it.
- Whether a frozen literal is worth deriving now: if all inputs are ported, derive. If one input is unported, freeze and add to the ledger.
- Anything else: ask. New conventions get added to this file, not invented per port.
