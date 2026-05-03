# restricted-readings (dao-class)

**Package:** `ichiran/dict`  
**Source:** `dict.lisp:221`  
**Metaclass:** `dao-class`

**Table:** `RESTRICTED-READINGS`  
**Primary key:** `(ID)`

## Inheritance

- Direct supers: `standard-object`
- Precedence list: `restricted-readings`, `standard-object`, `slot-object`, `t`

## Columns

| name | column | type | initargs | readers |
|---|---|---|---|---|
| ID | id | `s-sql:serial` | NIL | (ID) |
| SEQ | seq | `integer` | (SEQ) | (SEQ) |
| READING | reading | `string` | (READING) | (READING) |
| TEXT | text | `string` | (TEXT) | (TEXT) |


## Source-walked references

- `ichiran/dict:id`
- `ichiran/dict:reading`
- `ichiran/dict:seq`
