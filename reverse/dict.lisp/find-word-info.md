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
- `ichiran/dict:exists-reading`
- `ichiran/dict:find-word`
- `ichiran/dict:find-word-full`
- `ichiran/dict:gen-score`
- `ichiran/dict:get-suffix-map`
- `ichiran/dict:make-segment`
- `ichiran/dict:reading`
- `ichiran/dict:segment-score`
- `ichiran/dict:seq`
- `ichiran/dict:word-info-from-segment`
- `ichiran/dict:word-info-kana`
- `ichiran/dict:word-info-seq`
