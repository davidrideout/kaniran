# reading (dao-class)

**Package:** `ichiran/kanji`  
**Source:** `kanji.lisp:42`  
**Metaclass:** `dao-class`

**Table:** `READING`  
**Primary key:** `(ID)`

## Inheritance

- Direct supers: `standard-object`
- Precedence list: `reading`, `standard-object`, `slot-object`, `t`

## Columns

| name | column | type | initargs | readers |
|---|---|---|---|---|
| ID | id | `s-sql:serial` | NIL | (ID) |
| KANJI-ID | kanji_id | `integer` | (KANJI-ID) | (KANJI-ID) |
| TYPE | type | `string` | (TYPE) | (READING-TYPE) |
| TEXT | text | `string` | (TEXT) | (TEXT) |
| SUFFIXP | suffixp | `boolean` | (SUFFIXP) | (SUFFIXP) |
| PREFIXP | prefixp | `boolean` | (PREFIXP) | (PREFIXP) |
| STAT-COMMON | stat_common | `integer` | NIL | (STAT-COMMON) |


## Source-walked references

- `ichiran/dict:text`
- `ichiran/kanji:id`
- `ichiran/kanji:kanji-id`
- `ichiran/kanji:prefixp`
- `ichiran/kanji:reading-type`
- `ichiran/kanji:stat-common`
- `ichiran/kanji:suffixp`
