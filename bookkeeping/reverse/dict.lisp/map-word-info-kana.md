# map-word-info-kana

**Package:** `ichiran/dict`  
**Source:** `dict.lisp:1728`  
**Definition form:** `defun`

## Inputs

`(ichiran/dict::fn ichiran/dict:word-info &key (ichiran/dict::separator "/")
  &aux
  (ichiran/dict::wkana (ichiran/dict:word-info-kana ichiran/dict:word-info)))`

## Outputs

Declared ftype: `(function (t t &key (:separator t)) *)`

## Dependencies (ichiran symbols)

- `ichiran/characters:join`
- `ichiran/dict:simplify-reading-list`
- `ichiran/dict:word-info-kana`

## Source-walked references

- `ichiran/characters:join`
- `ichiran/dict:simplify-reading-list`
- `ichiran/dict:word-info`
- `ichiran/dict:word-info-kana`
