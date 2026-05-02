# find-word-info

**Package:** `ichiran/dict`  
**Source:** `dict.lisp:1850`  
**Definition form:** `defun`

## Inputs

`(s-sql:text &key ichiran/dict::reading ichiran/dict::root-only &aux
             (ichiran/dict::end (length s-sql:text)))`

## Outputs

Declared ftype: `(function (t &key (:reading t) (:root-only t))
                  (values list &optional))`

## Dependencies (ichiran symbols)

- `ichiran/characters:test-word`
- `ichiran/dict:exists-reading`
- `ichiran/dict:find-word`
- `ichiran/dict:find-word-full`
- `ichiran/dict:gen-score`
- `ichiran/dict:get-suffix-map`
- `ichiran/dict:make-segment`
- `ichiran/dict:segment-score`
- `ichiran/dict:word-info-from-segment`
- `ichiran/dict:word-info-kana`
- `ichiran/dict:word-info-seq`

## Source-walked references

- `ichiran/characters:test-word`
- `ichiran/conn:*connection*`
- `ichiran/dict:*suffix-map-temp*`
- `ichiran/dict:*suffix-next-end*`
- `ichiran/dict:all-words`
- `ichiran/dict:collect`
- `ichiran/dict:else`
- `ichiran/dict:end`
- `ichiran/dict:exists-reading`
- `ichiran/dict:find-word`
- `ichiran/dict:find-word-full`
- `ichiran/dict:for`
- `ichiran/dict:gen-score`
- `ichiran/dict:get-suffix-map`
- `ichiran/dict:in`
- `ichiran/dict:make-segment`
- `ichiran/dict:reading`
- `ichiran/dict:root-only`
- `ichiran/dict:segment-score`
- `ichiran/dict:segments`
- `ichiran/dict:seq`
- `ichiran/dict:wi`
- `ichiran/dict:wis`
- `ichiran/dict:word`
- `ichiran/dict:word-info-from-segment`
- `ichiran/dict:word-info-kana`
- `ichiran/dict:word-info-seq`
