# kanji (dao-class)

**Package:** `ichiran/kanji`  
**Source:** `kanji.lisp:10`  
**Metaclass:** `dao-class`

**Table:** `KANJI`  
**Primary key:** `(ID)`

## Inheritance

- Direct supers: `standard-object`
- Precedence list: `kanji`, `standard-object`, `slot-object`, `t`

## Columns

| name | column | type | initargs | readers |
|---|---|---|---|---|
| ID | id | `s-sql:serial` | NIL | (ID) |
| TEXT | text | `string` | (TEXT) | (TEXT) |
| RADICAL-C | radical_c | `integer` | (RADICAL-C) | (RADICAL-C) |
| RADICAL-N | radical_n | `integer` | (RADICAL-N) | (RADICAL-N) |
| GRADE | grade | `(or s-sql:db-null integer)` | (GRADE) | (GRADE) |
| STROKES | strokes | `integer` | (STROKES) | (STROKES) |
| FREQ | freq | `(or s-sql:db-null integer)` | (FREQ) | (FREQ) |
| STAT-COMMON | stat_common | `integer` | NIL | (STAT-COMMON) |
| STAT-IRREGULAR | stat_irregular | `integer` | NIL | (STAT-IRREGULAR) |


## Source-walked references

- `ichiran/kanji:freq`
- `ichiran/kanji:grade`
- `ichiran/kanji:id`
- `ichiran/kanji:radical-c`
- `ichiran/kanji:radical-n`
- `ichiran/kanji:stat-common`
- `ichiran/kanji:stat-irregular`
- `ichiran/kanji:strokes`
