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
- `ichiran/dict:always`
- `ichiran/dict:candidates`
- `ichiran/dict:for`
- `ichiran/dict:get-candidates`
- `ichiran/dict:get-glosses`
- `ichiran/dict:gloss`
- `ichiran/dict:glosses`
- `ichiran/dict:in`
- `ichiran/dict:match`
- `ichiran/dict:matched`
- `ichiran/dict:ngloss`
- `ichiran/dict:nwords`
- `ichiran/dict:reading`
- `ichiran/dict:seq`
- `ichiran/dict:thereis`
- `ichiran/dict:update-gloss`
- `ichiran/dict:word`
- `ichiran/dict:words`
