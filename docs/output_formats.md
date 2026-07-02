# Output formats

kaniran runs one segmentation/romanization pipeline over the input and then
renders the result in one of **four formats**. The pipeline runs exactly once;
the format only chooses how its result is turned into text. The same four
formats are exposed by the `kaniran-cli` binary today and by the HTTP server
(`kaniran-server`) as it comes online — both call the same `render()` entry
point, so a given `(format, input, limit)` produces byte-identical output in
either.

| Format | Output | One-line description |
|---|---|---|
| `romanize` | plain text | Just the romaji string. The default. |
| `romanize-info` | plain text | Romaji, then a dictionary-info trailer per word. |
| `v1` | JSON | ichiran-compatible nested positional arrays. |
| `v2` | JSON | Flat tokens + a dictionary-entries table, joined by JMdict sequence number. |

## Selecting a format

### CLI (`kaniran-cli`)

```
kaniran-cli [--format <FORMAT>] [-f|--full] [-l|--limit N] <text>...
```

| Flag | Effect |
|---|---|
| *(no flag)* | `romanize` (the default) |
| `-f`, `--full` | `v1` (alias for `--format v1`) |
| `--format <FORMAT>` | Explicit format; one of `romanize`, `romanize-info`, `v1`, `v2`. Overrides `-f`. |
| `-l N`, `--limit N` | Beam width — keep the top `N` segmentations. Affects the JSON formats only. Default `1`. |

Examples:

```sh
kaniran-cli "一覧は最高だぞ"                       # romanize
kaniran-cli --format romanize-info "食べたい"       # romaji + info trailer
kaniran-cli -f "食べたい"                           # v1 JSON
kaniran-cli --format v2 "食べたい"                  # v2 JSON
kaniran-cli --format v2 -l 5 "食べたくなかった"
```

### HTTP API (`kaniran-server`)

The format is a request parameter on `/segment` (query string for `GET`, JSON
body for `POST`):

```sh
curl "http://127.0.0.1:3000/segment?text=食べたい&format=v2&limit=1"
curl -H 'content-type: application/json' \
     -d '{"text":"食べたい","format":"v2","limit":1}' \
     http://127.0.0.1:3000/segment
```

| Parameter | Default | Notes |
|---|---|---|
| `text` | *(required)* | Empty/whitespace text is a `400`. |
| `format` | `v2` | Same four names; also accepts the aliases `romaji`→`romanize`, `info`→`romanize-info`, `full`→`v1`. Unknown format is a `400`. |
| `limit` | server default (`5`) | Beam width. |
| `include_paths` | `false` | Include the `paths` array (all kept readings) in `v2`. Renders only when `limit > 1` as well — see [Beam width and `paths`](#beam-width-limit-and-paths). |
| `include_entries` | `true` | Include the `entries` table in `v2`. `false` keeps the tokens at full detail (furigana included) but drops the dictionary table. |
| `include_furigana` | `true` | Include `furigana` ruby segments on `v2` tokens and entry kanji forms. |

The response `Content-Type` follows the format: `application/json` for `v1` /
`v2`, `text/plain; charset=utf-8` for `romanize` /
`romanize-info`.

> The CLI defaults `limit` to `1`, the server to `5`. And the `paths` array is
> opt-in on the server (`include_paths`, default `false`) but always on in the
> CLI. See [Beam width and `paths`](#beam-width-limit-and-paths) below.

---

## `romanize` — plain romaji

The thinnest format: the romanization of the best segmentation, nothing else.

Input `食べたい`:

```
tabetai
```

Input `一覧は最高だぞ`:

```
ichiran wa saikō da zo
```

Romanization uses traditional Hepburn (long vowels as macrons: `saikō`).

## `romanize-info` — romaji plus a dictionary trailer

The romaji line, a blank line, then one `* <word>  <info>` block per top-level
word. Each block is the human-readable dictionary summary — compound
breakdown, conjugation chain, glosses, suffix notes.

Input `食べたい`:

```
tabetai

* tabetai  食べたい 【たべたい】 Compound word: 食べ + たい
 * 食べ 【たべ】
[ Conjugation: [v1] Continuative (~i)
  食べる 【たべる】 : to eat ]
 * たい  [suffix]: want to... / would like to... 
```

This is a text report meant for reading, not parsing. For structured word/gloss
data use `v2`.

---

## `v1` — ichiran-compatible nested JSON

`v1` mirrors upstream ichiran's `jsown` output exactly: **positional arrays**,
no named keys at the top levels, and conjugated forms represented as generated
dictionary entries with **synthetic sequence numbers** (`seq ≥ 10000000`). It
exists for drop-in compatibility with tools written against ichiran.

The shape, from outside in:

- The whole result is an array of **segments**.
- A gap (punctuation, latin, unrecognized text) is a bare **string**.
- A word segment is an array of `[word_list, score]` **alternatives**.
- Each entry in a `word_list` is the triple `[romaji, word_object, prop]`,
  where `prop` is always `[]`.

Input `世界` (`--limit 1`):

```json
[
  [
    [
      [
        [
          "sekai",
          {
            "reading": "世界 【せかい】",
            "text": "世界",
            "kana": "せかい",
            "score": 325,
            "seq": 1373860,
            "gloss": [
              { "pos": "[n]", "gloss": "the world; society; the universe" },
              { "pos": "[n]", "gloss": "sphere; circle; world" },
              { "pos": "[adj-no]", "gloss": "world-renowned; world-famous" },
              { "pos": "[n]", "gloss": "realm governed by one Buddha; space", "field": "{Buddh}", "info": "original meaning" }
            ],
            "conj": []
          },
          []
        ]
      ],
      325
    ]
  ]
]
```

The word object carries these keys (present as applicable):

| Key | Description |
|---|---|
| `reading` | Composite `"漢字 【かな】"` (or bare kana). |
| `text` | Surface text. |
| `kana` | Kana reading — a string, or an array when the entry has several. |
| `score` | Scoring value for this word. |
| `seq` | Sequence number. **Real JMdict `seq` for dictionary entries; synthetic (`≥ 10000000`) for conjugated forms.** |
| `gloss` | Array of `{pos, gloss, [info], [field]}` senses, with bracket decoration (`[n]`, `{Buddh}`). |
| `conj` | Conjugation analyses; recursively nested via `via` chains. Empty for dictionary forms. |
| `compound` / `components` | For compounds: the surface pieces and their full word objects. |
| `suffix` | Suffix-grammar annotation. |
| `counter` | `{value: "Value: N", ordinal: […]}` for number+counter words. |

For a conjugated compound the nesting and synthetic ids are visible. `食べたい`
under `v1` becomes one compound word (`食べ` + `たい`) whose member `食べ`
carries the synthetic `seq` `10093091` and a nested `conj` describing the
`Continuative (~i)` step off `食べる`. `v2` resolves that back to the real root
`seq` `1358280` — see below.

---

## `v2` — tokens + entries JSON

`v2` is kaniran's own format (not from ichiran). The response is two flat
collections joined by JMdict sequence number:

- **`tokens`** — the segmentation, in input order. `text` is the verbatim
  input slice, so concatenating `tokens[].text` reproduces the input exactly
  (there are no offsets; position follows from order). A token references its
  dictionary entry by id; conjugated tokens carry flat root-to-surface `steps`
  analyses with the dictionary form; compound members share a `compound`
  group id; suffix members carry a `{class, description}` grammar
  annotation; kanji tokens carry `furigana` ruby segments.
- **`entries`** — every entry referenced by any token, keyed by sequence
  number: kanji/kana forms with JMdict commonness data (`common`, `tags`) and
  headword furigana, plus structured senses — `pos`/`gloss`/`misc`/`field`/
  `dial` arrays, usage `info`, and `restrict_kanji`/`restrict_kana` naming the
  forms a sense is limited to.

Absent means empty: null/empty/false fields are omitted entirely. No
positional arrays, no recursion, and no synthetic ids — every id is a real
JMdict entry, with conjugated forms pointing at their dictionary root.

### Top-level object

| Field | Type | Description |
|---|---|---|
| `text` | string | The original input. |
| `romanization` | string | Romaji of the best path. |
| `score` | integer | Total score of the best path. |
| `tokens` | array | Flat, ordered tokens covering the whole input. |
| `entries` | object | Dictionary entries referenced by the tokens, keyed by sequence number. |
| `paths` | array | *(optional)* Every kept reading, when opted in via `include_paths` and more than one survived; see [Beam width and `paths`](#beam-width-limit-and-paths). Omitted otherwise. |

### Example

Input `食べたい` (`--limit 1`) — a two-member suffix compound, the first
member conjugated:

```json
{
  "text": "食べたい",
  "romanization": "tabetai",
  "score": 378,
  "tokens": [
    {
      "text": "食べ",
      "reading": "たべ",
      "romanization": "tabe",
      "furigana": [
        {
          "text": "食",
          "reading": "た"
        },
        {
          "text": "べ"
        }
      ],
      "entry": 1358280,
      "score": 378,
      "conjugation": [
        {
          "entry": 1358280,
          "steps": [
            {
              "form": "Continuative (~i)",
              "pos": "v1"
            }
          ],
          "base_form": "食べる",
          "base_reading": "たべる",
          "description": "Continuative (~i)"
        }
      ],
      "compound": 1
    },
    {
      "text": "たい",
      "reading": "たい",
      "romanization": "tai",
      "entry": 2017560,
      "score": 378,
      "compound": 1,
      "suffix": {
        "class": "tai",
        "description": "want to... / would like to..."
      }
    }
  ],
  "entries": {
    "1358280": {
      "kanji": [
        {
          "text": "食べる",
          "common": 25,
          "tags": [
            "ichi1",
            "news2",
            "nf25"
          ],
          "furigana": [
            {
              "text": "食",
              "reading": "た"
            },
            {
              "text": "べる"
            }
          ]
        },
        {
          "text": "喰べる",
          "furigana": [
            {
              "text": "喰",
              "reading": "た"
            },
            {
              "text": "べる"
            }
          ]
        }
      ],
      "kana": [
        {
          "text": "たべる",
          "common": 25,
          "tags": [
            "ichi1",
            "news2",
            "nf25"
          ]
        }
      ],
      "senses": [
        {
          "pos": [
            "v1",
            "vt"
          ],
          "gloss": [
            "to eat"
          ]
        },
        {
          "pos": [
            "v1",
            "vt"
          ],
          "gloss": [
            "to live on (e.g. a salary)",
            "to live off",
            "to subsist on"
          ]
        }
      ]
    },
    "2017560": {
      "kana": [
        {
          "text": "たい",
          "common": 0,
          "tags": [
            "spec1"
          ]
        },
        {
          "text": "ったい"
        }
      ],
      "senses": [
        {
          "pos": [
            "aux-adj",
            "adj-i"
          ],
          "gloss": [
            "want to do ...",
            "would like to do ..."
          ],
          "info": "after the -masu stem of a verb"
        },
        {
          "pos": [
            "suf",
            "adj-i"
          ],
          "gloss": [
            "very ..."
          ],
          "info": "after a noun or the -masu stem of a verb; also ったい"
        }
      ]
    }
  }
}
```

### Gaps

Text with no dictionary match (punctuation, latin, unknown spans) becomes a
gap token — `{text, romanization, gap: true}` and nothing else — with `text`
verbatim. Input `Hello 世界！` yields `"Hello "` (gap), `"世界"`, and `"！"`
(gap): the full-width `！` stays in `text` while its `romanization` is `"! "`,
and the same holds for `、` / `。` (`", "` / `". "`). Concatenating token
texts always reproduces the input.

---

## Beam width (`limit`) and `paths`

`limit` is the search beam width: how many candidate segmentations to keep. It
only affects the JSON formats (`romanize`/`romanize-info` always render the
single best path). See [`beam_with_limit.md`](./beam_with_limit.md) for how the
beam interacts with scoring.

`paths` is opt-in. It appears only when **all** of these hold: the caller
enabled it (`include_paths=true` on the API; the CLI always enables it),
`limit > 1`, and the input is genuinely ambiguous so more than one reading
survives. Then the JSON grows a `paths` array — one entry per reading, each a
full path object `{score, romanization, tokens}` with the same token shape as
the top level. `paths[0]` is the same reading as the top-level
`tokens`/`score`/`romanization`; later entries are the alternatives, ordered by
score. The single `entries` table covers every path. Otherwise `paths` is
omitted entirely (not `null`).

Input `橋` (`--limit 2`, `include_paths=true`) — the best reading plus a
raw-character fallback (which, having no dictionary entry, gets neither an
`entry` nor `furigana`):

```json
{
  "text": "橋",
  "romanization": "hashi",
  "score": 21,
  "tokens": [
    {
      "text": "橋",
      "reading": "はし",
      "romanization": "hashi",
      "furigana": [
        {
          "text": "橋",
          "reading": "はし"
        }
      ],
      "entry": 1237410,
      "score": 21
    }
  ],
  "entries": {
    "1237410": {
      "kanji": [
        {
          "text": "橋",
          "common": 5,
          "tags": [
            "ichi1",
            "news1",
            "nf05"
          ],
          "furigana": [
            {
              "text": "橋",
              "reading": "はし"
            }
          ]
        }
      ],
      "kana": [
        {
          "text": "はし",
          "common": 5,
          "tags": [
            "ichi1",
            "news1",
            "nf05"
          ]
        }
      ],
      "senses": [
        {
          "pos": [
            "n"
          ],
          "gloss": [
            "bridge"
          ]
        }
      ]
    }
  },
  "paths": [
    {
      "score": 21,
      "romanization": "hashi",
      "tokens": [
        {
          "text": "橋",
          "reading": "はし",
          "romanization": "hashi",
          "furigana": [
            {
              "text": "橋",
              "reading": "はし"
            }
          ],
          "entry": 1237410,
          "score": 21
        }
      ]
    },
    {
      "score": -500,
      "romanization": "橋",
      "tokens": [
        {
          "text": "橋",
          "reading": "橋",
          "romanization": "橋",
          "score": 0
        }
      ]
    }
  ]
}
```

So by default neither surface emits `paths`: the CLI defaults `limit` to `1`
(one reading), and the server defaults `include_paths` to `false`. To get
`paths` from the server, pass both `include_paths=true` and a `limit` above `1`;
from the CLI, pass `-l N` (it opts into `paths` automatically).

Two consequences worth spelling out:

- **Raising `limit` alone changes nothing visible.** The beam runs wider, but
  only the best reading is rendered — `limit=5` output is identical to
  `limit=1` unless the wider beam changed which reading *wins*
  ([`beam_with_limit.md`](./beam_with_limit.md)).
- **Same-span dictionary ties are not `paths`.** Several entries tied at one
  span (がくせい = 学生/学制) render as `alternatives` on that token, at any
  `limit`, with no flags. `paths` is for readings that differ in segmentation
  or word choice across the sentence.

---

## How the formats relate

All four are renderings of one pipeline run:

- `romanize` and `romanize-info` are **plain text** for humans.
- `v1` is **ichiran-compatible** nested JSON — positional, synthetic conjugation
  ids, recursive `conj`/`via`. Use it for compatibility with existing ichiran
  tooling.
- `v2` is the **structured JSON for new consumers** — flat tokens referencing
  a dictionary-entries table, verbatim text, real sequence numbers. The
  `include_paths` / `include_entries` / `include_furigana` flags drop its
  optional sections individually (the former `v2-minimal` format is
  `include_entries=false&include_furigana=false`).
