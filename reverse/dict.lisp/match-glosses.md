# match-glosses

**Package:** `ichiran/dict`  
**Source:** `dict.lisp:1921`  
**Definition form:** `defun`

## Inputs

`(s-sql:text ichiran/dict::reading ichiran/dict::words &key
             (ichiran/characters:normalize 'identity)
             ichiran/dict::update-gloss)`

## Outputs

Declared ftype: `(function (t t t &key (:normalize t) (:update-gloss t))
                  (values t &optional boolean))`

## Dependencies (ichiran symbols)

- `ichiran/dict:get-candidates`
- `ichiran/dict:get-glosses`
