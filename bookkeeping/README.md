# bookkeeping/

Internal development and auditing tooling. **None of this is part of the kaniran release** — you don't need anything here to build, install, or use kaniran. See the top-level [README](../README.md) for that.

This directory holds the machinery used to build the Rust port of ichiran and to verify it against the original:

- **`reverse/`** — the dependency graph and porting plan. Introspected metadata for every ichiran symbol (the generated `*.lisp/` md trees), plus the Python scripts (`build_graph.py`, `query.py`) that turn it into a topological port order (`reverse/scripts/PORT_PLAN.md`) and audit ported signatures.
- **`ichiran-extractor/`** — the bulk fixture-capture pipeline (FastAPI + pooled SBCL workers) that records real `(input, output)` pairs from the running ichiran. Those fixtures are what the `kaniran-audit` crate replays for corpus-parity checks. Includes the `:ichi-trace` tracer (`trace_capture.lisp`).
- **`CONVENTIONS.md`** — coding/naming conventions for the ported code.
- **`ichiran-repl.sh`** — helper to open a REPL on the remote ichiran host.
