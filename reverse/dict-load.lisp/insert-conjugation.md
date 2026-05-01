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
