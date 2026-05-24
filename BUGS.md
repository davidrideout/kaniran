# BUGS.md

Investigation findings, verdicts, and fixes. Consolidates the cycle-484
troubleshooting (formerly `484_investigation.md` / `484_investigation2.md`)
and the 2026-05-23 resolution session.

**Relationship to `KNOWN_ISSUES.md`.** That file is the *strict* ledger of
accepted divergences where the code is equivalent but outputs still differ
(database row-ordering, identity aliasing). This file is the broader
investigation record: what was found, the verdict (real bug / benign /
audit-replay artifact), what was fixed, and what is still open. Items that
are accepted output-divergences point to the relevant `KNOWN_ISSUES.md`
section instead of re-deriving the mechanism.

---

## TL;DR

The `find_word_suffix` audit (177,910,861 rows) had 1,359 failures
(99.9992% pass). All are now explained; **none are port-logic bugs**:

| Cluster | Count | Verdict |
|---|---|---|
| naku `conjugations` slot | ~1,121 | Benign — slot differs, output identical (verified) |
| `たら` result-count | ~95 | Audit-replay artifact — uncaptured dynamic context |
| SQL row-ordering (てくれ/いられ/らしい) | ~234 | Benign — equivalent row, identical render (`KNOWN_ISSUES.md` §1) |
| `hintedp` | ~121 | Not a divergence — correctly ported; mislabel of the row-ordering rows |

Separately, the resolution session found and fixed **one real bug** in the
capture tooling (closure `score-mod` projection, §2 below) and closed the
sampling-gap coverage (§3).

---

## 1. `find_word_suffix` audit failures (cycle 484)

Corpus: `extracted_find_word_layer_2026_05_21_dedup/dict/find_word_suffix_idxs.parquet`,
177,909,502 pass / 1,359 fail / 0 skipped.

### 1a. naku `conjugations` slot — BENIGN (verified output-identical)

**Symptom.** A compound's last word is the なく suffix row (seq 10648808);
Rust has `conjugations=None`, upstream has `Some([656991])`.

**Mechanism.** Upstream lazily mutates the shared cached row through the
compound-text `(setf word-conjugations)` delegation (`dict.lisp:666-667`);
Rust holds owned cache values and clones into the compound, so the write
never reaches the cache. This is the aliasing leak documented in
`KNOWN_ISSUES.md` §2 (which also lists the three pending fidelity options:
`Arc<RwLock>`, explicit compound state, or side-table memoization).

**Verdict — no observable output impact (verified 2026-05-23 on the ichiran host):**
seq 10648808 has exactly **one** conjugation in the DB (id 656991), so every
consumer of the slot is identical for `None` ("all") vs `Some([656991])`
("that one"):
- `get-conj-data(10648808, nil|‌(656991), "なく")` → identical single
  conj-data (`[adj-i] Adverbial`, from 1529520 ない).
- `calc-score` of the なく-ending compound → identical (score 871,
  common NIL both ways): the `conj-only` gate (`dict.lisp:804`) flips, but
  `(common なく)` is already NULL and the entry is not a dictionary root,
  so `common` and `root-p` are unchanged.
- `best-kana-conj` is never reached (kana-only; errors on a kana row).
- `pair-words-by-conj` consumes the slot only inside `suffix-rashii`, not
  this compound path.

The structural fidelity fix (`KNOWN_ISSUES.md` §2 options) is therefore
**not required for output parity** for naku; it would only matter for a
multi-id suffix row that gets filtered to a strict subset (the ~205 other
multi-id rows in `KNOWN_ISSUES.md` §2 remain latent).

### 1b. `たら` result-count — AUDIT-REPLAY ARTIFACT (not a bug)

**Symptom.** For `なくなったら` (idx 17096, sentence
`…しきれなくなったらしく…`) the audit replay returns 3 compounds; the
captured upstream result is 0. ~86 of the 95 result-count failures are
`たら`-tails.

**Mechanism.** `find_word_suffix` reads two dynamic specials —
`*suffix-map-temp*` (the full-sentence suffix map) and `*suffix-next-end*`
(the current end position) — that are **not** function arguments, so the
tracer never captured them. During real segmentation `なくなったら` is
called twice:

| call | `suffix_next_end` | suffixes visible | result |
|---|---|---|---|
| top-level (window `[12:18]`) | 18 | `ら`,`たら`,`ったら`,`なったら` | 3 |
| nested (parent suffix decremented the end) | 17 | `た`,`った`,`なった` | 0 |

Both rows are captured. The audit replays with `suffix_map_temp = None`,
which routes through `get_suffixes(word)` — that enumerates the word's own
suffixes (`…なったら`, productive `:adv`/NARU → 3) regardless of position.
The nested row's true answer was 0 (the map at position 17 ends at `た`,
not `ら`). So the replay returns 3 where the fixture says 0.

Verified on the ichiran host: map@17 → 0, map@18 → 3, `get_suffixes` path → 3.

**Verdict.** Production is correct — `find_word_suffix.rs:95-103` takes the
map path and decrements `suffix_next_end` (`:157`); `join_substring_words*`
binds both (`join_substring_words_star_.rs:130-131`). Only the isolated
replay, which builds an empty context, can't reproduce the state. A faithful
audit would require re-extracting with `suffix_next_end` + the sentence
recorded. See also: `find-word-info` is a second top-level setter of these
specials (`dict.lisp:1851`) and must set them when it is ported.

**Untested branch.** The map path of `find_word_suffix` has no unit-test or
audit coverage (every test + the audit run with `suffix_map_temp = None`).
Pin it with a map-bound unit test (`word="なくなったら"`, next-end=17 → 0,
next-end=18 → 3). See §4.

### 1c. SQL row-ordering (てくれ / いられ / らしい) — BENIGN

~234 rows where Rust and upstream pick a different but equivalent dictionary
row for the same surface (e.g. いられ seq 10235873 vs 11156143). Both rows
are valid; `romanize` output is byte-identical; only an internal `via`
provenance id differs. This is the DB row-ordering nondeterminism in
`KNOWN_ISSUES.md` §1 (no `ORDER BY` on the underlying SELECTs; the upstream
Postmodern loader permutation is not reproducible from any SQL column).

### 1d. `hintedp` — NOT A DIVERGENCE (correctly ported)

`hintedp=true` has a single writer in all of upstream: the abbr-suffix
proxy construction (`dict-grammar.lisp:574`); there is no `setf`. Rust sets
it on all three proxy arms (`def_abbr_suffix_macro.rs:98/109/120`). Verified
on the ichiran host: abbr proxies are `hintedp=T` (source `NIL`) on both sides; the
てくれ/いられ compounds are `hintedp=NIL` on every word on both sides. The
"121 hintedp-drift" rows in the original notes are the §1c row-ordering
cluster surfacing `hintedp` as the first differing token, not a real
`hintedp` difference.

---

## 2. Projector closure `score-mod` — REAL BUG, FIXED 2026-05-23

**Symptom.** Capturing `suffix-sou`/`suffix-kudasai`/`suffix-desu`/
`suffix-desho` over productive input produced **0 captures** (all
*skipped*). This is also why the diverse_250k corpus recorded these four as
"all-empty": their productive results were silently dropped, not absent.

**Mechanism.** These four have `:score (constantly N)`, so their compound's
`score-mod` slot holds a **closure**. The JSON projector
(`ichiran-extractor/projectors_json.lisp`) had no `function` clause, so the
closure fell through to identity and `encode-json` errored → the whole
capture was gated out as a skip.

**Fix (two additive changes, matching pre-existing TODOs):**
- `projectors_json.lisp`: added `flatten-to-json ((v function))` →
  `(:obj ("kind" . "constantly") ("value" . (funcall v 0)))`. Emits
  `"score_mod":{"kind":"constantly","value":N}`.
- `kaniran-core/audit/common/mod.rs` `parse_captured_score_mod`: added the
  `{"kind":"constantly","value":N}` → `ScoreMod::Constant(N)` arm, and made
  the `Array` (Stack) arm recurse so a constantly can nest.

**Status.** Fixed, deployed, and validated end-to-end — see §3 (the four now
capture and audit 100%). The `ScoreMod::Constant` path, previously a
documented "audit blackout," is now exercised.

---

## 3. Sampling gaps (not bugs) — closed 2026-05-23

Five functions captured zero non-empty results in diverse_250k
(`suffix-desho/desu/kudasai/sou`, `abbr-meba`), plus the thin 〜ば
contraction family (`abbr-keba/geba/seba/teba/neba/reba`). These are not
dead code — unit tests and live-segmenter probes show them productive given
clean input; the corpus just never sampled a productive call. (Note: the
four `constantly`-score ones *were* reached but their captures were dropped
by §2.)

**Resolution.** 2,342 targeted sentences appended to
`corpus/diverse_250k_2026_05_09.parquet` (rows 250000–252341,
`source=gap_*`). Re-extracted only those rows on the 11 functions
(`--skip 250000`), deduped (idx-preserving) to
`corpus/gap_suffixes_2026_05_23/<fn>.parquet`, and audited:
**6,400/6,400 pass, 0 fail, 0 skipped** (sou 821, desho 3359, desu 1378,
kudasai 687, abbr-{geba 14, keba 28, meba 20, neba 22, reba 36, seba 20,
teba 15}).

**Still open (coverage, not bugs):**
- `abbr-ii` — its contraction surface (`find-word-full(root + "いい")`) was
  not pinned down; needs a probe before a gap corpus can be built.
- `abbr-beba` — no fixtures exist yet; out of scope until extracted.

---

## 4. Open follow-ups

- **`find-word-info` must set the suffix-map context when ported.** It is
  the second top-level setter of `*suffix-map-temp*` / `*suffix-next-end*`
  (`dict.lisp:1851`); mirror `join_substring_words_star_.rs:130-131`
  (`get_suffix_map` + `with_suffix_map_temp` + `with_suffix_next_end`).
  `find_word_suffix` is the sole reader; with an empty context it silently
  falls back to `get_suffixes` and takes the wrong suffix set for nested
  calls (the §1b mechanism).
- **Add a map-path unit test to `find_word_suffix.rs`** (see §1b) — the
  production-relevant branch currently has no coverage.
- **`KNOWN_ISSUES.md` §2 multi-id rows remain latent** — ~205 suffix-cache
  rows have multi-id `get-conj-data` results where the aliasing leak could
  filter to a strict subset and change output. Only naku (single-id) is
  verified benign; re-check the others when next-layer audits land.

---

## Appendix — reproduction

**One-shot upstream eval on the ichiran host** (read-only; UTF-8 needs a real temp
file, `/dev/stdin` chokes on the form-tracking stream):

```sh
ssh $REMOTE_HOST 'cat > /tmp/probe.lisp' <<'LISP'
(handler-case
  (postmodern:with-connection ichiran/conn:*connection*
    ...) ; ichiran/dict:: forms; bind ichiran/dict::*suffix-map-temp* etc.
  (error (e) (format t "ERROR: ~a~%" e)))
LISP
ssh $REMOTE_HOST 'LANG=en_US.UTF-8 sbcl --core $REMOTE_STORAGE/ichiran.core \
  --noinform --non-interactive --load /tmp/probe.lisp'
```

**Sample idxs** (replay via the corpus row `number`):
- 1a naku: idx 33998 `用が足りなくなる`, 58801 `気をつけなくっちゃ`
- 1b たら: idx 17096 `…しきれなくなったらしく…`
- 1c row-ordering: idx 19743 `来てくれた`, 182747 `忘れていられる`
