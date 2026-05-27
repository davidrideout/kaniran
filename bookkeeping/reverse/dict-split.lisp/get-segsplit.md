# get-segsplit

**Package:** `ichiran/dict`  
**Source:** `dict-split.lisp:823`  
**Definition form:** `defun`

## Inputs

`(ichiran/dict::segment &aux
  (ichiran/dict::word (ichiran/dict::segment-word ichiran/dict::segment)))`

## Outputs

Declared ftype: `(function (t)
                  (values (or ichiran/dict::segment null) &optional))`

## Dependencies (ichiran symbols)

- `ichiran/characters:join`
- `ichiran/dict:calc-score`
- `ichiran/dict:copy-segment`
- `ichiran/dict:get-kana`
- `ichiran/dict:get-split`
- `ichiran/dict:get-text`
- `ichiran/dict:primary`
- `ichiran/dict:segment-info`
- `ichiran/dict:segment-score`
- `ichiran/dict:segment-word`
- `ichiran/dict:word-conj-data`

## Source-walked references

- `ichiran/characters:join`
- `ichiran/dict:*segsplit-map*`
- `ichiran/dict:*split-map*`
- `ichiran/dict:calc-score`
- `ichiran/dict:compound-text`
- `ichiran/dict:copy-segment`
- `ichiran/dict:get-kana`
- `ichiran/dict:get-split`
- `ichiran/dict:get-text`
- `ichiran/dict:primary`
- `ichiran/dict:segment`
- `ichiran/dict:segment-info`
- `ichiran/dict:segment-score`
- `ichiran/dict:segment-text`
- `ichiran/dict:segment-word`
- `ichiran/dict:simple-text`
- `ichiran/dict:word-conj-data`
- `ichiran/dict:word-conjugations`
