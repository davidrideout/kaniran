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
