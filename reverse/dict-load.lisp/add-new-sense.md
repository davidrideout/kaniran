# add-new-sense

**Package:** `ichiran/dict`  
**Source:** `dict-load.lisp:91`  
**Definition form:** `defun`

## Inputs

`(ichiran/dict::seq ichiran/dict::positions ichiran/dict::glosses &aux
                    (ichiran/dict::senses
                     (ichiran/dict::get-senses-raw ichiran/dict::seq)))`

## Outputs

Declared ftype: `(function (t t t) (values t &optional number))`

## Dependencies (ichiran symbols)

- `ichiran/dict:get-senses-raw`
- `ichiran/dict:id`
- `ichiran/dict:sense-exists-p`
