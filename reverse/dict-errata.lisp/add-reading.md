# add-reading

**Package:** `ichiran/dict`  
**Source:** `dict-errata.lisp:35`  
**Definition form:** `defun`

## Inputs

`(ichiran/dict::seq ichiran/dict::reading &key (ichiran/dict::common :null)
                    (ichiran/dict::conjugate-p t) (ichiran/dict::table nil))`

## Outputs

Declared ftype: `(function (t t &key (:common t) (:conjugate-p t) (:table t))
                  (values t &optional))`

## Dependencies (ichiran symbols)

- `ichiran/characters:test-word`
- `ichiran/dict:n-kana`
- `ichiran/dict:n-kanji`
