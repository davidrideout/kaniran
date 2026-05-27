# find-word-suffix

**Package:** `ichiran/dict`  
**Source:** `dict-grammar.lisp:706`  
**Definition form:** `defun`

## Inputs

`(ichiran/dict::word &key ichiran/dict::matches)`

## Outputs

Declared ftype: `(function (t &key (:matches t)) (values list &optional))`

## Dependencies (ichiran symbols)

- `ichiran/dict:get-suffixes`
- `ichiran/dict:make-slice`
- `ichiran/dict:match-unique`
- `ichiran/dict:seq`
- `ichiran/dict:subseq-slice`

## Source-walked references

- `ichiran/dict:*suffix-class*`
- `ichiran/dict:*suffix-list*`
- `ichiran/dict:*suffix-map-temp*`
- `ichiran/dict:*suffix-next-end*`
- `ichiran/dict:get-suffixes`
- `ichiran/dict:make-slice`
- `ichiran/dict:match-unique`
- `ichiran/dict:seq`
- `ichiran/dict:subseq-slice`
