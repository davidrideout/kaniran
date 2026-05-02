# reading-str (generic function)

**Package:** `ichiran/dict`  
**Source:** `dict.lisp`  
**Definition form:** `defgeneric`

## Inputs (generic lambda list)

`(ichiran/dict::obj)`

## Outputs

_unknown — no docstring_

## Dependencies at generic dispatch site

_(none detected)_


## Methods

### method (LIST)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict:word-info)`

**Dependencies:**

- `ichiran/dict:word-info-reading-str`

### method (ICHIRAN/DICT:WORD-INFO)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict:word-info)`

**Dependencies:**

- `ichiran/dict:word-info-reading-str`

### method (ICHIRAN/DICT::SIMPLE-TEXT)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::obj)`

**Dependencies:**

- `ichiran/dict:get-kana`
- `ichiran/dict:get-kanji`
- `ichiran/dict:reading-str*`

### method (INTEGER)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::obj)`

**Dependencies:**

- `ichiran/dict:reading-str-seq`


## Source-walked references

- `ichiran/dict:get-kana`
- `ichiran/dict:get-kanji`
- `ichiran/dict:obj`
- `ichiran/dict:reading-str*`
- `ichiran/dict:reading-str-seq`
- `ichiran/dict:simple-text`
- `ichiran/dict:word-info`
- `ichiran/dict:word-info-reading-str`
