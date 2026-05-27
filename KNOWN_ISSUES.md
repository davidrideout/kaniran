# Known issues

**Scope — read before adding anything.** This document is ONLY for
discrepancies between kaniran and upstream ichiran where the two
snippets of code are *equivalent* and we still cannot explain why the
outputs differ — i.e. the divergence is not in the logic. In practice
that means database row-ordering nondeterminism (no `ORDER BY`, loader
permutation, hash-bucket iteration) and the like.

Do NOT add anything else here. No suspected bugs, no "might be a
problem", no unverified hypotheses, no port TODOs, no ruled-out
investigations. A real logic discrepancy is a port bug — fix it, don't
log it here. If a discrepancy is not verified to be code-equivalent-
but-output-divergent, it does not belong in this file.

Each entry: scope, mechanism, reproduction, current disposition.

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

**Update 2026-05-26.** Option 1 was in fact silently shipped — a
`seq DESC, id DESC` variant landed in `get_kana_forms_star_` on
2026-05-15 (commit `166625b`), bundled into an unrelated port commit
and self-justified with a misapplied CONVENTIONS §4.4 citation. It was
**removed 2026-05-26** after the cli-full e2e audit showed it
net-harmful — see §3. The UNION is back to upstream's no-`ORDER BY`
shape; the 459 get_suffixes cache-tie rows return as expected
divergence.

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

**Disposition.** The suffix subsystem is now ported
(`find_word_with_conj_prop`, `find_word_suffix`, the `def-simple-suffix`
family, `adjoin_word`, `set_word_conjugations`), and the live
segmentation path builds compounds through it. But Rust value-copies
cache rows into compounds — no shared object identity — so **Rust never
leaks**: it produces the clean, unmutated conjugation set. The leak is
therefore not latent; it now surfaces at the **cli-full e2e** layer as
`conj` array-length divergences where the *capture* is narrower than
Rust on a compound's last component. The `get_suffixes` captured-field
mismatches (125 + 1) are the same mechanism one layer down.

Verified 2026-05-26 on 出てけ → いけ: clean single-sentence upstream
(`dict-segment "出てけ"`) and the `get-kana-forms*` cache both assign
word-conjugations `(352337 1181022)` — two ids, identical to Rust. The
cli-full capture renders only `1` (continuative-of-potential), having
dropped the imperative of 行く via a cross-sentence
`(setf word-conjugations)` write onto the shared いけ cache row earlier in
the pool run. The dropped entry, the imperative, is the correct reading
for 出てけ ("get out"), so here the leaked value is a degradation, not a
refinement — matching it would make kaniran's output worse. Romaji and
segmentation are unaffected; only the conjugation annotation differs.

The fidelity choice is no longer gated on landing "before the subsystem
ports" — the subsystem ported and Rust defaults to the clean,
non-leaking, more-correct output. Three options:

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

Current lean: keep the clean output (none of the three). Option 1
(mirror) reproduces upstream's write-through faithfully, but that means
reproducing the leak — nondeterministic, processing-order-dependent, and
(as 出てけ shows) able to drop the correct reading. Options 2/3 would
re-narrow deterministically to match upstream's within-sentence setf
cases, but also suppress readings the clean path keeps. Since the leaked
value is not a spec and is sometimes worse, kaniran keeps the clean
output and the cli-full audit treats these `conj`-length rows as capture
leak-residue, not a port defect. Recorded in the suffix-cache file's
module doc ([`_star_suffix_cache_star_.rs`]) and tracked here.

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

## 3. cli-full e2e: Passive/Potential `prop.type` on `いる`-auxiliary compounds

**Scope.** The cli-full e2e audit (`(jsown:to-json (romanize* text :limit 5))`,
runner `cli_full_test`) shows `conj[N].prop[N].type` (and
`.via[N].prop[N].type`) reading `Passive` where the captured Lisp reads
`Potential` (or the reverse) on `〜ていられ…` / `〜てられ…` constructions.
This is the downstream surfacing §1's "Downstream impact" flagged to
re-check when next-layer audits land. It is **not** a conj-layer bug.

**Mechanism.** `〜てられない` attaches `いる` as an auxiliary through a
`*suffix-cache*` entry keyed by spelling (`られない`, `いられません`,
`いられよう`, …). `いる`'s negative forms are homonyms: its Potential
(e.g. seq 10551851 `いられない`) and Passive (e.g. seq 10235827) share
the same kana but are distinct UNION rows. The §1 last-write-wins
populator keeps whichever the loader iterates last for that key, and
the conj layer then faithfully renders that seq's `prop.type`. Which
reading a spelling key resolves to is therefore the §1 nondeterminism.

Proof it is arbitrary, not meaningful: in a single SBCL process the
cache resolves `られない`→Potential (10551851) but `いられません`→Passive
(10235828) and `いられよう`→Passive (10235857) — structurally identical
homonym ties landing on different readings within one process. The pool
workers that captured the corpus likewise disagree across processes.

**Reproduction.** Subset `cli_full_failures.txt`'s `rust="Passive"
lisp="Potential"` rows (31) into a parquet and run
`cargo run --release --bin cli_full_test -- --path <subset>`:
**31 fail** with the old `ORDER BY u.seq DESC, u.id DESC` in
`get_kana_forms_star_`; **2 fail** without it (verified 2026-05-26).
The 2 residuals are `いられません`/`いられよう` keys whose captures landed
on Passive while Rust's wire order lands on Potential.

**Disposition.** Same as §1 — irreducible upstream loader
nondeterminism; no deterministic `ORDER BY` reconciles all captures
because different spelling keys (and different pool workers) land on
different homonyms. The `ORDER BY` was removed (see §1 Update
2026-05-26); the residual divergences are expected at the e2e layer,
not a per-input specification.

---

## 4. `find-substring-words` bucket order diverges across DB builds

**Scope.** The `find_substring_words` order-sensitive audit
(`find_substring_words_test`, captured 2026-05-26 over the 250k diverse
corpus) shows **33,628 / 536,669 (6.3%)** rows whose `*substring-hash*`
bucket order differs from the captured Lisp, concentrated on 12 common
homonym keys (`いる` 27,805, `字` 2,109, `彼の` 1,939, `氏` 1,120, `モノ`,
`分かり`, `やろ`, `いこう`, `きのう`, `とろ`, `つつまし`, `うんち`).
**100% of the 33,628 are pure reorderings** — identical seq multiset,
zero rows added/dropped/changed; only the relative order of adjacent
homonym tuples differs.

**Mechanism.** `find-substring-words` bulk-fetches each substring's
rows with `(query (:select … :from table :where (:in 'text (:set keys))))`
(`dict.lisp:514-518`) — no `ORDER BY` — then `(push (cons table kt) …)`
(`dict.lisp:517`), which prepends, so each bucket is the reverse of the
fetch order. The Rust port mirrors this (`insert(0, …)`). The fetch
order of an unordered `text = ANY(...)` scan is PostgreSQL physical heap
/ scan order, which is **not identical between the `.103` capture DB and
the local audit DB** for a handful of homonym tuples, even though both
were built from the same JMdict import. Distinct from §1: this is a raw
`query` (both sides read the wire order directly — within one DB they
agree exactly), so the divergence is the physical-placement difference
between the two DBs, not the Postmodern `query-dao` loader permutation.

**Reproduction.** For homonym key `いる` the two DBs return the same 10
rows, last two swapped:

```
local: … 1587780 1322180 1391500
.103:  … 1587780 1391500 1322180
```

Rust's bucket is the exact reverse of the local fetch (the prepend fix,
verified by the `bucket_is_reverse_of_fetch_order` unit test); the Lisp
bucket is the reverse of `.103`'s. Run
`cargo run --release --bin find_substring_words_test -- --path corpus/find_substring_words_2026_05_26/dict/find_substring_words.parquet`
→ pass=503,041 fail=33,628 skipped=0; every failure is a same-multiset
reorder.

**Disposition.** Not a port bug — the bucket content is 100% faithful
and the within-DB order is correct (reverse of that DB's fetch). The
cross-DB order divergence is irreducible: upstream's no-`ORDER BY` query
gives no order contract, so two DB builds can legitimately disagree.
The order is load-bearing downstream (last-iterated homonym wins in
`find-word` / `pair-words-by-conj`), so this is one source feeding the
§3 cli-full Passive/Potential divergence. Pass-rate is not the gate;
the divergence pattern (pure reorderings on homonym keys) is.

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
