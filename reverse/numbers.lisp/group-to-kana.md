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

## Source-walked references

- `ichiran/numbers:class-to-kana`
- `ichiran/numbers:finally`
- `ichiran/numbers:for`
- `ichiran/numbers:group`
- `ichiran/numbers:in`
- `ichiran/numbers:kana`
- `ichiran/numbers:last-class`
- `ichiran/numbers:last-val`
- `ichiran/numbers:num-sandhi`
- `ichiran/numbers:result`
- `ichiran/numbers:val`
- `ichiran/numbers:with`
