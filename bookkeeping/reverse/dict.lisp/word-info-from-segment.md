# word-info-from-segment

**Package:** `ichiran/dict`  
**Source:** `dict.lisp:1327`  
**Definition form:** `defun`

## Inputs

`(ichiran/dict::segment &aux
  (ichiran/dict::word (ichiran/dict::segment-word ichiran/dict::segment)))`

## Outputs

Declared ftype: `(function (t) *)`

## Dependencies (ichiran symbols)

- `ichiran/dict:get-kana`
- `ichiran/dict:get-text`
- `ichiran/dict:ordinalp`
- `ichiran/dict:primary`
- `ichiran/dict:segment-end`
- `ichiran/dict:segment-score`
- `ichiran/dict:segment-start`
- `ichiran/dict:segment-word`
- `ichiran/dict:seq`
- `ichiran/dict:true-text`
- `ichiran/dict:value-string`
- `ichiran/dict:word-conjugations`
- `ichiran/dict:word-type`
- `ichiran/dict:words`

## Source-walked references

- `ichiran/dict:compound-text`
- `ichiran/dict:counter-text`
- `ichiran/dict:get-kana`
- `ichiran/dict:get-text`
- `ichiran/dict:ordinalp`
- `ichiran/dict:primary`
- `ichiran/dict:segment`
- `ichiran/dict:segment-end`
- `ichiran/dict:segment-score`
- `ichiran/dict:segment-start`
- `ichiran/dict:segment-word`
- `ichiran/dict:seq`
- `ichiran/dict:simple-text`
- `ichiran/dict:true-text`
- `ichiran/dict:value-string`
- `ichiran/dict:word-conjugations`
- `ichiran/dict:word-info`
- `ichiran/dict:word-type`
- `ichiran/dict:words`
