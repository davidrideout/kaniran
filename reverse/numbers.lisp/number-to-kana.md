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
- `ichiran/numbers:across`
- `ichiran/numbers:collect`
- `ichiran/numbers:cur-group`
- `ichiran/numbers:else`
- `ichiran/numbers:finally`
- `ichiran/numbers:for`
- `ichiran/numbers:group`
- `ichiran/numbers:group-to-kana`
- `ichiran/numbers:groups`
- `ichiran/numbers:in`
- `ichiran/numbers:kanji`
- `ichiran/numbers:kanji-method`
- `ichiran/numbers:last-class`
- `ichiran/numbers:last-val`
- `ichiran/numbers:n`
- `ichiran/numbers:number-to-kanji`
- `ichiran/numbers:separator`
- `ichiran/numbers:val`
- `ichiran/numbers:with`
