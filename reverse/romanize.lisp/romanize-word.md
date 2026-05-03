# romanize-word

**Package:** `ichiran`  
**Source:** `romanize.lisp:217`  
**Definition form:** `defun`

## Inputs

`(ichiran::word &key (method ichiran:*default-romanization-method*)
  ichiran::original-spelling (ichiran/characters:normalize t))`

## Outputs

Declared ftype: `(function
                  (t &key (:method t) (:original-spelling t) (:normalize t)) *)`

## Dependencies (ichiran symbols)

- `ichiran/characters:normalize`
- `ichiran/dict:process-hints`
- `ichiran:get-character-classes`
- `ichiran:r-special`
- `ichiran:romanize-list`

## Source-walked references

- `ichiran/characters:normalize`
- `ichiran/dict:process-hints`
- `ichiran:*default-romanization-method*`
- `ichiran:get-character-classes`
- `ichiran:r-special`
- `ichiran:romanize-list`
