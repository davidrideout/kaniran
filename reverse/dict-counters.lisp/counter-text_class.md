# counter-text (defclass)

**Package:** `ichiran/dict`  
**Source:** `dict-counters.lisp:9`  
**Metaclass:** `standard-class`

## Inheritance

- Direct supers: `standard-object`
- Precedence list: `counter-text`, `standard-object`, `slot-object`, `t`

## Direct slots

| name | initform | allocation | initargs | readers | writers |
|---|---|---|---|---|---|
| TEXT | `nil` | INSTANCE | (TEXT) | (COUNTER-TEXT) | NIL |
| KANA | `nil` | INSTANCE | (KANA) | (COUNTER-KANA) | NIL |
| NUMBER-TEXT | `nil` | INSTANCE | (NUMBER-TEXT) | (NUMBER-TEXT) | NIL |
| NUMBER | `nil` | INSTANCE | NIL | (NUMBER-VALUE) | NIL |
| SOURCE | `nil` | INSTANCE | (SOURCE) | (SOURCE) | NIL |
| ORDINALP | `nil` | INSTANCE | (ORDINALP) | (ORDINALP) | NIL |
| SUFFIX | `nil` | INSTANCE | (SUFFIX) | (COUNTER-SUFFIX) | NIL |
| ACCEPTS-SUFFIXES | `nil` | INSTANCE | (ACCEPTS) | (COUNTER-SUFFIX-ACCEPTS) | NIL |
| SUFFIX-DESCRIPTIONS | `nil` | INSTANCE | (SUFFIX-DESCRIPTIONS) | (COUNTER-SUFFIX-DESCRIPTIONS) | NIL |
| DIGIT-OPTS | `nil` | INSTANCE | (DIGIT-OPTS) | (DIGIT-OPTS) | NIL |
| COMMON | `nil` | INSTANCE | (COMMON) | (COUNTER-COMMON) | NIL |
| ALLOWED | `nil` | INSTANCE | (ALLOWED) | (COUNTER-ALLOWED) | NIL |
| FOREIGN | `nil` | INSTANCE | (FOREIGN) | (COUNTER-FOREIGN) | NIL |


## Source-walked references

- `ichiran/dict:common`
- `ichiran/dict:counter-allowed`
- `ichiran/dict:counter-common`
- `ichiran/dict:counter-foreign`
- `ichiran/dict:counter-kana`
- `ichiran/dict:counter-suffix`
- `ichiran/dict:counter-suffix-accepts`
- `ichiran/dict:counter-suffix-descriptions`
- `ichiran/dict:digit-opts`
- `ichiran/dict:number-text`
- `ichiran/dict:number-value`
- `ichiran/dict:ordinalp`
- `ichiran/dict:source`
