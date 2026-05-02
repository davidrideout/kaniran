# find-word-as-hiragana

**Package:** `ichiran/dict`  
**Source:** `dict.lisp:592`  
**Definition form:** `defun`

## Inputs

`(ichiran/dict::str &key ichiran/dict::exclude ichiran/dict::finder)`

## Outputs

Declared ftype: `(function (t &key (:exclude t) (:finder t))
                  (values list &optional))`

## Dependencies (ichiran symbols)

- `ichiran/characters:as-hiragana`
- `ichiran/dict:find-word`
- `ichiran/dict:seq`

## Source-walked references

- `ichiran/characters:as-hiragana`
- `ichiran/dict:collect`
- `ichiran/dict:exclude`
- `ichiran/dict:find-word`
- `ichiran/dict:finder`
- `ichiran/dict:for`
- `ichiran/dict:in`
- `ichiran/dict:proxy-text`
- `ichiran/dict:seq`
- `ichiran/dict:str`
- `ichiran/dict:w`
- `ichiran/dict:words`
