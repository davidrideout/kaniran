# romanize

**Package:** `ichiran`  
**Source:** `romanize.lisp:257`  
**Definition form:** `defun`

## Inputs

`(ichiran::input &key (method ichiran:*default-romanization-method*)
  (ichiran::with-info nil))`

## Outputs

Declared ftype: `(function (t &key (:method t) (:with-info t))
                  (values simple-string list &optional))`

## Dependencies (ichiran symbols)

- `ichiran/characters:basic-split`
- `ichiran/characters:normalize`
- `ichiran/dict:simple-segment`
- `ichiran/dict:word-info-str`
- `ichiran:join-parts`
- `ichiran:romanize-word-info`

## Source-walked references

- `ichiran/characters:basic-split`
- `ichiran/characters:normalize`
- `ichiran/dict:simple-segment`
- `ichiran/dict:word-info-str`
- `ichiran:*default-romanization-method*`
- `ichiran:definitions`
- `ichiran:finally`
- `ichiran:for`
- `ichiran:in`
- `ichiran:input`
- `ichiran:into`
- `ichiran:join-parts`
- `ichiran:parts`
- `ichiran:rom`
- `ichiran:romanize-word-info`
- `ichiran:split-text`
- `ichiran:split-type`
- `ichiran:with`
- `ichiran:with-info`
- `ichiran:word`
