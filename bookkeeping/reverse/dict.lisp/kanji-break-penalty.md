# kanji-break-penalty

**Package:** `ichiran/dict`  
**Source:** `dict.lisp:702`  
**Definition form:** `defun`

## Inputs

`(ichiran/dict::kanji-break ichiran/dict::score &key ichiran/dict::info
  s-sql:text ichiran/dict::use-length ichiran/dict::score-mod)`

## Outputs

Declared ftype: `(function
                  (t t &key (:info t) (:text t) (:use-length t) (:score-mod t))
                  (values t &optional))`

## Dependencies (ichiran symbols)

- `ichiran/characters:mora-length`
- `ichiran/dict:calc-score`
- `ichiran/dict:get-suffixes`

## Source-walked references

- `ichiran/characters:mora-length`
- `ichiran/dict:*no-kanji-break-penalty*`
- `ichiran/dict:*score-cutoff*`
- `ichiran/dict:calc-score`
- `ichiran/dict:get-suffixes`
- `ichiran/dict:score-mod`
- `ichiran/dict:text`
