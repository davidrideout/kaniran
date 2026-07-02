# Segmentation beam width (the `--limit` knob)

## What it is

When kaniran segments a sentence it doesn't commit to one split greedily — it
runs a search over candidate segmentations and keeps the best-scoring ones. The
`-l` / `--limit` flag on `kaniran-cli` (and the `limit` argument threaded through
`romanize*` / `dict-segment` / `find-best-path`) sets **how many candidate paths
that search keeps**.

It is easy to read `--limit` as "how many alternative readings to print," and it
is that — but it is also the **beam width of the search itself**. Inside
`find-best-path`, every intermediate position in the sentence keeps only its top
`limit` partial paths; anything ranked below that is discarded before the search
continues. Because a bonus between non-adjacent words (a "synergy") is only added
when two pieces are joined, a partial path that ranks just below the cutoff at
one position can be the one that — after a later bonus — would have produced the
best *full* sentence reading. At `limit = 1` that path is pruned before the bonus
is ever applied.

The consequence: **a wider beam can change the single best reading, not just add
fallback alternatives.** The best-path score is monotonically non-decreasing as
the beam widens, and it stops changing once the beam is wide enough to never
prune the eventual winner. The width at which that happens, for a given sentence,
is its *convergence width*.

## `limit` vs. `include_paths` — what you actually see

`limit` alone changes only the *search*; the rendered output stays the single
best reading. The extra readings a wide beam keeps become visible only through
the v2 `paths` array, which requires all three of:

- `include_paths=true` (HTTP default `false`; the CLI always opts in),
- `limit > 1` (a width-1 beam keeps nothing else), and
- a genuinely ambiguous input (otherwise one reading survives regardless).

So `limit=5` without `include_paths` usually looks identical to `limit=1` — the
beam ran wider, but only the winner was rendered (they differ only when the
wider beam changed which reading *wins*, per the section above). Same-span
dictionary ties (`alternatives` on a token) are unrelated to either knob and
render at any width. See [`output_formats.md`](./output_formats.md).

## The default: 5 (a deliberate divergence from ichiran's CLI)

ichiran's CLI ships `--limit 1`. kaniran defaults to **5**, matching the
faithfully-ported internal `find-best-path` default (`DEFAULT_LIMIT` in
`kaniran-core/src/dict/path.rs`) and the width the corpus oracle is captured at.
This is an intentional divergence from ichiran's CLI default of 1, not a port
discrepancy.

The sweep below shows width 3 is enough to match width-5 output for practical
purposes (and width 2 already fixes the grammatical errors), so a faster default
of 2 or 3 is defensible. The default stays at 5 for complete parity with the
reference oracle, including the rare cosmetic `では` cases that only resolve at
widths 4–5.

### Configuring it

The default is read at context-build time from the same layered config the
database URL uses — an optional `kaniran.toml` in the working directory, overlaid
by the environment:

```toml
# kaniran.toml
segmentation_limit = 3
```

or `SEGMENTATION_LIMIT=3` in the environment. When neither is set it falls back
to `KANI_DEFAULT_SEGMENTATION_LIMIT` (5) in
`kaniran-core/src/conn/get_segmentation_limit_env.rs`. The value is loaded once
and stored on `KaniranShared.segmentation_limit` (the process-lifetime,
`Arc`-shared half of `KaniranContext`), so reading it costs a field load with no
atomic — nothing new is `Arc`-cloned per call. Precedence: the CLI `-l`/`--limit`
flag overrides; otherwise the configured default is used.

## How the value was chosen

A full sweep over the 1.5M-sentence `cli_full` corpus
(`cli_full_ichiran_latest`) scored the best reading of every sentence at beam
widths 1–5 and at a width-10 reference (treated as "best possible"), and recorded
each sentence's convergence width — the smallest width whose best-reading score
reaches the width-10 ceiling.

### Convergence-width distribution (n = 1,497,263)

| width needed for best reading | sentences | cumulative |
|------------------------------:|----------:|-----------:|
| 1                             | 1,497,040 | 99.9851%   |
| 2                             |       168 | 99.9963%   |
| 3                             |        42 | 99.9991%   |
| 4                             |         6 | 99.9995%   |
| 5                             |         6 | 99.9999%   |
| >5 (width 5 still short)      |         1 | 100.0000%  |

The gains fall off a cliff after width 1, and again after 3:

- **Width 1** already gives the best reading for 99.9851% of sentences. The
  ~0.015% it gets wrong include genuine grammatical errors — most visibly a
  common verb being shattered: `できる` / `出来る` ("to be able to") split into
  pieces (`dekiru` → `de kiru`, or `出来` + a stray `る`).
- **Width 2** fixes the bulk of those (168 sentences).
- **Width 3** mops up the last genuine fixes. Of its 42 sentences, ~34 are the
  cosmetic case below, but ~8 carry real segmentation content — more `できる`
  splits, the conjunction `だから` kept whole (`da kara` → `dakara`), and a
  broken `なら する` (`narasu ru` → `nara suru`). It reaches the optimal score on
  99.9991% of the corpus and reproduces the width-5 reference output
  **byte-for-byte on a 50,000-sentence validation sample**.
- **Widths 4, 5, and beyond** only move 13 sentences in 1.5M, and every one of
  them is the same cosmetic case: the compound particle `では` rendered as
  `dewa` vs. split as `de wa` (a couple are `等` read `tō` vs. `nado`). Same
  meaning, alternate valid reading, score gap of ~13 points — and width 5
  itself can't resolve the last one.

So the search has two regimes: **real segmentation quality is captured by width
3; everything past 3 chases cosmetic particle spacing.** Width 3 is the point
where the output is indistinguishable from ichiran's width-5 default for
practical purposes, at lower cost — a faster default of 2 or 3 is defensible. The
default stays at 5 only for complete parity with the reference oracle, including
those cosmetic `では` cases that resolve at widths 4–5.

### Throughput by width

Single-thread, 20,000 sentences, rkyv backend, steady state:

| width | throughput | vs width 5 |
|------:|-----------:|-----------:|
| 1     | 1460.6 /s  | +90%       |
| 2     | 1209.3 /s  | +57%       |
| 3     |  922.5 /s  | +20%       |
| 5     |  768.7 /s  | —          |

The default (width 3) runs ~20% faster than width 5 while producing the same
output for practical purposes.

(Throughput scales down as the beam widens because the search keeps and extends
more partial paths per position. The gloss/JSON-assembly phase also grows with
width, since every kept alternative is glossed.)

## Reproducing the analysis

Two audit binaries under `kaniran-audit` produce these numbers (rkyv backend;
`DATABASE_URL=memory://corpus/<archive>.rkyv`):

- `cli_full_beam_sweep` — the full convergence-width sweep. Scores each sentence
  at widths 1–5 plus a reference width and prints the histogram; dumps the
  wide-beam tail (width ≥ 4) as examples.

  ```
  cargo run --profile profiling -p kaniran-audit --features rkyv \
    --bin cli_full_beam_sweep -- \
    --path corpus/cli_full_1_5m_ichiran_latest_2026_06_10.parquet --reference 10
  ```

- `cli_full_limit_compare` — renders the best reading at two widths and reports
  how often (and where) they differ. Use it to inspect a specific pair, e.g.
  `--low 1 --high 5` for the user-visible divergences or `--low 3 --high 5` to
  confirm width 3 matches the reference.

  ```
  cargo run --profile profiling -p kaniran-audit --features rkyv \
    --bin cli_full_limit_compare -- \
    --path corpus/cli_full_1_5m_ichiran_latest_2026_06_10.parquet \
    --limit 50000 --low 1 --high 5
  ```

- `cli_full_profile --seg-limit <N>` — single-thread throughput at a chosen
  width (the rows in the throughput table above).
