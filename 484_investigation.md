# find-word-suffix audit — cycle-484 failure investigation

## Audit summary

Corpus: 177,910,861 rows (`extracted_find_word_layer_2026_05_21_dedup/dict/find_word_suffix_idxs.parquet`, re-deduped with `ARRAY_AGG(idx ORDER BY idx) AS idxs` for source-sentence replay).

Result: **177,909,502 pass / 1,359 fail / 0 skipped** (99.9992%).

Wall: 28m43s with `ASYNC_CONCURRENCY=50`, pool `max_connections=100`. (Previous run at 32/100 was 28m25s — the audit is parquet-decode + JSON-parse + fingerprint bound, not DB-bound.)

## Failure classes

| Class | Count | % |
|---|---|---|
| naku classic (Rust `conjugations=None`, Lisp `Some([656991])`) | 969 | 71% |
| Result count mismatch | 95 | 7% |
| Other drift (same compound count, no naku slot diff) | 295 | 22% |
| **TOTAL** | **1,359** | |

The previous session's "all 1,359 are naku" was wrong on **390 failures (29%)**. Sampling 19/1,359 happened to land entirely in the dominant naku cluster.

## Nature of the divergence (first-divergence field)

Not all failures are about `conjugations`. Field breakdown across all 1,359:

| First-divergence field | Count | % |
|---|---|---|
| `conjugations` | 1,121 | 82% |
| `hintedp` | 121 | 9% |
| (N compounds differs) | 95 | 7% |
| `primary` | 15 | 1% |
| `kana` | 5 | <1% |
| `text` | 2 | <1% |

**Three real failure mechanisms, not one:**

1. **conjugations slot drift (1,121)** — Lisp's lazy mutation of cache entries via aliased identity. naku dominates but spreads to other cached suffixes.
2. **hintedp slot drift (121, all in Class 3)** — distinct subsystem. hintedp is a per-text flag set during hint processing for kana rendering. Memory `project_hint_subsystem_state_2026_05_13.md` notes the hint port already had 6 follow-up fix commits.
3. **Suffix dispatch logic (95 + 22 = 117)** — wrong number of compounds OR wrong primary/kana/text. Logic-level bug in a suffix-fn (suffix-to / others), independent of slot state.

Per-class first-divergence:

- **Class 1 (969)**: 967 conjugations + 2 hintedp — monolithic.
- **Class 2 (95)**: all result-count.
- **Class 3 (295)**: 154 conjugations (52%) + 119 hintedp (40%) + 22 mixed (primary/kana/text).

## Cycle-484 wave attribution

Failures map to ~95% identifiable cycle-484 ports:

| Wave | Suffix-fn | Failure word-tail | Count |
|---|---|---|---|
| 2 | suffix-neg | なく | 969 |
| 2 | suffix-neg / nai-abbr | ない | 33 |
| 3 | suffix-te+space (kureru/morau/itadaku) | てくれ + くれ | 116 |
| 3 | suffix-teiru | いられ + られ | 62 |
| 5 (likely) | suffix-to | たら | 89 (86 result-count + 3 drift) |
| 6 | suffix-rashii | らしい | 14 |
| 4 | suffix-adv (naru) | なる | 2 |
| (various / scoring) | mixed | した, ます, etc. | 7 |
| — | uncategorized tails | ? | ~67 |

## Root cause patterns

### Class 1 — naku classic (969)

Documented in detail in `MEMORY.md` → `project_find_word_suffix_session_2026_05_22.md`. Mechanism: Lisp's `find-word-with-conj-prop` lazily mutates the naku cache entry's `conjugations` slot through aliased identity (`(setf (word-conjugations (car (last (words …)))))` at `dict.lisp:667`). Rust's suffix cache holds owned `KanaText` values; `find_word_suffix` clones into the compound. The Rust setter at `set_word_conjugations.rs:21` lands on the compound's clone, never reaches the cache.

The eager-populate experiment (revert in this session) only inverts the failure class — captures from before Lisp's cache warmed remain divergent in the other direction.

Real fix paths:
- **Identity-aliasing structural change** — switch suffix cache to `Arc<RwLock<KanaText>>` and ripple through ~100 files that match `KaniWordDispatchEnum::Kana`. Eliminates Class 1.
- **Accept as Lisp lazy-timing artifact** — document the gap and move on.

### Class 2 — result count mismatch (95)

Overwhelmingly concentrated on `たら` (86 of 95 = 91%). Rust and Lisp return entirely different numbers of compounds for words ending in `たら`. Sample: `なくなったら` (idx 17096) — Rust returns 3 compounds, Lisp returns 0.

Probably a bug in **suffix-to** (wave 5) — the `:to` key in the suffix list dispatches to that handler. The "Rust returns extra compounds" shape suggests Rust isn't applying some gate that Lisp does (offset check, conjugation predicate, etc.).

The 9 `?`-tail result-count failures have shapes like `じら`, `夫ら` (raw kana-text rows, no obvious suffix). May be a `:ra` suffix issue (wave 4) or different mechanism entirely.

### Class 3 — other drift (295)

Three sub-clusters, each likely a distinct port bug:

- **てくれ / くれ (116)** — suffix-te+space (wave 3). Loaded via `(load-conjs :te+space 1269130 :kureru)` which populates conjugations at load time, so this is NOT the naku pattern. Likely a different mutation or selection issue inside suffix-te+space.
- **いられ / られ (78)** — suffix-teiru (wave 3). Earlier dig showed Rust picks seq=10235873 vs Lisp seq=11156143 for the same text "いられ". Both are valid DB rows; the cache loader's "last-write-wins" stores a different kf depending on `get_kana_forms` query order. Probably an ORDER BY discrepancy between Rust's sqlx query and Lisp's `query-dao`.
- **らしい (14)** — suffix-rashii (wave 6). Smaller cluster; needs sample-by-sample inspection.

## Sample failing idxs (replay via `sed -n "${idx}p" corpus/diverse_250k_2026_05_09.txt`)

### Class 1 — naku classic
- idx=58801 word=`気をつけなく`
- idx=33998 word=`用が足りなく`

### Class 2 — result count (たら)
- idx=17096 word=`なくなったら` — Rust returns 3 compounds, Lisp returns []
- idx=17096 word=`れなくなったら`

### Class 2 — result count (uncategorized)
- idx=22328 word=`じら`
- idx=115809 word=`夫ら`

### Class 3 — てくれ
- idx=19743 word=`来てくれ`
- idx=135858 word=`助けてくれ`

### Class 3 — いられ
- idx=182747 word=`忘れていられ`
- idx=96139 word=`入っていられ`

### Class 3 — られ
- idx=19875 word=`になってられ`
- idx=19875 word=`冷静になってられ`

### Class 3 — らしい
- idx=236311 word=`とれていたらしい`
- idx=87562 word=`にさせようとしたらしい`

### Class 3 — ない
- idx=51751 word=`愛そうにない`
- idx=204205 word=`落ちついていられない`

### Class 3 — くれ
- idx=124361 word=`ないでくれ`
- idx=37896 word=`くつろいでくれ`

## suffix-rashii dedicated audit (this session, after find_word_suffix)

Wired a fresh audit binary (`kaniran-core/audit/dict/suffix_rashii_test.rs`) against the existing dedup parquet `corpus/extracted_chunk_c_suffix_abbr_2026_05_16/dict/suffix_rashii.parquet` (11,669 rows, 127 KB).

Wall: **3 seconds** (vs 28 min for the 178M-row find_word_suffix audit).

| Stage | Failures | Notes |
|---|---|---|
| Initial (Debug fingerprint including id) | 218 | id slot dominated all 218 |
| After stripping `id: NNN, ` | **20** | 99.83% pass — all 20 are seq drift |
| Result-count mismatches | 0 | — |
| Other field drift | 0 | — |

### Why id-strip was needed

suffix-rashii is reached via `find-word-with-conj-type → find-word` during extraction. The segmenter binds `*substring-hash*`, so `find-word` reconstructs root-word DAOs via `(apply 'make-instance init)` (`dict.lisp:493`) where `init` lacks `:id`. Captures see id=null → audit-side struct `id=0`. Rust replay calls the production DB path → real ids. Strict id equality would fail every synthesized row. Same treatment as `find_word_suffix_test`.

The opus completeness reviewer initially missed this (claimed no synthesis path was reached), but the empirical id=0 in 186 of 218 Lisp captures was the disproof.

### Root cause of the remaining 20 (and likely the 116/62/14 wider-audit clusters)

**Non-determinism in row ordering at two stages, accumulated across the call chain:**

1. **SQL queries have no `ORDER BY`.** Example at `kaniran-core/src/dict/find_word_seq.rs:67`:
   ```sql
   SELECT * FROM kana_text WHERE text = $1 AND seq = ANY($2)
   ```
   PostgreSQL row return order without ORDER BY is undefined — depends on physical layout, parallel-worker scheduling, planner stats. When two rows share a surface text (e.g. seq=10463959 and seq=10037996 both `text="やって"`), they come back in whatever order. Same query, same DB, same row set — different visit order between Rust's sqlx and Lisp's postmodern.

2. **`pair-words-by-conj` uses a hash-table.** Lisp's hash-table-values iteration order is undefined; Rust's `HashMap` is randomized by SipHash. The Rust port at `kaniran-core/src/dict/pair_words_by_conj.rs:39` faithfully mirrors with `HashMap`. When multiple words land in the same conjugation-key bucket, `setf (elt arr idx)` overwrites last-write-wins.

The two stacks converge on **different valid candidates** because each picks "first valid match" from a multi-row result, and the "first" disagrees. Neither violates the upstream spec — the spec doesn't define an order. The audit's sort-fingerprints-before-comparing only bridges identical sets in different orders; it can't bridge two singleton results built around different candidates.

This is the same root cause behind the 116 てくれ + 62 いられ + 14 らしい drift clusters in the wider find-word-suffix audit. Fixing it at the SQL/HashMap layer would benefit the whole suffix-fn family.

### Fix options (same as the find_word_suffix wider audit)

1. **Add `ORDER BY id` to underlying SELECTs** (`find_word_seq.rs:67`, `find_word.rs` chains, etc.) — pins Rust's order. **Lisp still has no ORDER BY** so the fix only converges if upstream also gets the same ORDER BY. Two-side patch.
2. **Patch upstream Lisp to add ORDER BY**, then re-extract — symmetric fix. Requires touching upstream (currently read-only) and re-extraction.
3. **Audit comparator accepts a set of valid candidates** — banned by `feedback_no_comparator_workarounds`.
4. **Accept ~0.17% as inherent non-determinism** — the 20 failures aren't port bugs; they're spec-undefined behavior. Document and move on.

Option 1+2 (synchronized ORDER BY both sides) is the only real fix. Option 4 is pragmatic if drift rates stay low across other suffix-fns.

## Recommended next-investigation order

1. **suffix-to / たら cluster (89 failures)** — densest non-naku concentration, result-count shape suggests a missing gate or different filter logic. Pick `なくなったら` (idx 17096) as the first repro.
2. **suffix-teiru / いられ cluster (78 failures)** — same SQL ordering non-determinism root cause as suffix-rashii's 20. Add ORDER BY id to find_word_seq.rs / find_word.rs.
3. **suffix-te+space / てくれ cluster (116 failures)** — same root cause likely.
4. **suffix-rashii (20)** — small, residual seq-drift. Defer until SQL ordering is patched.
5. **naku class 1 (969)** — only fixable via structural identity-aliasing or accepted as artifact. Defer pending other classes.

## Tooling state at end of session

- Audit runner emits all failures inline (`audit/common/mod.rs:434` — `eprintln!("FAIL [row N idxs=[…]] msg")`)
- Captured rows carry `idxs: Vec<i64>` from the new dedup column
- Pool `max_connections=100`, `ASYNC_CONCURRENCY=50` — DB headroom plenty, won't help further; bottleneck is parquet-decode CPU
- Dedup script `dedup_find_word_suffix_idxs.sh` deployed on .103; pattern is reusable for other FQNs
- New audit binary: `kaniran-core/audit/dict/suffix_rashii_test.rs` (registered in `kaniran-core/Cargo.toml`)
- Eager-populate "fix" for naku reverted from `kaniran-core/src/dict/init_suffixes_thread.rs` — moves failures from one class to another, doesn't reduce total
- Memory added this session:
  - `feedback_dump_all_failures_to_file.md`
  - `feedback_dedup_must_preserve_idx.md`
- Branch: `troubleshoot-484` (this branch)
- Remote parquet kept on .103: `extracted_find_word_layer_2026_05_21_dedup/dict/find_word_suffix_idxs.parquet` (4.6 GB, idxs preserved)
