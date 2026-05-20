# find-best-path session handoff

## Branch

Work lives on **`wip/find-best-path`** (this branch). Branched from `master` (`3ce1658 cull_segments`). Note hyphen — there's also a stale `wip/find_best_path` (underscores) from a prior session that should not be confused with this one.

To resume: `git checkout wip/find-best-path`.

## Status (2026-05-20)

`ichiran/dict:find-best-path` — **ported, unit tests green, audit clean, perf workable** (~64 rows/s ≈ 2.5 hr full-parquet extrapolation). Marked `ported` + `extracted: chunk_b_segmentation_2026_05_14` in `symbols.csv`.

`audit-signatures` clean (no new divergence).

## What changed this session

Three perf-driven structural changes landed in `src/`. Two are pre-approved Arc refactors; one is a new Arc-wrap on `SegmentList.top`.

| file | change | approved? |
|---|---|---|
| `_star_segfilter_list_star_.rs` | `SegFilter` type alias takes `Arc<SegmentList>` | yes (prior session) |
| 16× `segfilter_*.rs` | sig + clone sites switched to `Arc<SegmentList>` | yes (prior session) |
| `apply_segfilters.rs` | rolling splits use `Arc<SegmentList>` internally; `Arc::unwrap_or_clone` at exit boundary | yes (prior session) |
| `make_segment_list_from.rs` | perf fix from prior session — build new SegmentList directly instead of cloning old then overwriting `segments`. Unchanged this session. | yes (prior session) |
| `segment_list_struct.rs` | **NEW**: `pub top: Option<Arc<TopArray>>` (was `Option<TopArray>`) | yes (this session) |
| `find_best_path.rs` | `Arc::new(TopArray::new(...))` at install; `Arc::make_mut(...)` at the two `register_item` mutation sites | yes (this session) |

The Arc-top change collapses the recursive deep-clone of `top` that fired every time `make_segment_list_from` or `SegmentList::clone()` ran. Each clone is now a refcount bump.

## Timing progression

Same hardware, release build, on `/tmp/fbp_3.parquet` (3 dense rows: 16/10/11 SLs):

| state | 3-row | 10-row dense (fbp_10_dense) | 2000-row (fbp_2000) |
|---|--:|--:|--:|
| pre-session (post-Arc-on-SegmentList only) | ~11 s | 6:47 (407 s) | n/a |
| post-Arc-top (this session) | <0.01 s | 2.56 s | 34.6 s (~64 rows/s) |

Full 522,823-row parquet extrapolates to ~2.5 hr at the current 64 rows/s rate.

## What was tried that didn't help

**Option A: `split_at_mut(i+1)` to eliminate the tai-clone inside the `(i, j)` inner loop.** Implemented and tested green; gave essentially no perf change (34.58 s vs 34.6 s baseline). Reverted.

The reason: the original flamegraph showed the tai-clone closure at 90% *inclusive* samples, which is misleading — that 90% includes the entire inner-loop body (get_seg_splits, register_item, etc.) that runs *under* that closure frame. The clone itself is small; removing it only spared the one `slot.clone()` per tai.

Self-time analysis (proper subtraction of child samples) reveals the real bottleneck.

## Real bottleneck (self-time profile)

`/tmp/fbp_2000_flame_v2.svg` — top self-time frames on the 2000-row run:

| % self | function |
|--:|---|
| 78.21 | `_open$NOCANCEL` |
| 6.38 | `alloc::alloc::alloc` |
| 2.23 | `drop_in_place<Segment>` |
| 1.76 | `drop_in_place<Option<KaniSegmentInfo>>` |
| 1.36 | `alloc::alloc::dealloc` |
| 0.93 | `drop_in_place<SegmentList>` |
| 0.91 | `drop_in_place<KanjiText>` |
| 0.75 | `serde_json index_into` |

`_open$NOCANCEL` is the macOS allocator's syscall to get memory pages from the OS. It's not file I/O. The full call stack:

```
_open$NOCANCEL
  alloc::alloc::alloc
    Global::allocate
      RawVecInner::try_allocate_in
        Vec::with_capacity_in   ← every new Vec<PathElement>, Vec<Segment>, etc.
```

Direct callers (samples beneath):

| samples | caller |
|--:|---|
| 5002 | `alloc::alloc::alloc` (allocation side) |
| 4139 | `drop_in_place<SegmentList>` (deallocation side) |
| 136 | drop `Option<KaniSegmentInfo>` |
| 70 | drop `KaniWordDispatchEnum` |
| 58 | `KaniWordDispatchEnum::clone` |

**Diagnosis: malloc/free churn.** Every `path.clone()`, `make_segment_list_from(...)`, and `apply_segfilters` output allocates fresh Vec storage; every loop iteration drops them. The Arc-top fix turned the worst recursive deep-clone into a refcount bump, but everything else is still allocating per-iteration.

## Options remaining

Both keep upstream pointer-share semantics — `top` already proved the pattern works.

**B. Wrap `PathElement::SegmentList(Arc<SegmentList>)`.** Same trick as `.top`, one layer deeper. The Lisp path cons-list holds pointers to segment-list objects shared with every other path that mentions them. The Rust port currently owns inside the enum variant, so cloning a `PathElement::SegmentList(SegmentList)` deep-clones the inner segments Vec. Wrapping in Arc makes that clone a refcount bump.

Apply-segfilters and get-seg-splits already produce `Arc<SegmentList>` internally; this would pass it through at the boundary instead of `Arc::unwrap_or_clone`-ing.

Scope: `PathElement` enum variant, plus every site that constructs / matches on `PathElement::SegmentList`. The audit binary's `compare_path_element` would need to deref through the Arc.

**C. Share the path itself: `payload: Arc<[PathElement]>` (or `Arc<Vec<PathElement>>`) on `TopArrayItem`.** Upstream's `(register-item top-of-seg2 accum path)` then `(register-item top accum+gap-right path)` register the *same* path pointer twice. The Rust port clones because each register takes ownership. Wrapping in Arc makes the second register a refcount bump and removes the `path.clone()` at `find_best_path.rs:194`.

Combines well with B — together the per-split cost becomes one path construction + N refcount bumps + N Vec<Segment> deep clones (the last is the remaining cost, addressable separately).

**Recommended order**: B first, reprofile, then C if `path.clone()` still surfaces.

## Profiling scaffolding (currently in tree)

Added this session and **not yet reverted** — kept in place so the user can resume profiling:

- `Cargo.toml` (workspace root): `[profile.profiling]` inherits release, debug=true.
- `kaniran-core/Cargo.toml`: `pprof = { version = "0.13", features = ["flamegraph"] }` regular dep.
- `audit/dict/find_best_path_test.rs` `main()`: env-gated profiling block.

Usage:
```sh
cargo build --profile profiling --manifest-path kaniran-core/Cargo.toml --bin find_best_path_test

# single-row (row 0) profile:
KANI_PROFILE=/tmp/fbp_flame.svg DATABASE_URL=... \
  target/profiling/find_best_path_test --path /tmp/fbp_1.parquet

# whole-parquet profile (sequential, no async concurrency):
KANI_PROFILE_ALL=/tmp/fbp_2000_flame.svg DATABASE_URL=... \
  target/profiling/find_best_path_test --path /tmp/fbp_2000.parquet
```

Output is a flamegraph SVG. Open in a browser, or parse with the Python self-time analyzer pattern (subtract child samples from parent samples using `fg:x` / `fg:w` attributes — see session transcript for the working snippet).

**Revert before merging to master**: drop the `pprof` dep, the `[profile.profiling]` block, and the `if let Ok(prof_path) = std::env::var(...)` block in the audit binary.

## Sample parquets (local + remote)

Local (`/tmp/`), still present:
- `fbp_1.parquet` — 1 row, 16 SLs (worst single-row dense)
- `fbp_3.parquet` — 3 rows, 16/10/11 SLs
- `fbp_2.parquet`, `fbp_small.parquet`, `fbp_tiny.parquet` — older small samples
- `fbp_10_dense.parquet` — 10 rows, all dense
- `fbp_100.parquet` — 100 rows, 64% have ≥11 SLs
- `fbp_2000.parquet` — first 2000 rows of full parquet (this session's workhorse)
- `fbp_full.parquet` — entire 522,823-row corpus (~656 MB; pulled from .103 this session)
- `find_best_path_sample{1,5,20,200}.parquet` — older samples

Remote (`.103`):
- `/home/david/storage/extracted_chunk_b_segmentation_2026_05_14_dedup/dict/find_best_path.parquet` — 522,823 rows, 656 MB

Flamegraphs saved this session:
- `/tmp/fbp_flame.svg` — 1-row pre-Arc-top profile (stale)
- `/tmp/fbp_2000_flame.svg` — 2000-row post-Arc-top + post-A profile (stale, A reverted)
- `/tmp/fbp_2000_flame_v2.svg` — 2000-row post-Arc-top, post-revert (**current**)

## Self-time profile parser (Python, working)

For future profiling runs. pprof's flamegraph SVG uses `fg:x` and `fg:w` attributes for sample positions:

```python
import re, collections, pathlib
svg = pathlib.Path("/tmp/fbp_2000_flame_v2.svg").read_text()
pat = re.compile(
    r'<title>([^<]+?) \((\d+) samples, [\d.]+%\)</title>'
    r'<rect [^>]*y="(\d+)"[^>]*fg:x="(\d+)"\s+fg:w="(\d+)"',
)
frames = [{"name": n, "samples": int(s), "y": int(y), "x": int(x), "w": int(w), "self": int(s)}
          for n, s, y, x, w in pat.findall(svg)]
by_y = collections.defaultdict(list)
for f in frames:
    by_y[f["y"]].append(f)
y_levels = sorted(by_y.keys())
for f in frames:  # subtract each frame's samples from its parent's self count
    cands = [y for y in y_levels if y > f["y"]]
    if not cands: continue
    for p in by_y[min(cands)]:
        if p["x"] <= f["x"] and (p["x"] + p["w"]) >= (f["x"] + f["w"]):
            p["self"] -= f["samples"]
            break
self_by_name = collections.defaultdict(int)
for f in frames:
    self_by_name[f["name"]] += max(f["self"], 0)
for n, s in sorted(self_by_name.items(), key=lambda kv: -kv[1])[:30]:
    print(f"{s:>7}  {n}")
```

## Recovery / verification commands

```sh
# Build + check
cargo check --manifest-path kaniran-core/Cargo.toml --lib --bins --tests
cargo build --release --manifest-path kaniran-core/Cargo.toml --bin find_best_path_test

# Unit tests (DB-backed)
DATABASE_URL=postgres://david@localhost/ichiran cargo test --manifest-path kaniran-core/Cargo.toml --lib dict::find_best_path -- --test-threads=1

# Audit (release)
DATABASE_URL=postgres://david@localhost/ichiran target/release/find_best_path_test --path /tmp/fbp_2000.parquet
```

## Working tree state at handoff

Modified vs master:
- `Cargo.lock`, `Cargo.toml` (workspace root: added `[profile.profiling]`)
- `ichiran-extractor/extractor_worker.lisp` (other session's work, not this)
- `kaniran-core/Cargo.toml` (added `pprof` dep, find_best_path_test bin entry)
- `kaniran-core/src/dict/_star_segfilter_list_star_.rs`
- `kaniran-core/src/dict/apply_segfilters.rs`
- `kaniran-core/src/dict/find_best_path.rs` (new file vs master)
- `kaniran-core/src/dict/make_segment_list_from.rs`
- `kaniran-core/src/dict/mod.rs`
- `kaniran-core/src/dict/segment_list_struct.rs`
- 16× `kaniran-core/src/dict/segfilter_*.rs`
- `kaniran-core/audit/dict/find_best_path_test.rs` (new vs master; has profiling block)
- `reverse/scripts/symbols.csv` (find-best-path marked ported)

Untracked: `find_best_path.md` (this file).

The cull_segments work from the other session is also in the tree — do not commit alongside.
