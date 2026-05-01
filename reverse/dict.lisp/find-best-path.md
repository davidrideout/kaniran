# find-best-path

**Package:** `ichiran/dict`  
**Source:** `dict.lisp:1190`  
**Definition form:** `defun`

## Inputs

`(ichiran/dict::segment-lists ichiran/dict::str-length &key
  (ichiran/dict::limit 5))`

## Outputs

Declared ftype: `(function (t t &key (:limit t)) (values list &optional))`

## Dependencies (ichiran symbols)

- `ichiran/dict:expand-segment-list`
- `ichiran/dict:gap-penalty`
- `ichiran/dict:get-array`
- `ichiran/dict:get-seg-initial`
- `ichiran/dict:get-seg-splits`
- `ichiran/dict:get-segment-score`
- `ichiran/dict:register-item`
- `ichiran/dict:segment-list-end`
- `ichiran/dict:segment-list-start`
- `ichiran/dict:segment-list-top`
- `ichiran/dict:tai-payload`
- `ichiran/dict:tai-score`
