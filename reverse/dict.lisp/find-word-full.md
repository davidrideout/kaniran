# find-word-full

**Package:** `ichiran/dict`  
**Source:** `dict.lisp:1052`  
**Definition form:** `defun`

## Inputs

`(ichiran/dict::word &key ichiran/characters:as-hiragana ichiran/dict::counter)`

## Outputs

Declared ftype: `(function (t &key (:as-hiragana t) (:counter t))
                  (values t &optional))`

## Dependencies (ichiran symbols)

- `ichiran/characters:consecutive-char-groups`
- `ichiran/dict:find-counter`
- `ichiran/dict:find-word`
- `ichiran/dict:find-word-as-hiragana`
- `ichiran/dict:find-word-suffix`
- `ichiran/dict:seq`
