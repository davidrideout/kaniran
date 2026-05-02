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
- `ichiran/dict:below`
- `ichiran/dict:d`
- `ichiran/dict:end`
- `ichiran/dict:ends`
- `ichiran/dict:finally`
- `ichiran/dict:find-sticky-positions`
- `ichiran/dict:find-substring-words`
- `ichiran/dict:find-word-full`
- `ichiran/dict:for`
- `ichiran/dict:from`
- `ichiran/dict:get-suffix-map`
- `ichiran/dict:into`
- `ichiran/dict:kanji-break`
- `ichiran/dict:katakana-group-end`
- `ichiran/dict:katakana-groups`
- `ichiran/dict:make-segment`
- `ichiran/dict:nconcing`
- `ichiran/dict:number-group-end`
- `ichiran/dict:number-groups`
- `ichiran/dict:part`
- `ichiran/dict:result`
- `ichiran/dict:segments`
- `ichiran/dict:start`
- `ichiran/dict:sticky`
- `ichiran/dict:str`
- `ichiran/dict:substring-hash`
- `ichiran/dict:suffix-map`
- `ichiran/dict:upto`
- `ichiran/dict:with`
- `ichiran/dict:word`
