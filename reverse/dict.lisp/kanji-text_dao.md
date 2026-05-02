# kanji-text (dao-class)

**Package:** `ichiran/dict`  
**Source:** `dict.lisp:86`  
**Metaclass:** `dao-class`

**Table:** `KANJI-TEXT`  
**Primary key:** `(ID)`

## Inheritance

- Direct supers: `simple-text`
- Precedence list: `kanji-text`, `simple-text`, `standard-object`, `slot-object`, `t`

## Columns

| name | column | type | initargs | readers |
|---|---|---|---|---|
| ID | id | `s-sql:serial` | NIL | (ID) |
| SEQ | seq | `integer` | (SEQ) | (SEQ) |
| TEXT | text | `string` | (TEXT) | (TEXT) |
| ORD | ord | `integer` | (ORD) | (ORD) |
| COMMON | common | `(or s-sql:db-null integer)` | (COMMON) | (COMMON) |
| COMMON-TAGS | common_tags | `string` | (COMMON-TAGS) | (COMMON-TAGS) |
| CONJUGATE-P | conjugate_p | `boolean` | (CONJUGATE-P) | (CONJUGATE-P) |
| NOKANJI | nokanji | `boolean` | (NOKANJI) | (NOKANJI) |
| BEST-KANA | best_kana | `(or s-sql:db-null string)` | (BEST-KANA) | (BEST-KANA) |

