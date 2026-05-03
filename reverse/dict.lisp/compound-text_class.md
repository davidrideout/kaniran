# compound-text (defclass)

**Package:** `ichiran/dict`  
**Source:** `dict.lisp:608`  
**Metaclass:** `standard-class`

## Inheritance

- Direct supers: `standard-object`
- Precedence list: `compound-text`, `standard-object`, `slot-object`, `t`

## Direct slots

| name | initform | allocation | initargs | readers | writers |
|---|---|---|---|---|---|
| TEXT | `nil` | INSTANCE | (TEXT) | (TEXT) | NIL |
| KANA | `nil` | INSTANCE | (KANA) | (GET-KANA) | NIL |
| PRIMARY | `nil` | INSTANCE | (PRIMARY) | (PRIMARY) | NIL |
| WORDS | `nil` | INSTANCE | (WORDS) | (WORDS) | NIL |
| SCORE-BASE | `nil` | INSTANCE | (SCORE-BASE) | NIL | NIL |
| SCORE-MOD | `nil` | INSTANCE | (SCORE-MOD) | (SCORE-MOD) | NIL |


## Source-walked references

- `ichiran/dict:get-kana`
- `ichiran/dict:primary`
- `ichiran/dict:score-base`
- `ichiran/dict:score-mod`
- `ichiran/dict:words`
