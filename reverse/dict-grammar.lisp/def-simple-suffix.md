# def-simple-suffix

**Package:** `ichiran/dict`  
**Source:** `dict-grammar.lisp:345`  
**Definition form:** `defmacro`

## Inputs

`(ichiran/dict::name keyword
  (&key (ichiran/dict::stem 0) (ichiran/dict::score 0)
   (ichiran/dict::connector ""))
  (ichiran/dict::root-var &optional ichiran/dict::suf-var
   ichiran/dict::patch-var)
  &body ichiran/dict::get-primary-words)`

## Outputs

_unknown — no declared ftype, no docstring_

## Dependencies (ichiran symbols)

_(none detected)_

## Source-walked references

- `ichiran/characters:destem`
- `ichiran/dict:*suffix-map-temp*`
- `ichiran/dict:adjoin-word`
- `ichiran/dict:connector`
- `ichiran/dict:defsuffix`
- `ichiran/dict:get-kana`
- `ichiran/dict:get-primary-words`
- `ichiran/dict:k`
- `ichiran/dict:name`
- `ichiran/dict:patch-var`
- `ichiran/dict:primary-words`
- `ichiran/dict:pw`
- `ichiran/dict:root-var`
- `ichiran/dict:score`
- `ichiran/dict:score-base`
- `ichiran/dict:stem`
- `ichiran/dict:suf`
- `ichiran/dict:suf-var`
