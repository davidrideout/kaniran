# kana-text (dao-class)

**Package:** `ichiran/dict`  
**Source:** `dict.lisp:128`  
**Metaclass:** `dao-class`

**Table:** `KANA-TEXT`  
**Primary key:** `(ID)`

## Inheritance

- Direct supers: `simple-text`
- Precedence list: `kana-text`, `simple-text`, `standard-object`, `slot-object`, `t`

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
| BEST-KANJI | best_kanji | `(or s-sql:db-null string)` | (BEST-KANJI) | (BEST-KANJI) |


## Source-walked references

- `ichiran/dict:best-kanji`
- `ichiran/dict:common`
- `ichiran/dict:common-tags`
- `ichiran/dict:conjugate-p`
- `ichiran/dict:id`
- `ichiran/dict:nokanji`
- `ichiran/dict:ord`
- `ichiran/dict:seq`
- `ichiran/dict:simple-text`
