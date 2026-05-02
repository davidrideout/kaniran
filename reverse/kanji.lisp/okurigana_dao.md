# okurigana (dao-class)

**Package:** `ichiran/kanji`  
**Source:** `kanji.lisp:67`  
**Metaclass:** `dao-class`

**Table:** `OKURIGANA`  
**Primary key:** `(ID)`

## Inheritance

- Direct supers: `standard-object`
- Precedence list: `okurigana`, `standard-object`, `slot-object`, `t`

## Columns

| name | column | type | initargs | readers |
|---|---|---|---|---|
| ID | id | `s-sql:serial` | NIL | (ID) |
| READING-ID | reading_id | `integer` | (READING-ID) | (READING-ID) |
| TEXT | text | `string` | (TEXT) | (TEXT) |

