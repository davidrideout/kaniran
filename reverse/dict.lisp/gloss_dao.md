# gloss (dao-class)

**Package:** `ichiran/dict`  
**Source:** `dict.lisp:178`  
**Metaclass:** `dao-class`

**Table:** `GLOSS`  
**Primary key:** `(ID)`

## Inheritance

- Direct supers: `standard-object`
- Precedence list: `gloss`, `standard-object`, `slot-object`, `t`

## Columns

| name | column | type | initargs | readers |
|---|---|---|---|---|
| ID | id | `s-sql:serial` | NIL | (ID) |
| SENSE-ID | sense_id | `integer` | (SENSE-ID) | (SENSE-ID) |
| TEXT | text | `string` | (TEXT) | (TEXT) |
| ORD | ord | `integer` | (ORD) | (ORD) |

