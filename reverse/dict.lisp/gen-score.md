# gen-score

**Package:** `ichiran/dict`  
**Source:** `dict.lisp:985`  
**Definition form:** `defun`

## Inputs

`(ichiran/dict::segment &key ichiran/dict::final ichiran/dict::kanji-break)`

## Outputs

Declared ftype: `(function (t &key (:final t) (:kanji-break t))
                  (values ichiran/dict::segment &optional))`

## Dependencies (ichiran symbols)

- `ichiran/dict:calc-score`
- `ichiran/dict:segment-word`

## Source-walked references

- `ichiran/dict:calc-score`
- `ichiran/dict:final`
- `ichiran/dict:kanji-break`
- `ichiran/dict:segment`
- `ichiran/dict:segment-info`
- `ichiran/dict:segment-score`
- `ichiran/dict:segment-word`
