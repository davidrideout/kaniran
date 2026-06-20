# kaniran 🦀

A Rust port of [ichiran](https://github.com/tshatrov/ichiran) — Japanese text segmentation and romanization, backed by the [JMdictDB](http://edrdg.org/~smg/) dictionary.

kaniran is a Rust port of ichiran (originally Common Lisp + PostgreSQL). At runtime it reads from a memory-mapped **rkyv archive** of ichiran's dictionary — a single self-contained file, no database server. You build that archive once (Postgres is needed only for this build step), then point kaniran at it.

## Requirements

- Rust (stable) and Cargo
- PostgreSQL — only to build the archive in step 1. kaniran does not connect to a database at runtime.

## 1. Build the dictionary archive

kaniran runs against an rkyv archive of ichiran's dictionary. Building it is a two-step, one-time process: bring up the dictionary in Postgres, then dump it to the archive.

1. Download the latest `.pgdump` file from ichiran's releases:
   https://github.com/tshatrov/ichiran/releases

2. Create the database. It must use UTF-8 with the Japanese locale, or collation will be wrong:

   ```sh
   createdb -E UTF8 -l ja_JP.utf8 -T template0 jmdict
   ```

   (`ja_JP.utf8` must be installed on the system. On Debian/Ubuntu: `sudo locale-gen ja_JP.UTF-8`.)

3. Restore the dump into it:

   ```sh
   pg_restore -d jmdict --no-owner --no-privileges ichiran-XXXXXX.pgdump
   ```

   Replace `ichiran-XXXXXX.pgdump` with the file you downloaded. This takes a while and the database ends up around 5 GB. A few warnings during restore are normal.

4. Dump the database into an rkyv archive:

   ```sh
   cargo run --release -p kaniran-loader --bin kaniran-rkyv-dumper -- \
       --db-url postgres:///jmdict \
       --out jmdict.rkyv
   ```

   The archive is self-contained — once it exists you don't need Postgres again.

Alternatively, you can build the database yourself from the source JMdict + kanjidic2 data instead of restoring the prebuilt dump (then dump it to an archive the same way) — see [docs/GETTING_STARTED_ALTERNATE.md](docs/GETTING_STARTED_ALTERNATE.md).

## 2. Configure the connection

kaniran finds the archive via a `database_url` in `kaniran.toml` (in the directory you run from), or the `DATABASE_URL` environment variable. The env var wins if both are set. Use a `memory://` URL pointing at the `.rkyv` file you built.

Copy the example and edit it:

```sh
cp kaniran.toml.example kaniran.toml
```

```toml
# kaniran.toml
database_url = "memory:///path/to/jmdict.rkyv"
```

`kaniran.toml` is gitignored, so your local path stays out of the repo. Equivalent env-var form:

```sh
export DATABASE_URL="memory:///path/to/jmdict.rkyv"
```

## 3. Use the CLI

Build it once:

```sh
cargo build --release -p kaniran-cli
```

The binary lands at `target/release/kaniran-cli`. Run it from a directory containing `kaniran.toml` (or with `DATABASE_URL` set to the archive).

Romanize a sentence (default):

```sh
$ kaniran-cli "カニに感謝"
kani ni kansha
```

Add dictionary info with `-i`:

```sh
$ kaniran-cli -i "カニに感謝"
kani ni kansha

* kani  カニ
1. [n] crab

* ni  に
1. [prt] at (place, time); in; on; during
2. [prt] to (direction, state); toward; into
3. [prt] for (purpose)
...

* kansha  感謝 【かんしゃ】
1. [vi,vt,vs,n] thanks; gratitude; appreciation; thankfulness
```

Get full segmentation as JSON with `-f` (use `-l` to cap the number of segmentations):

```sh
$ kaniran-cli -f -l 1 "カニ"
[[[[["kani",{"reading":"カニ","text":"カニ","kana":"カニ","score":32,"seq":1202410,"gloss":[{"pos":"[n]","gloss":"crab"}],"conj":[]},[]]],32]]]
```

During development you can skip the build step with `cargo run`:

```sh
cargo run --release -p kaniran-cli -- -i "カニに感謝"
```

## 4. Run the tests

The core crate's test suite includes tests that read the dictionary, so it needs the archive set and must run single-threaded:

```sh
DATABASE_URL="memory:///path/to/jmdict.rkyv" \
  cargo test -p kaniran-core -- --test-threads=1
```

## Credit

All segmentation and romanization logic is ported from [ichiran](https://github.com/tshatrov/ichiran) by Timofei Shatrov. See `LICENSE`.
