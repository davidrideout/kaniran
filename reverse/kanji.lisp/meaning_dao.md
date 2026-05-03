# meaning (dao-class)

**Package:** `ichiran/kanji`  
**Source:** `kanji.lisp:83`  
**Metaclass:** `dao-class`

**Table:** `MEANING`  
**Primary key:** `(ID)`

## Inheritance

- Direct supers: `standard-object`
- Precedence list: `meaning`, `standard-object`, `slot-object`, `t`

## Columns

| name | column | type | initargs | readers |
|---|---|---|---|---|
| ID | id | `s-sql:serial` | NIL | (ID) |
| KANJI-ID | kanji_id | `integer` | (KANJI-ID) | (KANJI-ID) |
| TEXT | text | `string` | (TEXT) | (TEXT) |


## Source-walked references

- `ichiran/kanji:id`
- `ichiran/kanji:kanji-id`
