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
- `ichiran/dict:attrs`
- `ichiran/dict:calc-score`
- `ichiran/dict:compound-text`
- `ichiran/dict:connector`
- `ichiran/dict:copy-segment`
- `ichiran/dict:for`
- `ichiran/dict:from`
- `ichiran/dict:get-kana`
- `ichiran/dict:get-split`
- `ichiran/dict:get-text`
- `ichiran/dict:i`
- `ichiran/dict:in`
- `ichiran/dict:new-seg`
- `ichiran/dict:primary`
- `ichiran/dict:root`
- `ichiran/dict:score`
- `ichiran/dict:segment`
- `ichiran/dict:segment-info`
- `ichiran/dict:segment-score`
- `ichiran/dict:segment-text`
- `ichiran/dict:segment-word`
- `ichiran/dict:simple-text`
- `ichiran/dict:split`
- `ichiran/dict:word`
- `ichiran/dict:word-conj-data`
- `ichiran/dict:word-conjugations`
