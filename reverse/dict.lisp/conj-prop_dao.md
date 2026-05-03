# conj-prop (dao-class)

**Package:** `ichiran/dict`  
**Source:** `dict.lisp:262`  
**Metaclass:** `dao-class`

**Table:** `CONJ-PROP`  
**Primary key:** `(ID)`

## Inheritance

- Direct supers: `standard-object`
- Precedence list: `conj-prop`, `standard-object`, `slot-object`, `t`

## Columns

| name | column | type | initargs | readers |
|---|---|---|---|---|
| ID | id | `s-sql:serial` | NIL | (ID) |
| CONJ-ID | conj_id | `integer` | (CONJ-ID) | (CONJ-ID) |
| CONJ-TYPE | conj_type | `integer` | (CONJ-TYPE) | (CONJ-TYPE) |
| POS | pos | `string` | (POS) | (POS) |
| NEG | neg | `(or s-sql:db-null boolean)` | (NEG) | (CONJ-NEG) |
| FML | fml | `(or s-sql:db-null boolean)` | (FML) | (CONJ-FML) |


## Source-walked references

- `ichiran/dict:conj-fml`
- `ichiran/dict:conj-id`
- `ichiran/dict:conj-neg`
- `ichiran/dict:conj-type`
- `ichiran/dict:id`
- `ichiran/dict:pos`
