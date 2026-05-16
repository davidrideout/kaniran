# Known issues

Open divergences between the Rust transliteration and upstream ichiran
that are mechanism-understood but not yet fixed. Each entry: scope,
mechanism, reproduction, current disposition.

Update when the underlying upstream behavior is replicated, formally
accepted as divergence, or when audit gates are adjusted to account
for it.

---

## 1. `*suffix-cache*` row identity diverges on UNION ties

**Scope.** Cache entries built by `init_suffixes_thread` for seqs whose
`get_kana_forms_star_` UNION returns multiple rows with the same
`text` (e.g. seq=1577980 `いる` produces two `text="いられて"` rows at
ids 538276 and 1717832). The populator's last-write-wins overwrite
picks whichever instance comes last in iteration order, so the cache
key (`られて`, `られなければ`, `られる`, …) holds different `KanaText`
rows in Rust vs upstream.

**Mechanism.** The upstream SQL UNION has no `ORDER BY`. Postgres
resolves it via `HashAggregate`, which emits rows in hash-bucket
iteration order — deterministic per query plan, but not stable across
DB drivers:

- `psql`, sqlx's `query_as`, and Postmodern's raw `query` read the
  wire order directly.
- Postmodern's `query-dao` (upstream's loader, `dict-grammar.lisp:21`)
  accumulates the result via a non-trivial permutation driven by its
  DAO loader's internal hash-table walk. The permutation is stable
  within a single SBCL process but varies across process restarts
  (in-memory allocator / hash sizing state), and is **neither equal
  nor a simple reversal of** the wire order for multi-row UNIONs.

Verified empirically 2026-05-15 by probing 20 seqs × 5 trials against
`.103`: per-trial stability confirmed; per-process variance confirmed;
no algebraic relation between `raw` and `dao` orders for results
larger than ~3 rows.

**Reproduction.** Run
`cargo run --bin get_suffixes_test --release -- --path corpus/extracted_get_suffixes_2026_05_15/dict/get_suffixes.parquet`.
Failure pattern with `ORDER BY u.seq, u.id` (or no ORDER BY):
459 rows of "triple[N].kf field mismatch (rust seq=X, lisp seq=Y)" —
same row count, same seqs paired, every run. Specifically:

- 285 cases at `[2]`, suffix=`られて`: rust seq=11156119, lisp seq=10235833
- 81 cases at `[4]`, suffix=`られなければ`: rust seq=11156125, lisp seq=10235839
- 12 cases at `[2]`, suffix=`られる`: rust seq=10551849, lisp seq=2809790
  (the 居られる dictionary-entry / conjugated-form variant — has
  `best_kanji` difference layered on top)

**Disposition.** No deterministic SQL `ORDER BY` clause can reproduce
upstream's permutation, because the loader behavior isn't a function
of any SQL column. Options for future work, none yet chosen:

1. Pin a stable Rust-side order (`ORDER BY u.seq, u.id`) and mark the
   459 cache-tie rows as expected divergence in the audit runner.
   Behavior is then reproducible Rust-side, but cache row identity
   doesn't match upstream for tied keys.
2. Re-extract `get_suffixes` fixtures from a Rust-driven run and use
   those as the gate going forward. Loses parity with upstream
   captures.
3. Replicate `query-dao`'s permutation. Probably infeasible (in-
   process allocator / hash state).

**Downstream impact.** The chosen cache row identity feeds into
`word_conj_data` for any compound built atop that suffix entry, which
in turn feeds segment scoring. Verified that for the specific seqs
involved (10235833 / 11156119 / 10235839 / 11156125 / 2809790 /
10551849), downstream conj-data is not observably divergent at the
segment-list audits we currently have — but this is empirical, not
guaranteed. Re-check when next-layer audits land.

**Capture stability.** The 459 failing rows are *not* a fixed
specification. `query-dao`'s permutation is a function of in-process
allocator + hash-table sizing state at the time `init_suffixes_thread`
runs. A fresh SBCL worker on the same corpus with the same DB will
produce a *different* parquet, with the same row count of failures
but different specific `lisp seq=` values per row. The captures encode
one realization of upstream's nondeterminism, not a stable contract.
Re-extraction does not converge on a canonical answer.

**Scope beyond `get_kana_forms_star_`.** The Postmodern DAO loader
permutation is a property of `select-dao` / `query-dao` themselves,
not specific to the `get_kana_forms_star_` UNION. **Any** upstream
function that returns DAOs via those two operators inherits the
same nondeterminism. Other functions in the current Rust port that
hit this in their upstream counterpart:

- [`find_word_seq`] — upstream `find-word-seq` is
  `(select-dao table (:and (:= 'text word) (:in 'seq (:set seqs))))`.
  Order of returned rows depends on the loader.
- [`get_kana_form`] — upstream is
  `(car (select-dao 'kana-text (:and (:= 'text text) (:= 'seq seq))))`.
  The `car` takes the loader's *first* row; if multiple rows match
  text+seq, "first" is loader-permutation-dependent. Especially
  load-bearing because callers consume a single row.
- [`get_conj_data`] — upstream is `select-dao` over conjugation
  rows; `&optional from/conj-ids` only constrains the WHERE clause,
  not the order.
- [`find_word_conj_of`] — combines `find-word-seq`'s `select-dao`
  result with a `query-dao` UNION-style branch. Two loader-permuted
  inputs feeding the documented `union :key #'id` semantics.

Audit fixtures for any of these will, like `get_suffixes`, encode
one realization of the loader's permutation. Single-row queries
(`get-kana-form` when text+seq has exactly one match) are unaffected
in practice; multi-row queries are subject to the same capture-
stability caveat as §1 above. Treat divergence-pattern matching, not
pass-rate, as the gate for any audit downstream of these.

[`find_word_seq`]: kaniran-core/src/dict/find_word_seq.rs
[`get_kana_form`]: kaniran-core/src/dict/get_kana_form.rs
[`get_conj_data`]: kaniran-core/src/dict/get_conj_data.rs
[`find_word_conj_of`]: kaniran-core/src/dict/find_word_conj_of.rs

---

## 2. Compound-text `(setf word-conjugations)` aliasing leak

**Scope.** Upstream's `compound-text` instances share `KanaText`
identity with `*suffix-cache*` rows by reference. Mutations on a
compound's `word-conjugations` slot delegate via
`dict.lisp:666` to the compound's last word, which IS the cache
instance — so the cache row's slot gets written. Rust's
`init_suffixes_thread` value-copies `KanaText` rows into the cache;
`adjoin_word` builds compounds from owned values; no aliasing exists
and no mutation can leak in.

**Affected cache entries.** Probed on `.103` 2026-05-15: 206 of 5532
suffix-cache rows have multi-id `get-conj-data` results, any of which
can be mutated to a strict subset by the compound-text setf path.
Examples: `がらせ` (seq=11227972, conj-ids `(1269731 1269748)`),
`させれる` / `させられる` (seq=10086300, `(88705 88709)`), `すぎられる`
(seq=10644397, `(667253 667257)`), `らしくなかった` (seq=10016793,
`(17227 450626)`).

**Mechanism (write-to-subset variant — 125 cases at `[1]`, word `なく`).**

```
DICT-SEGMENT "ジョンとポールが分からなくなった"
 → JOIN-SUBSTRING-WORDS*
 → FIND-WORD-FULL "からなくなった"
 → FIND-WORD-SUFFIX "からなくなった"
 → SUFFIX-ADV "からなく" "なった" #<KANA-TEXT 10374832 なった>
 → FIND-WORD-WITH-CONJ-PROP "からなく" <negative-stem filter>
   → COMPOUND-TEXT (10437514 10648808) からなく
   → (setf (word-conjugations COMPOUND-TEXT) (656991))
     → delegates to (car (last (words …))) = cached #<KANA-TEXT 10648808 なく>
     → cache row's conjugations slot mutated to (656991)
```

The compound-text was built earlier in the same call chain by
`suffix-neg` doing `(adjoin-word verb-stem-kf cached-naku-kf)`. From
this point on, every `get_suffixes` capture for any sentence ending
in `…なく` reads the leaked `(656991)` from the cache row.

For seq=10648808 specifically `(656991)` is the only conj-id
`get_conj_data` would return anyway, so downstream filtering is a
no-op — the leak is captured-field-only here. For the other ~205
multi-id rows, an analogous leak path can produce a strict subset
filter that changes downstream `get_conj_data` / `best_kana_conj` /
`best_kanji_conj` / segment scoring.

**Mechanism (clear-to-nil variant — 1 case at `[2]`, word `愛そうにない`).**

Same compound-text delegation, but the writer is
`find-word-with-conj-prop … :allow-root t` (called from `abbr-nee`,
`dict-grammar.lisp:582`). The `:allow-root` arm fires when `conj-data`
is null, setting `conj-ids` to nil and writing nil into
`word-conjugations`. If the compound's last word is a cache row that
was tagged `:ROOT` by `get_kana_forms_star_`, the cache row's slot
gets cleared back to nil.

Captured at row 316689 / 396209 in the corpus — order-dependent state
accumulation; the specific antecedent sentence wasn't isolated, but
the mechanism is structurally identical to the write-to-subset case.

**Reproduction (write variant).** On `.103`, single SBCL session:

```lisp
(ichiran/conn::with-connection ichiran/conn::*connection*
  (ichiran/dict::init-suffixes t))
;; cache "なく" row → conjugations=NIL  (matches Rust)

(ichiran/conn::with-connection ichiran/conn::*connection*
  (ichiran:romanize* "ジョンとポールが分からなくなった。"))
;; cache "なく" row → conjugations=(656991)  (leaked; Rust still NIL)
```

The post-leak state persists for the life of the SBCL process.

**Disposition.** Latent — currently observable only as captured-field
mismatches in `get_suffixes` audit replay (125 + 1 cases). No live
Rust segmentation path reads from a leaked cache row yet because the
suffix subsystem (`find_word_with_conj_prop`, the `def-simple-suffix`
family) isn't ported.

The fidelity choice must land **before** the suffix subsystem ports,
not after. Three options, none chosen:

1. **Mirror the aliasing.** Cache holds `Arc<RwLock<KanaText>>`,
   `adjoin_word` wraps that shared cell, `(setf word-conjugations)`
   propagates via the cell. Faithful to upstream; inverts
   CONVENTIONS §4.9 for one specific quirk.
2. **Promote to explicit compound state.** Drop the
   "compound's conjugations come from last word" delegation; give
   `CompoundText` its own `conjugations` field, populated by
   `find_word_with_conj_prop`. Diverges from upstream class
   hierarchy; matches observable output.
3. **Side-table memoization.** `HashMap<(seq, id), WordConjugations>`
   updated by `find_word_with_conj_prop`, consulted by
   `word_conj_data` when reading suffix-cache rows. Matches output;
   adds machinery upstream doesn't have.

Option 2 reads cleanest semantically — the leak's actual content is
"the compound knows its own conjugations" — but this is a design
decision pending. Recorded in the suffix-cache file's module doc
([`_star_suffix_cache_star_.rs`]) and tracked here.

**Capture stability.** The captured slot values are functions of the
worker's sentence-processing history, not the `get_suffixes` call
alone. A leaked value persists for the life of the SBCL process; once
sentence N triggers a `(setf word-conjugations cached-row …)` via the
compound delegation, every subsequent `get_suffixes` capture for any
input that lands on that cache key reads the leaked value. Sentence
order in the corpus, sentence order in the worker pool's input
stream, and which specific sentence first triggers the leak all
affect which rows in the parquet show the divergence. A fresh
extraction with the corpus shuffled would shift the divergent rows'
positions; the per-row leaked values would also differ in the rare
cases where multiple distinct leak triggers coexist.

The 126 failing rows are therefore a snapshot of mutation state at
specific call sites, not a per-input specification. Treating them as
a hard pass/fail gate would re-fail on every recapture.

[`_star_suffix_cache_star_.rs`]: kaniran-core/src/dict/_star_suffix_cache_star_.rs

---

## On audit interpretation

The `get_suffixes` parquet captures both deterministic and stateful
properties of upstream behavior in the same trace. Per-row pass/fail
conflates the two. Practical interpretation:

- **Pass count (currently 395,705 / 396,209)** measures the
  algorithm: loop bounds, character-index `subseq_slice` walks,
  shortest-suffix-first ordering, `parse_suffix_val` flattening, cache
  hit/miss discrimination, keyword tagging. These are pure functions
  of input and Rust matches.
- **The 504 failures** are evidence of upstream nondeterminism
  surfacing in the trace, not Rust bugs. They split into 459 row-
  identity divergences (issue 1) and 125 + 1 leaked-slot divergences
  (issue 2). Both are mechanism-understood; neither is a per-input
  specification.

When porting the next layer (suffix subsystem, segment-list, scoring)
and re-extracting fixtures, expect the *same kind* of state-leak
signatures in those parquets too, in different shapes. Plan audit
runners to distinguish "pure-function divergence" (real port bug) from
"state-leak divergence" (upstream's nondeterminism surfacing) up front
— don't bolt the distinction on after the fact.
