# insert-conjugation

**Package:** `ichiran/dict`  
**Source:** `dict-load.lisp:375`  
**Definition form:** `defun`

## Inputs

`(ichiran/dict::readings &key ichiran/dict::seq ichiran/dict::from
  ichiran/dict::pos ichiran/dict::conj-type ichiran/dict::neg ichiran/dict::fml
  ichiran/dict::via)`

## Outputs

Declared ftype: `(function
                  (t &key (:seq t) (:from t) (:pos t) (:conj-type t) (:neg t)
                   (:fml t) (:via t))
                  (values boolean &optional))`

## Dependencies (ichiran symbols)

- `ichiran/dict:id`
- `ichiran/dict:lex-compare`

## Source-walked references

- `ichiran/dict:*secondary-conjugation-types-from*`
- `ichiran/dict:conj-id`
- `ichiran/dict:conj-prop`
- `ichiran/dict:conj-source-reading`
- `ichiran/dict:conj-type`
- `ichiran/dict:conjugate-p`
- `ichiran/dict:conjugation`
- `ichiran/dict:entry`
- `ichiran/dict:id`
- `ichiran/dict:kana-text`
- `ichiran/dict:kanji-text`
- `ichiran/dict:lex-compare`
- `ichiran/dict:ord`
- `ichiran/dict:pos`
- `ichiran/dict:reading`
- `ichiran/dict:seq`
- `ichiran/dict:source-text`
