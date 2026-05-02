# conjugation (dao-class)

**Package:** `ichiran/dict`  
**Source:** `dict.lisp:238`  
**Metaclass:** `dao-class`

**Table:** `CONJUGATION`  
**Primary key:** `(ID)`

## Inheritance

- Direct supers: `standard-object`
- Precedence list: `conjugation`, `standard-object`, `slot-object`, `t`

## Columns

| name | column | type | initargs | readers |
|---|---|---|---|---|
| ID | id | `s-sql:serial` | NIL | (ID) |
| SEQ | seq | `integer` | (SEQ) | (SEQ) |
| FROM | from | `integer` | (FROM) | (SEQ-FROM) |
| VIA | via | `(or integer s-sql:db-null)` | (VIA) | (SEQ-VIA) |

