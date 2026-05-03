# calc-score

**Package:** `ichiran/dict`  
**Source:** `dict.lisp:775`  
**Definition form:** `defun`

## Inputs

`(ichiran/dict::reading &key ichiran/dict::final ichiran/dict::use-length
                        (ichiran/dict::score-mod 0) ichiran/dict::kanji-break
                        &aux ichiran/dict::ctr-mode)`

## Outputs

Declared ftype: `(function
                  (t &key (:final t) (:use-length (or null (mod 10001)))
                   (:score-mod t) (:kanji-break t))
                  (values t &optional list))`

## Dependencies (ichiran symbols)

- `ichiran/characters:count-char-class`
- `ichiran/characters:mora-length`
- `ichiran/dict:*copulae*`
- `ichiran/dict:*final-prt*`
- `ichiran/dict:*non-final-prt*`
- `ichiran/dict:*semi-final-prt*`
- `ichiran/dict:*skip-words*`
- `ichiran/dict:*weak-conj-forms*`
- `ichiran/dict:apply-score-mod`
- `ichiran/dict:args`
- `ichiran/dict:calc-score`
- `ichiran/dict:cnt`
- `ichiran/dict:collect`
- `ichiran/dict:common`
- `ichiran/dict:common-bonus`
- `ichiran/dict:common-of`
- `ichiran/dict:common-p`
- `ichiran/dict:compare-common`
- `ichiran/dict:compound-text`
- `ichiran/dict:conj-data`
- `ichiran/dict:conj-data-from`
- `ichiran/dict:conj-data-prop`
- `ichiran/dict:conj-data-via`
- `ichiran/dict:conj-of`
- `ichiran/dict:conj-of-common`
- `ichiran/dict:conj-of-data`
- `ichiran/dict:conj-of-ord`
- `ichiran/dict:conj-only`
- `ichiran/dict:conj-props`
- `ichiran/dict:conj-type`
- `ichiran/dict:conj-types`
- `ichiran/dict:conj-types-p`
- `ichiran/dict:cop-da-p`
- `ichiran/dict:counter-text`
- `ichiran/dict:ctr-mode`
- `ichiran/dict:entry`
- `ichiran/dict:final`
- `ichiran/dict:finally`
- `ichiran/dict:for`
- `ichiran/dict:from`
- `ichiran/dict:get-non-arch-posi`
- `ichiran/dict:get-original-text`
- `ichiran/dict:get-split`
- `ichiran/dict:id`
- `ichiran/dict:in`
- `ichiran/dict:info`
- `ichiran/dict:into`
- `ichiran/dict:is-arch`
- `ichiran/dict:kanji-break`
- `ichiran/dict:kanji-break-penalty`
- `ichiran/dict:kanji-p`
- `ichiran/dict:katakana-p`
- `ichiran/dict:len`
- `ichiran/dict:length-multiplier-coeff`
- `ichiran/dict:long-p`
- `ichiran/dict:n-kanji`
- `ichiran/dict:new-len`
- `ichiran/dict:new-prop-score`
- `ichiran/dict:no-common-bonus`
- `ichiran/dict:nokanji`
- `ichiran/dict:non-final-particle-p`
- `ichiran/dict:nparts`
- `ichiran/dict:of-type`
- `ichiran/dict:ord`
- `ichiran/dict:ot`
- `ichiran/dict:part`
- `ichiran/dict:part-score`
- `ichiran/dict:part-scores`
- `ichiran/dict:particle-p`
- `ichiran/dict:plen`
- `ichiran/dict:pmlen`
- `ichiran/dict:posi`
- `ichiran/dict:prefer-kana`
- `ichiran/dict:primary-nokanji`
- `ichiran/dict:primary-p`
- `ichiran/dict:pronoun-p`
- `ichiran/dict:prop`
- `ichiran/dict:prop-score`
- `ichiran/dict:proxy-text`
- `ichiran/dict:ptext`
- `ichiran/dict:reading`
- `ichiran/dict:root-p`
- `ichiran/dict:row`
- `ichiran/dict:score`
- `ichiran/dict:score-base`
- `ichiran/dict:score-mod`
- `ichiran/dict:score-mod-split`
- `ichiran/dict:secondary-conj-p`
- `ichiran/dict:semi-final-particle-p`
- `ichiran/dict:sense`
- `ichiran/dict:sense-id`
- `ichiran/dict:sense-prop`
- `ichiran/dict:seq`
- `ichiran/dict:seq-set`
- `ichiran/dict:skip-by-conj-data`
- `ichiran/dict:slen`
- `ichiran/dict:smlen`
- `ichiran/dict:sp-seq-set`
- `ichiran/dict:split`
- `ichiran/dict:split-info`
- `ichiran/dict:tag`
- `ichiran/dict:test-conj-prop`
- `ichiran/dict:then`
- `ichiran/dict:tpart`
- `ichiran/dict:true-text`
- `ichiran/dict:use-length`
- `ichiran/dict:use-length-bonus`
- `ichiran/dict:wc`
- `ichiran/dict:with`
- `ichiran/dict:word-conj-data`
- `ichiran/dict:word-conjugations`
- `ichiran/dict:word-type`

## Source-walked references

- `ichiran/characters:count-char-class`
- `ichiran/characters:mora-length`
- `ichiran/dict:*copulae*`
- `ichiran/dict:*final-prt*`
- `ichiran/dict:*non-final-prt*`
- `ichiran/dict:*semi-final-prt*`
- `ichiran/dict:*skip-words*`
- `ichiran/dict:*weak-conj-forms*`
- `ichiran/dict:apply-score-mod`
- `ichiran/dict:common`
- `ichiran/dict:compare-common`
- `ichiran/dict:compound-text`
- `ichiran/dict:conj-data`
- `ichiran/dict:conj-data-from`
- `ichiran/dict:conj-data-prop`
- `ichiran/dict:conj-data-via`
- `ichiran/dict:conj-type`
- `ichiran/dict:counter-text`
- `ichiran/dict:entry`
- `ichiran/dict:get-non-arch-posi`
- `ichiran/dict:get-original-text`
- `ichiran/dict:get-split`
- `ichiran/dict:id`
- `ichiran/dict:is-arch`
- `ichiran/dict:kanji-break-penalty`
- `ichiran/dict:length-multiplier-coeff`
- `ichiran/dict:n-kanji`
- `ichiran/dict:nokanji`
- `ichiran/dict:ord`
- `ichiran/dict:primary-nokanji`
- `ichiran/dict:proxy-text`
- `ichiran/dict:reading`
- `ichiran/dict:root-p`
- `ichiran/dict:score-base`
- `ichiran/dict:score-mod`
- `ichiran/dict:sense`
- `ichiran/dict:sense-id`
- `ichiran/dict:sense-prop`
- `ichiran/dict:seq`
- `ichiran/dict:skip-by-conj-data`
- `ichiran/dict:tag`
- `ichiran/dict:test-conj-prop`
- `ichiran/dict:true-text`
- `ichiran/dict:word-conj-data`
- `ichiran/dict:word-conjugations`
- `ichiran/dict:word-type`
