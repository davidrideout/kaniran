# load-entry

**Package:** `ichiran/dict`  
**Source:** `dict-load.lisp:113`  
**Definition form:** `defun`

## Inputs

`(ichiran/dict::content &key ichiran/dict::if-exists ichiran/dict::upstream
                        ichiran/dict::seq ichiran/dict::conjugate-p)`

## Outputs

Declared ftype: `(function
                  (t &key (:if-exists t) (:upstream t) (:seq t)
                   (:conjugate-p t))
                  (values t &optional))`

## Dependencies (ichiran symbols)

- `ichiran/dict:conjugate-entry-outer`
- `ichiran/dict:find-word`
- `ichiran/dict:get-text`
- `ichiran/dict:insert-readings`
- `ichiran/dict:insert-senses`
- `ichiran/dict:load-secondary-conjugations`
- `ichiran/dict:next-seq`
- `ichiran/dict:node-text`
- `ichiran/dict:seq`

## Source-walked references

- `ichiran/dict:*pos-with-conj-rules*`
- `ichiran/dict:conjugate-entry-outer`
- `ichiran/dict:conjugate-p`
- `ichiran/dict:content`
- `ichiran/dict:entry`
- `ichiran/dict:find-word`
- `ichiran/dict:get-text`
- `ichiran/dict:insert-readings`
- `ichiran/dict:insert-senses`
- `ichiran/dict:kana-text`
- `ichiran/dict:kanji-text`
- `ichiran/dict:load-secondary-conjugations`
- `ichiran/dict:next-seq`
- `ichiran/dict:node-text`
- `ichiran/dict:sense-prop`
- `ichiran/dict:seq`
- `ichiran/dict:tag`
