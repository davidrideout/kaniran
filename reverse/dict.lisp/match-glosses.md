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

## Source-walked references

- `ichiran/characters:normalize`
- `ichiran/conn:*connection*`
- `ichiran/dict:get-candidates`
- `ichiran/dict:get-glosses`
- `ichiran/dict:gloss`
- `ichiran/dict:reading`
- `ichiran/dict:seq`
- `ichiran/dict:words`
