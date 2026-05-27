# number-to-kana

**Package:** `ichiran/numbers`  
**Source:** `numbers.lisp:125`  
**Definition form:** `defun`

## Inputs

`(ichiran/numbers::n &key (ichiran/numbers::separator #\ )
  (ichiran/numbers::kanji-method 'ichiran/numbers:number-to-kanji))`

## Outputs

Declared ftype: `(function (t &key (:separator t) (:kanji-method t)) *)`

## Dependencies (ichiran symbols)

- `ichiran/characters:join`
- `ichiran/numbers:group-to-kana`

## Source-walked references

- `ichiran/characters:join`
- `ichiran/numbers:*char-number-class-hash*`
- `ichiran/numbers:group-to-kana`
- `ichiran/numbers:number-to-kanji`
