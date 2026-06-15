# How to load JMdict + kanjidic2 into a kaniran database

This builds a kaniran Postgres database from source dictionary data:
JMdict (Japanese-English dictionary) and kanjidic2 (kanji database), both
published by the [Electronic Dictionary Research and Development Group
(EDRDG)](http://www.edrdg.org/). The kaniran loader parses the raw XML and
populates every table the segmenter/romanizer reads at runtime.

If you only want a working database and don't care about rebuilding from
source, restore ichiran's prebuilt `.pgdump` instead (see the top-level
`README.md`). The steps below are for loading fresh dictionary data yourself.

## Prerequisites

- PostgreSQL (client tools `createdb` / `dropdb` / `psql` on your `PATH`,
  and a running server you can connect to).
- A Rust toolchain (`cargo`), to build the loader.
- This repository checked out. The custom-data files
  (`kaniran-core/data/sources/{extra.xml,gyoseiku.csv,jichitai.csv}`) ship
  with the repo and are read from a path baked in at compile time — you do
  not download these.

## 1. Get the dictionary data

JMdict and kanjidic2 are updated roughly daily. The quickest path is the
helper script, which downloads the current releases from EDRDG and
decompresses them into `data/` at the repository root:

```sh
init/fetch_data.sh          # -> data/JMdict_e.xml, data/kanjidic2.xml
```

Pass `--force` to refresh existing files; set `JMDICT_URL` / `KANJIDIC_URL`
to pin a specific snapshot. If you ran the script, the data is ready — skip
the rest of this section and go to step 2. To fetch the files by hand
instead:

**JMdict (English edition, `JMdict_e`):**

```sh
# curl
curl -O http://ftp.edrdg.org/pub/Nihongo/JMdict_e.gz
# or wget
wget http://ftp.edrdg.org/pub/Nihongo/JMdict_e.gz

gunzip JMdict_e.gz          # -> JMdict_e   (~62 MB XML)
mv JMdict_e JMdict_e.xml
```

**kanjidic2:**

```sh
# curl
curl -O http://www.edrdg.org/kanjidic/kanjidic2.xml.gz
# or wget
wget http://www.edrdg.org/kanjidic/kanjidic2.xml.gz

gunzip kanjidic2.xml.gz     # -> kanjidic2.xml   (~15 MB XML)
```

Notes:
- Use the **`JMdict_e`** edition (English only). The loader expects the
  EDRDG `JMdict_e` schema; the full multilingual `JMdict.gz` and the
  example-sentence `JMdict_e_examples.gz` are different files.
- To pin a snapshot for reproducible loads, save the files with the
  publish date, e.g. `JMdict_e_2026-06-05.xml`. The `Last-Modified`
  header tells you the release date:
  `curl -sI http://ftp.edrdg.org/pub/Nihongo/JMdict_e.gz | grep -i last-modified`.

### License / attribution

JMdict and kanjidic2 are the property of EDRDG and are used in conformance
with its [licence](http://www.edrdg.org/edrdg/licence.html) (Creative
Commons Attribution-ShareAlike 4.0). Any product built from this data must
acknowledge EDRDG.

## 2. Create the database and apply the schema

The loader empties and repopulates the tables, but it does **not** create
them — apply `init/schema.sql` to a fresh database first.

```sh
# from the repository root
DB=kaniran

dropdb --if-exists "$DB"
createdb -E UTF8 -T template0 "$DB"
psql -q -d "$DB" -v ON_ERROR_STOP=1 -f init/schema.sql
```

## 3. Run the loader

One command runs the whole chain — JMdict entries, conjugations, errata +
custom data, best kanji/kana links, then kanjidic2 and the kanji stats:

```sh
cargo run --release -p kaniran-loader --bin full_e2e_load -- \
    --db-url postgres:///kaniran \
    --jmdict-path   data/JMdict_e.xml \
    --kanjidic-path data/kanjidic2.xml
```

`--db-url` is any libpq URL (`postgres://user:pass@host/db`); the
`postgres:///kaniran` short form uses the local socket and your current
role. The build is slow the first time and the load itself takes a while —
the finished database is several GB and holds millions of conjugation rows.

What the loader does, in order:
1. `load_jmdict` — `entry`, `kanji_text`, `kana_text`, `sense`, `gloss`,
   `sense_prop`, `restricted_readings`, then the conjugation pass
   (`conjugation`, `conj_prop`, `conj_source_reading`) and `load_extras`
   (errata + the bundled custom data).
2. `load_best_readings` — fills `kanji_text.best_kana` / `kana_text.best_kanji`.
3. `load_kanjidic` — `kanji`, `reading`, `okurigana`, `meaning`.
4. `load_kanji_stats` — `kanji.stat_common` / `stat_irregular` /
   `reading.stat_common` (reads the JMdict tables, so it must run last).

Useful flags: `--skip-kanji` (JMdict only), `--skip-extras` (no errata /
secondary conjugations). Recovery flags for resuming a crashed load are
documented in `kaniran-loader/src/bin/full_e2e_load.rs`.

## 4. Dump the database to an rkyv archive

Once the database is loaded, dump the runtime dictionary tables into a
single memory-mapped rkyv archive. This is the read-only backend the
segmenter/romanizer uses at runtime in place of Postgres.

```sh
cargo run --release -p kaniran-loader --bin kaniran-rkyv-dumper -- \
    --db-url postgres:///kaniran \
    --out path/to/dump.rkyv
```

`--db-url` is the database you just loaded (falls back to `DATABASE_URL`
if omitted); `--out` is where the archive is written. The dump streams
every table in physical row order, so the archive reproduces Postgres's
query orderings exactly. Expect peak memory roughly twice the archive
size; the on-disk table sizes printed at the start show the magnitude.
