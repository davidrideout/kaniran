# true-kanji (generic function)

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

### method (T)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::obj)`

**Dependencies:**

- `ichiran/dict:get-kanji`

### method (ICHIRAN/DICT::PROXY-TEXT)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::obj)`

**Dependencies:**

- `ichiran/dict:source`
- `ichiran/dict:true-kanji`


## Source-walked references

- `ichiran/dict:get-kanji`
- `ichiran/dict:obj`
- `ichiran/dict:proxy-text`
- `ichiran/dict:source`
