# get-senses-json

**Package:** `ichiran/dict`  
**Source:** `dict.lisp:1537`  
**Definition form:** `defun`

## Inputs

`(ichiran/dict::seq &key ichiran/dict::pos-list ichiran/dict::reading
                    ichiran/dict::reading-getter)`

## Outputs

Declared ftype: `(function
                  (t &key (:pos-list t) (:reading t) (:reading-getter t))
                  (values list &optional))`

## Dependencies (ichiran symbols)

- `ichiran/characters:join`
- `ichiran/dict:get-senses`
- `ichiran/dict:match-sense-restrictions`
- `ichiran/dict:split-pos`
