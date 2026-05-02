Port the next 11 items from `reverse/scripts/PORT_PLAN.md` — items 51–61, all in `ichiran/characters`. Closes out the package.

| # | Symbol | Kind | Lisp source |
|---|---|---|---|
| 51 | `kanji-prefix` | fn | `characters.lisp:280-284` |
| 52 | `long-vowel-modifier-p` | fn | `characters.lisp:47-53` |
| 53 | `match-diff` | fn | `characters.lisp:326-357` |
| 54 | `mora-length` | fn | `characters.lisp:245-249` |
| 55 | `simplify-ngrams` | fn | `characters.lisp:210-217` |
| 56 | `normalize` | fn | `characters.lisp:224-232` |
| 57 | `rendaku` | fn | `characters.lisp:298-309` |
| 58 | `safe-subseq` | fn | `characters.lisp:359-363` |
| 59 | `sequential-kanji-positions` | fn | `characters.lisp:179-183` |
| 60 | `unrendaku` | fn | `characters.lisp:286-296` |
| 61 | `voice-char` | fn | `characters.lisp:81-83` |

## Read first

1. **[`CONVENTIONS.md`](../../CONVENTIONS.md)** — the canonical rules. Don't relitigate; apply mechanically. Especially relevant for this batch: §3 (doc-comments), §4.1 (predicate → `bool`), §4.2 (gethash-with-default — the `voice-char` example is the *exact* shape), §4.4 (binary `&key` → enum), §4.5 (char positions for offsets), §4.6 (drop `:fresh`, return new `String`), §5.2 (`OnceLock` for compiled regex), §6 (no low-value tests).
2. **[`HANDOFF.md`](./HANDOFF.md)** — current state, frozen-literal ledger, conventions pointers.
3. **[`CLAUDE.md`](../../CLAUDE.md)** — repo orientation, methodology.
4. The path-translation rules and tests in **[`kaniran-core/src/kani/naming.rs`](../../kaniran-core/src/kani/naming.rs)** — single source of truth for "where does this file go." If unsure of a path, feed the FQN through `kani::naming::fqn_to_path`.

Auto-memory carries the project-specific feedback rules (no Lisp internals in chat, no follow-up offers, no low-value tests, verify before claiming, frozen-literal pattern).

## Per-item shapes

Verify each function body against `characters.lisp` directly (line numbers from `symbols.csv` can drift from the checked-in source — see HANDOFF.md gotchas). The 11 split into four groups.

### Group A — trivial accessors / one-liners (5 ports)

Each ≤8 lines of Rust, single CONVENTIONS rule. No tests unless you find a non-obvious behavior worth pinning.

- **`voice-char` (61)** — `(gethash cc *dakuten-hash* cc)`. CONVENTIONS §4.2 spells out this exact case: `pub fn voice_char(cc: KanaClass) -> KanaClass { dakuten_hash().get(&cc).copied().unwrap_or(cc) }`. Both input and output are `KanaClass`, so the §4.2 fall-back-to-self idiom collapses to `unwrap_or` directly — no `Option`-shaped API needed (contrast with `get-char-class`, where input was `char` and output was `KanaClass`, forcing `Option<KanaClass>`).
- **`mora-length` (54)** — count chars not in `"っッぁァぃィぅゥぇェぉォゃャゅュょョー"`. `pub fn mora_length(&str) -> usize`. Char-aware: iterate `s.chars()` and count those not in the set. The set is small enough to inline as a `&str` and call `.contains(c)` per-char, or to materialize as a `&[char]` constant.
- **`kanji-prefix` (51)** — `(or (ppcre:scan-to-strings "^.*<kanji-regex>" word) "")`. Returns the longest prefix of `word` ending in a kanji character, or `""` if no kanji is present. `pub fn kanji_prefix(&str) -> String`. Cache the compiled regex (`OnceLock<Regex>`). The pattern interpolates `KANJI_REGEX` (`^.*[々ヶ〆一-龯]`).
- **`safe-subseq` (58)** — bounds-checked subseq. `pub fn safe_subseq(s: &str, start: usize, end: Option<usize>) -> Option<String>`. Char-position semantics. Validate `start <= len` and (if end is Some) `start <= end <= len`; on failure return `None`. The Lisp's `&optional end` becomes `Option<usize>` — no enum needed; the parameter has only natural meaning.
- **`sequential-kanji-positions` (59)** — `(?=[々一-龯][々一-龯])` lookahead. `pub fn sequential_kanji_positions(word: &str, offset: usize) -> Vec<usize>` returning *char* positions of the second kanji in each adjacent pair, plus `offset`. fancy-regex supports lookahead; cache via `OnceLock`. The Lisp's `&optional (offset 0)` becomes a required `usize` — only one upstream caller and giving them an explicit `0` reads fine.

### Group B — small but slightly more (4 ports)

- **`long-vowel-modifier-p` (52)** — predicate over `(modifier_class, prev_char)`. The Lisp builds a small `(:+a #\A :+i #\I :+u #\U :+e #\E :+o #\O)` plist for the modifier→vowel map, then compares the *last character of the prev-char's class name* (as an upstream Lisp keyword string) to that vowel. The "class name string" is the upstream symbol's printed form (`:KA` → `"KA"`, last char `'A'`). Per CONVENTIONS §1, this is a perfect case for adding a `KanaClass::lisp_name(&self) -> &'static str` method on the sidecar (`kani_kana_class.rs`) — one big match returning `"A"`, `"KA"`, `"SHI"`, …, `"+A"`, `"LONG-VOWEL"`, etc. Then the function reduces to:
  ```rust
  pub fn long_vowel_modifier_p(modifier: KanaClass, prev_char: char) -> bool {
      let vowel = match modifier {
          KanaClass::PlusA => 'A', KanaClass::PlusI => 'I', KanaClass::PlusU => 'U',
          KanaClass::PlusE => 'E', KanaClass::PlusO => 'O',
          _ => return false,
      };
      let Some(class) = get_char_class(prev_char) else { return false };
      class.lisp_name().chars().last() == Some(vowel)
  }
  ```
  The lisp_name method is also useful documentation/debug — landing it once unblocks any future port that wants the upstream symbol form.
- **`rendaku` (57)** — voice the first character. `pub fn rendaku(txt: &str, voicing: Voicing) -> String` where `Voicing { Dakuten, Handakuten }` (CONVENTIONS §4.4 — the binary keyword `:handakuten t` becomes a 2-variant enum, defined inline). Drop `:fresh`, always allocate (CONVENTIONS §4.6). Body: char-class-lookup the first char; pick the voicing hash by enum; if the class has a voiced form, find the input char's position within `KANA_CHARACTERS[class]` and substitute the corresponding char from `KANA_CHARACTERS[voiced]`. Empty input passes through.
- **`unrendaku` (60)** — mirror of `rendaku` but always uses `*undakuten-hash*` — no voicing-flag knob. `pub fn unrendaku(txt: &str) -> String`. Drop `:fresh`. Same shape as `rendaku` minus the enum.
- **`simplify-ngrams` (55)** — alternation-based replace_all. The Lisp builds a regex from the keys (`(:alternation key1 key2 ...)`) and replaces each match with the looked-up value. Generic over the input pair shape so it works with both `&[(&str, &str)]` (`*punctuation-marks*`) and `Vec<(String, String)>` (the runtime `dakuten_join()` result):
  ```rust
  pub fn simplify_ngrams<S, T>(s: &str, map: &[(S, T)]) -> String
  where S: AsRef<str>, T: AsRef<str>,
  ```
  Compile a fresh regex per call (caller-driven map; unbounded keys). Use fancy-regex's `replace_all` with a closure that looks up the matched key in the slice. Order: preserve input order — cl-ppcre's `:alternation` tries left-to-right and the upstream maps (`*punctuation-marks*`, `*dakuten-join*`) avoid prefix collisions. Add a behavioral test pinning `simplify_ngrams("か゛", dakuten_join())` → `"が"` (verifies the runtime derivation feeds correctly into this function).

### Group C — non-trivial (1 port)

- **`match-diff` (53)** — recursive optimal alignment of two strings. Multi-value return: `(values list-of-segments score)` or `nil`. The Rust shape needs to carry both:
  ```rust
  pub enum MatchSegment {
      Equal(String),
      Diff(String, String),  // (s1-piece, s2-piece) — both possibly empty
  }
  pub fn match_diff(s1: &str, s2: &str) -> Option<(Vec<MatchSegment>, usize)>;
  ```
  The Lisp returns:
  - `()` (no values) for the two `(zerop l1)` / `(zerop l2)` base cases — translate to `Some((vec![], 0))` or `None`? Read the Lisp carefully: `(cond ((zerop l1)) ((zerop l2)) (t ...))` — those branches return `nil` (the value of `(zerop l1)` when true). So both empty inputs return `nil`. Translate to `None`.
  - `(values (list s1) l1)` when `s1 == s2` (mismatch is `nil`).
  - `(values (list (list s1 s2)) 0)` when one input has length 1 (and they differ) — single Diff segment, score 0.
  - The `(= m 0)` case: two-level loop over `(i, j)` pairs in `(1..l1, 1..l2)` looking for a matching char and recursing into `(s1[i..], s2[j..])`; keep the candidate with the highest score. Returns the best, prepending `(Diff s1[..i] s2[..j])`. None of the candidates working → returns nil → `None`.
  - The `(= m l1)` and `(= m l2)` cases: prefix matched fully, only the last char differs. Returns segments with one Equal and one Diff.
  - The general case: prefix `s1[..m]` matches; recurse on `(s1[m..], s2[m..])` with score `+m`.

  Char-position semantics throughout (CONVENTIONS §4.5) — the Lisp's `subseq` and `mismatch` are character-based on SBCL strings. Use `chars().count()` for `length` and `s.chars().take(n).collect::<String>()` / `.skip(n)` for slicing. Don't conflate with byte positions when the inputs contain multi-byte CJK.

  Tests: pin a few canonical alignments — equal strings returning `Some((vec![Equal(s.into())], len))`, single-char mismatch returning `Some((vec![Diff(...)], 0))`, a prefix+suffix match producing the expected segment list. Don't try to exhaustively test the search heuristic; one or two reading/word pairs from the upstream test suite are enough.

  Naming: `match-diff` lives in `match_diff.rs` and the segment enum belongs inline (CONVENTIONS §4.3). Don't name the enum `Match` — that collides with `fancy_regex::Match` in any file that imports both.

  The doc-comment must call out two behaviors that aren't obvious from the signature:
  - The "best-match" loop is a naive O(l1·l2 · recursion) search. Fine for the short kana strings the upstream uses; document the complexity.
  - The base cases return `None` for empty input, *not* `Some((vec![], 0))`. The Lisp's bare `(zerop l1)` falls out of the cond returning `nil`; preserve that.

### Group D — none

(Group D is reserved for special cases: dead-but-port-anyway functions and doc-only macros. None remain in characters.)

## Order

Suggested order — minimizes cross-dependency back-tracking:

1. **First: `KanaClass::lisp_name()`** — add the method to `kani_kana_class.rs`. Touches only the sidecar; doesn't itself count as a port. Unblocks 52.
2. **Group A** in any order (51, 54, 58, 59, 61). All independent.
3. **Group B small ones**: 52 (long-vowel-modifier-p; depends on `lisp_name()` and on the existing `get_char_class`), then 60 (unrendaku), then 57 (rendaku) — they share shape; doing unrendaku first lets you write rendaku as a generalization. Then 55 (simplify-ngrams) — independent.
4. **`normalize` (56)** — depends on `simplify-ngrams` (55) and the existing `to_normal_char`. Two-pass implementation: replace abnormal chars, then simplify ngrams.
5. **`match-diff` (53)** — independent; the heaviest item, save for when context is fresh.

## Done means

- 61 of 944 symbols ported. `query.py stats` shows `ichiran/characters 0 61` (i.e. 0 pending, 61 ported).
- `cargo test -p kaniran-core` passes. Test count up by ~5–8 (kanji_prefix's prefix-empty-when-no-kanji edge, simplify-ngrams's dakuten-join roundtrip, 2–3 match-diff alignments, sequential-kanji-positions's lookahead semantics if the lookahead behavior surprised you). Don't add tests just to add them — see CONVENTIONS §6.
- `KanaClass::lisp_name()` lands as a method on the sidecar with full coverage (one arm per variant).
- HANDOFF.md "What got built / changed this session" updated with this batch's table; "Next in the plan" pointed at the next package (likely `ichiran/conn` 27 symbols, or `ichiran/numbers` 13 symbols — whichever wave next opens; check `query.py next`).
- `query.py mark <fqn>... --status ported`, then `query.py plan --out reverse/scripts/PORT_PLAN.md` regenerated.
- This file (`NEXT_PROMPT.md`) regenerated for the next package's first wave.
- Optional but recommended: with `ichiran/characters` complete, this is a natural inflection point to run the `:ichi-trace` tracer against `(ichiran/test:run-all-tests)` and harvest fixtures for the functions ported so far. The fixture-replay infra in `kani::fixture` is already wired. See [`CLAUDE.md`](../../CLAUDE.md) "Tracer / sniffer".

## Don'ts (catch-all reminders — full list in CONVENTIONS §9)

- Don't re-derive `*abnormal-chars*` opportunistically; it's a separate ledger entry and not in this batch.
- Don't hand-edit `PORT_PLAN.md` or `symbols.csv` — `query.py mark` / `query.py plan`.
- Don't run `build_graph.py` casually; it wipes statuses.
- Don't add tests that pin hand-typed data against itself in the same file.
- Don't introduce new conventions ad-hoc — extend CONVENTIONS.md if you genuinely find a gap.
- Don't import `fancy_regex::Match` in a file that also names a local `Match` variant. (`match-diff` segment enum: pick a different name.)
