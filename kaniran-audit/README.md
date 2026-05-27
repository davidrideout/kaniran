# kaniran-audit

Internal tooling. **If you just want to use kaniran, you can ignore this crate** — see the top-level [README](../README.md).

This crate exists for bulk correctness auditing of `kaniran-core`. It holds one small binary per ported function that replays a large corpus of recorded `(input, output)` fixtures from the original ichiran and checks that kaniran produces byte-identical results.

It is not a library, not part of the public API, and not needed to build or run kaniran. The fixtures it replays are generated separately and kept out of the repo.

Each binary lives at `audit/<module>/<function>_test.rs` and is run by hand against a fixture file, e.g.:

```sh
cargo run -p kaniran-audit --bin <function>_test -- --path <fixtures>.parquet
```
