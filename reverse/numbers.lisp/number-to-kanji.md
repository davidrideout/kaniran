# number-to-kanji

**Package:** `ichiran/numbers`  
**Source:** `numbers.lisp:35`  
**Definition form:** `defun`

## Inputs

`(ichiran/numbers::n &rest ichiran/numbers::keys &key
  (ichiran/numbers::digits ichiran/numbers:*digit-kanji-default*)
  (ichiran/numbers::powers ichiran/numbers:*power-kanji*)
  (ichiran/numbers::1sen nil))`

## Outputs

Declared ftype: `(function (t &rest t &key (:digits t) (:powers t) (:1sen t))
                  (values string &optional))`

## Dependencies (ichiran symbols)

- `ichiran/numbers:number-to-kanji`
