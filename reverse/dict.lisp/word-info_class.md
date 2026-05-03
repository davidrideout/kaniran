# word-info (defclass)

**Package:** `ichiran/dict`  
**Source:** `dict.lisp:1245`  
**Metaclass:** `standard-class`

## Inheritance

- Direct supers: `standard-object`
- Precedence list: `word-info`, `standard-object`, `slot-object`, `t`

## Direct slots

| name | initform | allocation | initargs | readers | writers |
|---|---|---|---|---|---|
| TYPE | `nil` | INSTANCE | (TYPE) | (WORD-INFO-TYPE) | ((SETF WORD-INFO-TYPE)) |
| TEXT | `nil` | INSTANCE | (TEXT) | (WORD-INFO-TEXT) | ((SETF WORD-INFO-TEXT)) |
| TRUE-TEXT | `nil` | INSTANCE | (TRUE-TEXT) | (WORD-INFO-TRUE-TEXT) | ((SETF WORD-INFO-TRUE-TEXT)) |
| KANA | `nil` | INSTANCE | (KANA) | (WORD-INFO-KANA) | ((SETF WORD-INFO-KANA)) |
| SEQ | `nil` | INSTANCE | (SEQ) | (WORD-INFO-SEQ) | ((SETF WORD-INFO-SEQ)) |
| CONJUGATIONS | `nil` | INSTANCE | (CONJUGATIONS) | (WORD-INFO-CONJUGATIONS) | ((SETF WORD-INFO-CONJUGATIONS)) |
| SCORE | `0` | INSTANCE | (SCORE) | (WORD-INFO-SCORE) | ((SETF WORD-INFO-SCORE)) |
| COMPONENTS | `nil` | INSTANCE | (COMPONENTS) | (WORD-INFO-COMPONENTS) | ((SETF WORD-INFO-COMPONENTS)) |
| ALTERNATIVE | `nil` | INSTANCE | (ALTERNATIVE) | (WORD-INFO-ALTERNATIVE) | ((SETF WORD-INFO-ALTERNATIVE)) |
| PRIMARY | `t` | INSTANCE | (PRIMARY) | (WORD-INFO-PRIMARY) | ((SETF WORD-INFO-PRIMARY)) |
| START | `nil` | INSTANCE | (START) | (WORD-INFO-START) | ((SETF WORD-INFO-START)) |
| END | `nil` | INSTANCE | (END) | (WORD-INFO-END) | ((SETF WORD-INFO-END)) |
| COUNTER | `nil` | INSTANCE | (COUNTER) | (WORD-INFO-COUNTER) | ((SETF WORD-INFO-COUNTER)) |
| SKIPPED | `0` | INSTANCE | (SKIPPED) | (WORD-INFO-SKIPPED) | ((SETF WORD-INFO-SKIPPED)) |


## Source-walked references

- `ichiran/dict:primary`
- `ichiran/dict:seq`
- `ichiran/dict:true-text`
- `ichiran/dict:word-info-alternative`
- `ichiran/dict:word-info-components`
- `ichiran/dict:word-info-conjugations`
- `ichiran/dict:word-info-counter`
- `ichiran/dict:word-info-end`
- `ichiran/dict:word-info-kana`
- `ichiran/dict:word-info-primary`
- `ichiran/dict:word-info-score`
- `ichiran/dict:word-info-seq`
- `ichiran/dict:word-info-skipped`
- `ichiran/dict:word-info-start`
- `ichiran/dict:word-info-text`
- `ichiran/dict:word-info-true-text`
- `ichiran/dict:word-info-type`
