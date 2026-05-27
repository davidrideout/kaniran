# join-substring-words*

**Package:** `ichiran/dict`  
**Source:** `dict.lisp:1069`  
**Definition form:** `defun`

## Inputs

`(ichiran/dict::str)`

## Outputs

Declared ftype: `(function (t)
                  (values list
                          (or list (simple-array * (*))
                              sb-kernel:extended-sequence)
                          &optional))`

## Dependencies (ichiran symbols)

- `ichiran/characters:consecutive-char-groups`
- `ichiran/characters:sequential-kanji-positions`
- `ichiran/dict:find-sticky-positions`
- `ichiran/dict:find-substring-words`
- `ichiran/dict:find-word-full`
- `ichiran/dict:get-suffix-map`
- `ichiran/dict:make-segment`

## Source-walked references

- `ichiran/characters:consecutive-char-groups`
- `ichiran/characters:sequential-kanji-positions`
- `ichiran/dict:*force-kanji-break*`
- `ichiran/dict:*max-word-length*`
- `ichiran/dict:*no-kanji-break*`
- `ichiran/dict:*substring-hash*`
- `ichiran/dict:*suffix-map-temp*`
- `ichiran/dict:*suffix-next-end*`
- `ichiran/dict:find-sticky-positions`
- `ichiran/dict:find-substring-words`
- `ichiran/dict:find-word-full`
- `ichiran/dict:get-suffix-map`
- `ichiran/dict:make-segment`
