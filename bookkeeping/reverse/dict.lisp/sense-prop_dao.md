# sense-prop (dao-class)

**Package:** `ichiran/dict`  
**Source:** `dict.lisp:197`  
**Metaclass:** `dao-class`

**Table:** `SENSE-PROP`  
**Primary key:** `(ID)`

## Inheritance

- Direct supers: `standard-object`
- Precedence list: `sense-prop`, `standard-object`, `slot-object`, `t`

## Columns

| name | column | type | initargs | readers |
|---|---|---|---|---|
| ID | id | `s-sql:serial` | NIL | (ID) |
| TAG | tag | `string` | (TAG) | (TAG) |
| SENSE-ID | sense_id | `integer` | (SENSE-ID) | (SENSE-ID) |
| TEXT | text | `string` | (TEXT) | (TEXT) |
| ORD | ord | `integer` | (ORD) | (ORD) |
| SEQ | seq | `integer` | (SEQ) | (SEQ) |


## Source-walked references

- `ichiran/dict:id`
- `ichiran/dict:ord`
- `ichiran/dict:sense-id`
- `ichiran/dict:seq`
- `ichiran/dict:tag`
- `ichiran/dict:text`
