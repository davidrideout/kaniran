# group-to-kana

**Package:** `ichiran/numbers`  
**Source:** `numbers.lisp:117`  
**Definition form:** `defun`

## Inputs

`(ichiran/numbers::group &key
  (ichiran/numbers::class-to-kana
   `(:jd ,ichiran/numbers::*digit-to-kana* :p
     ,ichiran/numbers::*power-to-kana*)))`

## Outputs

Declared ftype: `(function (t &key (:class-to-kana t)) (values t &optional))`

## Dependencies (ichiran symbols)

- `ichiran/numbers:num-sandhi`
