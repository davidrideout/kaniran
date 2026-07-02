# kaniran-server HTTP API

An axum HTTP service over the kaniran segmentation/romanization pipeline.
The dictionary is a memory-mapped rkyv snapshot, loaded once at startup;
requests run on the blocking thread pool. All responses below are real server
output.

## Running

```sh
DATABASE_URL=memory:///abs/path/to/snapshot.rkyv ./kaniran-server
```

| Env var | Default | Meaning |
|---|---|---|
| `DATABASE_URL` | *(required)* | rkyv snapshot URL, `memory://<path>.rkyv` |
| `KANIRAN_HTTP_HOST` | `0.0.0.0` | Bind interface (`127.0.0.1` for loopback-only) |
| `KANIRAN_HTTP_PORT` | `3000` | Bind port |
| `KANIRAN_DEFAULT_LIMIT` | `5` | Beam width when a request omits `limit` |
| `KANIRAN_LOG` | `info` | `tracing` filter when `RUST_LOG` is unset |

## Endpoints

| Method + path | Purpose |
|---|---|
| `GET /health` | Liveness probe |
| `GET /segment` | Segment/romanize; parameters as query string |
| `POST /segment` | Same, parameters as a JSON body |
| `GET /docs` | Swagger UI (interactive, try-it-out) |
| `GET /api-docs/openapi.json` | Raw OpenAPI spec backing `/docs` |

## `GET /health`

```
$ curl http://127.0.0.1:3000/health
ok
```

## `GET /segment` / `POST /segment`

Both routes take the same parameters — query string on `GET`, JSON body on
`POST`:

```sh
curl 'http://127.0.0.1:3000/segment?text=食べたい&format=v2&limit=1'
curl -X POST http://127.0.0.1:3000/segment \
     -H 'Content-Type: application/json' \
     -d '{"text": "食べたい", "format": "v2", "limit": 1}'
```

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `text` | string | *(required)* | Japanese text to process. Empty/whitespace → `400` |
| `format` | string | `v2` | One of `romanize`, `romanize-info`, `v1`, `v2`, `v2-minimal`; aliases `romaji`, `info`, `full`, `minimal`. Unknown → `400` |
| `limit` | integer | server default (`5`) | Segmentation beam width; affects the JSON formats only |
| `include_paths` | boolean | `false` | Add the `paths` array (every kept reading) to `v2` / `v2-minimal` when the input is ambiguous and `limit > 1` |
| `include_entries` | boolean | `true` | Include the `entries` table in `v2`; `false` keeps tokens at full detail but drops the dictionary table |

`Content-Type` of the response follows the format: `application/json` for
`v1` / `v2` / `v2-minimal`, `text/plain; charset=utf-8` for `romanize` /
`romanize-info`.

### `format=romanize` — plain romaji

```
$ curl '…/segment?text=一覧は最高だぞ&format=romanize'
ichiran wa saikō da zo
```

### `format=romanize-info` — romaji plus a dictionary trailer

Human-readable text, not meant for parsing:

```
$ curl '…/segment?text=食べたい&format=romanize-info'
tabetai

* tabetai  食べたい 【たべたい】 Compound word: 食べ + たい
 * 食べ 【たべ】
[ Conjugation: [v1] Continuative (~i)
  食べる 【たべる】 : to eat ]
 * たい  [suffix]: want to... / would like to...
```

### `format=v1` — ichiran-compatible nested JSON

Positional arrays, synthetic conjugation ids, recursive `conj`/`via` — for
compatibility with existing ichiran tooling only. See
[`output_formats.md`](./output_formats.md) for the shape.

```
$ curl '…/segment?text=橋&format=v1&limit=1'
[[[[["hashi",{"reading":"橋 【はし】","text":"橋","kana":"はし","score":21,"seq":1237410,"gloss":[{"pos":"[n]","gloss":"bridge"}],"conj":[]},[]]],21]]]
```

### `format=v2` — tokens + entries JSON

The structured format for new consumers: flat `tokens` (verbatim input
slices, in order) referencing a dictionary `entries` table by JMdict sequence
number. Specified field by field in
[`v2-api-spec-pt2.md`](./v2-api-spec-pt2.md). Absent fields mean
null/empty/false.

`GET /segment?text=食べたい&format=v2&limit=1` (pretty-printed):

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

### `format=v2-minimal` — tokens only

`v2` without the `entries` table and without `furigana`; token-local data
(conjugation analyses, `suffix`, `counter`, `compound`) stays.

`GET /segment?text=食べたい&format=v2-minimal&limit=1` (pretty-printed):

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
  ]
}
```

## Errors

JSON body `{"error": "<message>"}` with the status code:

| Status | When |
|---|---|
| `400` | Empty `text`, or unknown `format` |
| `422` | Malformed JSON body on `POST` |
| `500` | Internal pipeline error |

```
$ curl '…/segment?text='
{"error":"`text` must not be empty"}                                        # 400
$ curl '…/segment?text=あ&format=v9'
{"error":"unknown format `v9` (expected: romanize, romanize-info, v1, v2, v2-minimal)"}  # 400
```

## `/docs` — interactive documentation

Swagger UI for the whole API is served at **`/docs`**, generated from the
handler annotations at build time and backed by the raw OpenAPI spec at
`/api-docs/openapi.json`. Use it to try requests from the browser.
