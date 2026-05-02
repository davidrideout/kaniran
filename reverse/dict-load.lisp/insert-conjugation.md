# insert-conjugation

**Package:** `ichiran/dict`  
**Source:** `dict-load.lisp:375`  
**Definition form:** `defun`

## Inputs

`(ichiran/dict::readings &key ichiran/dict::seq ichiran/dict::from
  ichiran/dict::pos ichiran/dict::conj-type ichiran/dict::neg ichiran/dict::fml
  ichiran/dict::via)`

## Outputs

Declared ftype: `(function
                  (t &key (:seq t) (:from t) (:pos t) (:conj-type t) (:neg t)
                   (:fml t) (:via t))
                  (values boolean &optional))`

## Dependencies (ichiran symbols)

- `ichiran/dict:id`
- `ichiran/dict:lex-compare`

## Source-walked references

- `ichiran/dict:*secondary-conjugation-types-from*`
- `ichiran/dict:collect`
- `ichiran/dict:conj`
- `ichiran/dict:conj-id`
- `ichiran/dict:conj-prop`
- `ichiran/dict:conj-source-reading`
- `ichiran/dict:conj-type`
- `ichiran/dict:conjugate-p`
- `ichiran/dict:conjugation`
- `ichiran/dict:else`
- `ichiran/dict:entry`
- `ichiran/dict:finally`
- `ichiran/dict:fml`
- `ichiran/dict:for`
- `ichiran/dict:from`
- `ichiran/dict:id`
- `ichiran/dict:in`
- `ichiran/dict:into`
- `ichiran/dict:k`
- `ichiran/dict:k.seq`
- `ichiran/dict:k.text`
- `ichiran/dict:kana-readings`
- `ichiran/dict:kana-text`
- `ichiran/dict:kanji-flag`
- `ichiran/dict:kanji-readings`
- `ichiran/dict:kanji-text`
- `ichiran/dict:kr`
- `ichiran/dict:lex-compare`
- `ichiran/dict:neg`
- `ichiran/dict:old-conj`
- `ichiran/dict:old-csr`
- `ichiran/dict:ord`
- `ichiran/dict:orig-reading`
- `ichiran/dict:pos`
- `ichiran/dict:r`
- `ichiran/dict:r.id`
- `ichiran/dict:r.seq`
- `ichiran/dict:r.text`
- `ichiran/dict:reading`
- `ichiran/dict:readings`
- `ichiran/dict:seq`
- `ichiran/dict:seq-candidates`
- `ichiran/dict:source-readings`
- `ichiran/dict:source-text`
- `ichiran/dict:via`
