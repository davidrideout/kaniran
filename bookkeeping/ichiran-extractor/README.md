# ichiran-extractor — bulk fixture capture for the Rust port

Pooled SBCL workers + FastAPI + parquet writer for harvesting
`(args, result)` fixtures from live ichiran on a remote host. Drives a
sentence corpus through a hardcoded entry-point sweep and writes one
parquet file per traced FQN — used by the Rust port at `kaniran-core/`
to verify each ported function against the behavior of the original
Lisp.

The default corpus is `../corpus/tatoeba_sentences.txt` (~248K Japanese
sentences from the Tatoeba project). Override with `--input <path>`.

Inspired by — and structurally a variant of — `../ichiran-worker`.
Differences from that project:

- Single `extract` op instead of nuclear/monster traces.
- Per-fn capture via `sb-int:encapsulate` (the `:ichi-trace` package in
  `trace_capture.lisp`) instead of intra-`calc-score` instrumentation.
- Output schema: `(args: utf8, result: utf8)` per FQN, partitioned to
  `<output_dir>/<package>/<symbol>.parquet`. No nested JSON blobs.
- Args/result are stored as readably-printed Lisp source (`prin1` with
  `*print-readably*` bound), parsed on the Rust side via the `lexpr`
  crate.

## Architecture

```
client (fetch_extractor.py)
        │ POST /install {"fqns":[...]}    one-shot, broadcasts to all workers
        │ POST /extract  {"text":"..."}   per sentence, fans across workers
        ▼
┌──────────────────────────────────────────────┐
│ uvicorn → ichiran_main_pooled:app (FastAPI)  │
│   pool.broadcast / pool.execute              │
└──────────────┬───────────────────────────────┘
               │ async acquire from asyncio.Queue
               ▼
┌──────────────────────────────────────────────┐
│ WorkerPool (ichiran_worker_pool.py)          │
│   N × Worker  ←─ asyncio.Queue (available)   │
│        │ JSON-line stdio                     │
│        ▼                                     │
│ sbcl --core ichiran.core                     │
│      --load extractor_worker.lisp            │
│   loop:  read-line → dispatch op             │
│          (install / extract / clear / ...)   │
└──────────────────────────────────────────────┘
```

## Files

| File | Role |
|------|------|
| `trace_capture.lisp` | `:ichi-trace` package. `install` / `install-many` / `uninstall-all` / `clear` / `drain` — `sb-int:encapsulate` wrapper that captures `(args, result)` per call with re-entrance guard and primitive-printability gate. |
| `extractor_worker.lisp` | The persistent SBCL loop. Loads `trace_capture.lisp`, defines the entry-point sweep (`*entry-points*`), dispatches JSON-line ops on stdin: `ping` / `quit` / `installed` / `install` / `clear` / `uninstall-all` / `extract`. |
| `ichiran_main_pooled.py` | FastAPI service. Routes for `/install` (broadcast), `/extract` (per-request), `/clear`, `/installed`. |
| `ichiran_worker_pool.py` | Generic SBCL subprocess pool. `execute(op, ...)` for round-robin requests; `broadcast(op, ...)` for setup ops that mutate every worker's state. |
| `wait_healthy.py` | Polls `/health` with timeout. |
| `deploy_server.sh` | scp + ssh wrapper that ships the lisp/python files to `$REMOTE_HOST`, kills any old uvicorn+SBCL workers, relaunches via `ssh -n -f`, then waits for the pool to go green. |
| `fetch_extractor.py` | The driver. Three subcommands: `install`, `clear`, `fetch <out_dir> [--input <path>]`. The `fetch` flow streams sentences through a bounded async queue, fans to N HTTP workers, partitions captures by FQN, writes per-FQN parquet. Default corpus is `../corpus/tatoeba_sentences.txt`. |

## Boot + run sequence

```sh
# 1. Deploy + start pool (one-time per code change).
REMOTE_HOST=david@192.168.1.103 \
REMOTE_API_DIR=/home/david/pooled-extractor \
REMOTE_STORAGE=/home/david/storage \
REMOTE_HOME=/home/david \
./deploy_server.sh --pool-size 8

# 2. Install the FQNs to capture (one-time per pool start).
#    fqns.txt has one FQN per line, e.g. ICHIRAN/CHARACTERS:NORMALIZE.
python3 fetch_extractor.py --api http://192.168.1.103:9100 \
    install fqns.txt

# 3. Drive the corpus (input defaults to ../corpus/tatoeba_sentences.txt).
python3 fetch_extractor.py --api http://192.168.1.103:9100 \
    fetch ../corpus/extracted \
    --workers 8 --limit 10000

# Override the corpus when needed:
python3 fetch_extractor.py --api http://192.168.1.103:9100 \
    fetch ../corpus/extracted --input /path/to/other.tsv \
    --workers 8
```

After step 3, `../corpus/characters/normalize.parquet` (and one parquet
per other installed FQN) has the dedup'd `(args, result)` pairs ready
to load from the Rust test side.

## Output schema

Each parquet file:

```
args   : utf8     — Lisp source text of the args list, e.g. ("お茶を飲む。" :DEFAULT)
result : utf8     — Lisp source of (multiple-value-list result), e.g. ("お茶を飲む。")
```

Every result is wrapped in `(...)` because the worker captures
`(multiple-value-list (apply original-fn args))`. Single-value returns
appear as a one-element list. Same convention as the existing
`kani::fixture` test infrastructure.

Arrow file-level KV metadata embeds:

| Key | Value |
|-----|-------|
| `ichiran_extractor_fqn` | the FQN this file traces (also derivable from the path) |
| `ichiran_rev`           | from `ICHIRAN_REV` env var at fetch time, else `"unknown"` |
| `captured_at`           | ISO-8601 UTC timestamp at run start |
| `driver`                | from `EXTRACTOR_DRIVER_REV` env, else `"unknown"` |
| `input`                 | input TSV path |

## Dedup invariant

`fetch_extractor.py` deduplicates rows by hash of the `args` string
within each FQN's writer. If the same `args` ever produces two
different `result` values, the writer logs a `WARN nondeterminism`
line on stderr and bumps `n_conflict`. Any non-zero `n_conflict` in
the per-fqn summary at end-of-run is a real bug — either upstream
ichiran has hidden state for this fn (bug or design), or the tracer
re-entrance guard didn't fire when it should have.

## Adapting the install set or entry-points

Two layers:

- **What gets captured**: `POST /install {fqns: [...]}` — runtime list,
  push from `fqns.txt`.
- **What gets called per /extract**: `*entry-points*` in
  `extractor_worker.lisp`. Hardcoded because each entry has a slightly
  different arity (e.g. `normalize` takes `:context :default`,
  `sequential-kanji-positions` takes `text 0`). Edit and redeploy.

Capturing only entry-points is fine; capturing entries + their internal
callees is what gives bulk-fixture coverage of leaf functions like
`get-char-class`.

## Reusing for a different system

`ichiran_worker_pool.py` is generic — it doesn't know about ichiran. To
adapt:

1. Replace `extractor_worker.lisp` with a worker for your system,
   keeping the JSON-line protocol shape.
2. Replace `ichiran_main_pooled.py` routes with whatever ops you expose.
3. Update `deploy_server.sh` file list and the kill patterns.

`trace_capture.lisp` is generic in the SBCL-encapsulate sense — it
records any function symbol resolvable in the running image.
