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

## Source-walked references

- `ichiran/dict:accum`
- `ichiran/dict:across`
- `ichiran/dict:collect`
- `ichiran/dict:expand-segment-list`
- `ichiran/dict:for`
- `ichiran/dict:gap-left`
- `ichiran/dict:gap-penalty`
- `ichiran/dict:gap-right`
- `ichiran/dict:get-array`
- `ichiran/dict:get-seg-initial`
- `ichiran/dict:get-seg-splits`
- `ichiran/dict:get-segment-score`
- `ichiran/dict:in`
- `ichiran/dict:initial-segs`
- `ichiran/dict:limit`
- `ichiran/dict:on`
- `ichiran/dict:path`
- `ichiran/dict:register-item`
- `ichiran/dict:score-tail`
- `ichiran/dict:score1`
- `ichiran/dict:score2`
- `ichiran/dict:score3`
- `ichiran/dict:seg`
- `ichiran/dict:seg-left`
- `ichiran/dict:seg1`
- `ichiran/dict:seg2`
- `ichiran/dict:segment`
- `ichiran/dict:segment-list`
- `ichiran/dict:segment-list-end`
- `ichiran/dict:segment-list-start`
- `ichiran/dict:segment-list-top`
- `ichiran/dict:segment-lists`
- `ichiran/dict:split`
- `ichiran/dict:str-length`
- `ichiran/dict:tai`
- `ichiran/dict:tai-payload`
- `ichiran/dict:tai-score`
- `ichiran/dict:tail`
- `ichiran/dict:top`
- `ichiran/dict:top-array`
- `ichiran/dict:with`
