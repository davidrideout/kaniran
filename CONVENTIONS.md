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

### 3.5. Block-level upstream cites inside function bodies

The header cite (§3.1) anchors the file as a whole. Inside the body, **cite the upstream form** for any block whose Rust shape isn't a transparent restatement of the Lisp at the cited line. Required for:

- **Idiom collapses** — an `:after` / `:around` / `:before` method inlined into a Rust ctor or wrapper, a `&key` keyword collapsed per §4.4, a `(handler-case ...)` becoming a `Result::Err` drop, a multi-value return collapsed to a tuple.
- **Mirrored call sites** — `make-instance`, `apply`, `funcall` over a tabled fn-pointer, `defmethod` dispatch arms — cite the call site in the upstream caller, not just the receiving symbol's definition.
- **Method dispatch arms** — when a `match` arm in a family dispatcher (§4.7) corresponds to a Lisp method override, cite the override's defmethod line.
- **Per-class slot defaults** — a subclass ctor branch applying an `:initform` that diverges from the base; cite the subclass defclass slot line so the divergence is reviewable.

Skip them when:
- The block is a trivial slot copy already covered by the file header (`args.text.clone()`, etc.).
- The Rust does the obvious thing the Lisp does (straight `if` / `match` over a closed enum) and the form name in the header is enough.

**Format:** `// <file>.lisp:<line> (<form-name>)` — symbol/form name is mandatory because line numbers rot under upstream renumbering and the form-name lets `grep` recover the location:

```rust
// dict-counters.lisp:51 (initialize-instance :after counter-text)
let number = parse_number(&number_text)?;
```

```rust
match args.class {
    // dict-counters.lisp:518 (defclass counter-hifumi) — :digit-set initarg
    CounterClass::Hifumi => Counter::Hifumi(CounterHifumi { base, digit_set: args.digit_set.clone() }),
    ...
}
```

This rule applies to inline `//` comments inside function bodies, not to the module `//!` header (which §3.1 already governs). The two cites are complementary: header pins the symbol, block cites pin the shape decisions.

---

## 4. Translating Lisp shapes to Rust APIs

The Lisp uses idioms (multi-value returns, plist keywords, in-place mutation, tagged cons cells) that don't translate 1:1 to idiomatic Rust. Codified decisions follow. **Apply these mechanically — don't relitigate them per file.**

### 4.1. Faithful return types — never collapse to `bool` at port time

Port the upstream return type as-is. If the upstream returns a position-or-nothing, the Rust port returns `Option<usize>`. If it returns the matched substring, the port returns `Option<String>`. Predicate callers write `result.is_some()` at the callsite — the few extra characters are the cost of a lossless API.

```rust
// Upstream returns a position or nothing.
// Port preserves that, even if every visible caller uses it as a predicate today.
pub fn test_word(word: &str, char_class: CharClass) -> Option<usize> {
    char_scanner(char_class).find(word).map(|m| m.start())
}

// Callers — predicate use is explicit, position is still available:
if test_word(w, CharClass::Kana).is_some() { ... }
```

**Do not collapse to `bool` based on caller analysis at port time.** The graph is being ported leaf-up — most callers are still in upstream and can't be verified to be predicate-only with any confidence. A wrong collapse silently drops data, breaks fixture replay (the captured value no longer compares directly), and forces a cross-cutting refactor when a future caller needs the dropped data.

**Deferred collapse is allowed once the graph is closed.** When every caller of a function is itself ported into Rust, run `rg '<fn_name>\(' kaniran-core/src/` against the Rust tree. If every callsite uses `.is_some()` (or equivalent truthiness check), open a single PR that changes the signature to `bool` and drops the `.is_some()` calls in the same commit. Cite this section in the commit and let `audit-signatures` flag the divergence in `divergences.md` for review.

The asymmetry is the point: recovering from a faithful port that turned out to be over-typed is a local refactor (one signature, finite verified callsites). Recovering from a premature collapse means widening the type back and touching every callsite that was written against the lossy shape.

### 4.2. Lookup with input as fallback

The Lisp idiom `(gethash k h k)` returns `k` when the key is missing — useful so the caller can chain. The Rust port returns the value type (not `Option<T>`) and inlines the fallback with `.unwrap_or(k)`:

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

### 4.6. Macros

Most Lisp macros in this codebase are either (a) DSL definers that register data into existing globals (`def-simple-suffix`, `def-counter`, `defsplit`) — the **data** is already captured in the relevant `*global*` (or collapsed into a static dispatcher), so the macro itself has nothing to translate; or (b) syntactic helpers (`hash-from-list`) whose call-sites are already directly ported.

**For these, mark the FQN `skip` with a reason** via `query.py mark <fqn> --status skip --reason "..."`. The reason should name where the data/code lives (the populated global, the dispatcher, the per-callsite ports). Do **not** create a doc-only `_macro` file — a no-op file is project clutter, not a port. The skip reason is the bookkeeping; it surfaces in `PORT_PLAN.md` next to the symbol.

A small minority of macros (~6 per the `reverse/` analysis) genuinely encode logic that needs a Rust translation. Those go in an `_macro` file as a regular function or a `macro_rules!` block, with the doc-comment explaining why one was chosen over the other. The `_macro` filename suffix is reserved for that case.

Don't try to write a Rust macro that mimics a Lisp expansion just to have something to point at — almost always the wrong tool.

### 4.7. Class hierarchies

Several Lisp packages — most prominently `ichiran/dict` — use CLOS class hierarchies with method dispatch. The `counter-text` family is the worked example: a base class (`counter-text`), 10 subclasses (`number-text`, `counter-tsu`, `counter-hifumi`, etc.), generic functions (`get-kana`, `verify`, `value-string`, `counter-join`) with method overrides per subclass, and `:around` method combination on the base.

Port these as **per-subclass newtype + sub-enum dispatcher**. Concretely:

- **One file per Lisp class.** The base (`counter-text`) and each subclass (`counter-tsu`, etc.) get their own `<name>_class.rs` per §1, each defining a `pub struct`. Bare subclasses with no added slots are newtypes around the base struct (`pub struct CounterTsu(pub CounterText)`); subclasses that add slots get a named struct (`pub struct CounterHifumi { pub base: CounterText, pub digit_set: Vec<i32> }`).
- **Methods live on the subclass struct.** A method that overrides for `counter-tsu` lands in `counter_tsu_class.rs` as `impl CounterTsu { pub fn get_kana(&self) -> String { ... } }`, not in the base file's match block. This mirrors the Lisp shape "method defined ON the subclass" and keeps each class self-contained.
- **A sibling enum in the base file dispatches.** `counter_text_class.rs` defines `pub enum Counter { Base(CounterText), Tsu(CounterTsu), Hifumi(CounterHifumi), ... }` with one variant per subclass. Per-generic dispatchers (`Counter::get_kana`, `Counter::verify`, etc.) match on `self` and delegate to the variant's own method, then apply any base-class `:around` wrapping (e.g. `counter-text`'s `:around` appending the `suffix` slot to `get-kana`).
- **Per-class slot defaults are constructor responsibility.** Lisp `:initform` overrides on a subclass slot (e.g. `counter-days-kun` defaulting `allowed` to a specific list) become defaults applied by that subclass's constructor — they're not visible from the struct definition. Doc-comment them on the subclass file so the future constructor port doesn't miss them.
- **Wider cross-family generics live on a higher dispatcher.** When a generic dispatches across multiple class families (`get-kana` on simple-text, proxy-text, compound-text, counter-text), the top-level enum (`Word`) wraps each family's sub-enum as one variant and its dispatcher delegates to the family's dispatcher. Each family handles its own `:around` internally; the top-level dispatcher does not stack additional wrappers.

Why not a single tagged-enum `CounterText { kind: CounterKind, ... }` with one match per generic? Smaller total code, but: (a) collapses 11 named Lisp classes into anonymous enum variants, breaking §1's per-symbol-file principle; (b) lumps every subclass's behavior into giant match blocks, hurting locality; (c) per-class slot-default overrides become conditional logic instead of a subclass-owned constructor; (d) subclasses with extra slots (counter-hifumi's `digit_set`) become asymmetric variant payloads.

Why not `trait + Box<dyn>`? Most literally faithful to CLOS dispatch, but costs a heap allocation and indirect call per value. Tokenization constructs tens of thousands of these per query — unacceptable. Static enum dispatch gives the same structural shape without the runtime cost.

### 4.8. Ctx-injection (database / shared cache access)

Lisp reaches Postgres through `postmodern:*connection*`, a dynamic special variable that `with-connection` rebinds for the active call tree. The Rust port replaces that with explicit injection: every fn that touches the database — or reads a global cache populated from the database — takes `&KaniranContext` as its first parameter.

```rust
pub async fn get_counter_ids(ctx: &KaniranContext) -> Result<Vec<i32>, sqlx::Error> { ... }
pub fn no_conj_data(ctx: &KaniranContext, seq: i32) -> bool { ... }
```

Rules:

- **First parameter, named `ctx`.** Order is `ctx, <verbatim-Lisp-args>`. Don't insert `ctx` mid-list, don't rename it.
- **`&KaniranContext`** — borrowed, not owned, not `&mut`. The context is constructed once via `KaniranContext::from_url` returning `Arc<Self>`; downstream calls borrow.
- **Async iff the body awaits sqlx.** A fn that only reads a populated cache field on `ctx` (e.g. `no_conj_data` reading `ctx.no_conj_data: HashSet<i32>`) stays synchronous. Touching `ctx.pool` makes it `async`.
- **Doc-comment cites the divergence.** Canonical wording, copy verbatim and substitute the Lisp arglist:

  > Diverges from the upstream lambda list `<lisp>` only by taking `&KaniranContext` for the database handle, replacing the upstream dynamic `*connection*` per [`crate::conn::kani_context`].

  When ctx-injection coexists with other shape changes (a `&key` keyword collapsed per §4.4, an `&optional` dropped per §4.6), describe the full shape change — don't paste the canonical wording and leave the rest unmentioned.
- **`audit-signatures` will flag ctx-injection as arity drift** (Rust arity is +1 against Lisp). Entries in `divergences.md` matching the form `arity N+1 ≠ Lisp N (req=N, opt=0, keys=[])` against a fn whose Rust signature starts with `ctx: &KaniranContext` are this convention. Commit them as-is; the visible drift is the audit's record that the convention applied.

### 4.9. Prefer references over clones

Almost everything in this port is read-only at the callsite: captured DAO fields (`.text`, `.kana`, `.seq`), `&KaniranContext`, dictionary entries, regex matches, projector output. Default to `&T` parameters, `&str` over `String`, `&[T]` over `Vec<T>`, and return references / `Cow<'_, str>` when the caller doesn't need ownership.

`.clone()` is justified only when:
- the value must outlive its source (caching, returning from a fn, storing in a struct field),
- ownership must cross a task boundary that requires `'static` (`tokio::spawn`, `JoinSet`); prefer borrow-friendly concurrency (`futures::stream::buffer_unordered`) to avoid the clone in the first place,
- the operation mutates and the original is still needed downstream,
- a sort / comparator key needs an owned value (and even then, try `cmp_by_key` with a borrowed key).

`String::clone`, `Vec::clone`, and `.to_string()` are not free. Audit replay and segmenter scoring loops process millions of rows; each elided clone matters end-to-end. Cloning to silence the borrow checker is a smell — re-examine the lifetimes first.

When porting a Lisp fn that takes a string, the Rust signature should be `&str`, not `String`. When porting one that returns a slot value owned by a longer-lived struct, return `&T` or `Cow<'a, T>`. The grandfather rule: if you wrote `arg.clone()` or `value.to_string()` and the borrow would have worked, delete the clone.

### 4.10. CL dynamic specials with caller-scoped rebind → ctx slot + `with_*`

Some Lisp specials aren't process-wide config — they're per-call-tree state that callers rebind via `(let ((*x* v)) …)`. Examples in ichiran: `*disable-hints*` (recursion guard around the get-kana `:around`), `*substring-hash*` (lookup cache populated by `find-word-full` for its nested-find subtree), `*suffix-map-temp*` / `*suffix-next-end*` (state threaded through suffix expansion), `*split-map*` (rebound at the splits entry).

These get a **field on `KaniranContext`** plus a **`with_<name>(self, v) -> Self`** helper. Rebind sites construct a sibling ctx and pass `&ctx2` downstream:

```rust
// rebind site — mirrors `(let ((*disable-hints* t)) (get-hint obj))`
let ctx2 = ctx.with_disable_hints(true);
get_hint(&ctx2, &wrapped).await?

// read site — mirrors `(when *disable-hints* …)`
if !ctx.disable_hints && !self.hintedp() { … }
```

Rules:

- **Don't use `tokio::task_local!` or `thread_local!`.** Both drop the binding silently at parallel-worker handoff: `thread_local!` across `.await` on multi-worker tokio, `tokio::task_local!` across `tokio::spawn`, and either across `rayon::scope` / `par_iter`. The ctx-slot pattern propagates by reference and is enforced by the borrow checker — crossing a parallel boundary requires capturing `&ctx2` into the closure, making propagation visible at the spawn site. Survives the planned async→sync (rkyv) and async→rayon migrations with no rewrite.
- **Cache fields adjacent to the slot get `Arc<…>`.** `KaniranContext` derives `Clone` so `with_*` is a cheap field copy; the existing `HashSet`/`HashMap` cache fields are `Arc`-wrapped so the clone is one atomic increment per field, not a deep copy. New caches added to ctx follow.
- **Default mirrors the upstream initform.** `(defvar *x* nil)` → `disable_hints: false` / `substring_hash: None` in `from_url`'s initializer. `with_*` returns to that default when callers exit the rebound scope (because the rebound `ctx2` is just a local that drops).
- **Read at the use site, not in every signature.** The point of moving the special onto ctx is dropping it from parameter lists. Only the function that actually consults the value reads `ctx.<field>`; intermediate functions just thread `&ctx` like any other shared state.
- **Doc-comment cites the rebind.** In the divergence block of the consulting function, name the rebind site and the upstream `let` form:

  > Mirrors the upstream `(let ((*disable-hints* t)) …)` at `dict.lisp:82` via [`KaniranContext::with_disable_hints`].

- **The `_star_<name>_star_.rs` file is doc + type only.** No live value lives there; the actual storage is on `KaniranContext`. The file holds the value type (e.g. `pub type SubstringHash = HashMap<…>`) plus a module-doc pointing at the ctx slot, mirroring how `*connection*`'s "port" is `KaniranContext::pool` with no `_star_connection_star_.rs` file at all (a defparameter with no non-trivial value type can skip the file entirely; the FQN gets marked `ported` with reason citing this section).

Contrast with §4.8: §4.8 covers `*connection*` (process-wide, never rebound; a plain field on ctx is enough). §4.10 covers specials that have a meaningful `let`-rebind in upstream control flow — those need the `with_*` helper.

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

### 5.4. Caches: never ship an empty-map stub

A cache port (`*counter-cache*`, `*suffix-cache*`, `*no-conj-data*`, etc.) **must** include its populator. Shipping a `OnceLock<HashMap>` whose `get_or_init(HashMap::new)` returns an always-empty map is **not** a port — it compiles, callers see "nothing in the cache" forever, the system is silently broken on every code path that depends on the cache.

If the populator isn't ready (anonymous `defcache` body that needs a hand-written method, or a named symbol scheduled later in the plan), leave the cache global at status `pending` or `wip`. Don't conflate "compiles" with "ported." A cache global is ported only when its populator runs and the map is non-empty under realistic input.

The Lisp populator routes for the existing cache globals are tracked alongside their wave numbers in HANDOFF.md.

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

   **`reverse/scripts/divergences.md` is committed.** After a port, `git diff reverse/scripts/divergences.md` is the review surface. New entries should be either (a) intentional, citing CONVENTIONS (e.g. §4.4 enum collapse) — commit alongside the port; or (b) a port bug — fix and re-run until the entry disappears.
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
- **No single-letter variable names** outside simple iterator-chain closures. Bindings created with `let`, function parameters, and destructured tuple slots must be descriptive (`actual_id`, `expected_row`, `kanji_text`) — not `a` / `e` / `k`. The exception is one-shot closure arguments in comprehensions where the type and role are obvious from the call site (`v.iter().map(|row| row.seq)` or `vec.sort_by_key(|kt| kt.text.clone())`); even there, a meaningful name is preferred when the closure body is more than a single field access. The rule exists because audit / port code is read more than written, and `a == e` doesn't tell a reviewer which side is the Rust value and which is the captured Lisp value.

---

## 10. When in doubt

- API shape: re-read §4 and grep upstream callers (`grep -n '<name>' *.lisp | grep -v '^<file>.lisp'`) to see how the return value is consumed.
- File path: feed the FQN to `kani::naming::fqn_to_path`.
- Whether to test something: §6. If the test would only catch a hand-typed data mistake, skip it.
- Whether a frozen literal is worth deriving now: if all inputs are ported, derive. If one input is unported, freeze and add to the ledger.
- Anything else: ask. New conventions get added to this file, not invented per port.
