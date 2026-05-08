# get-text (generic function)

**Package:** `ichiran/dict`  
**Source:** `dict.lisp`  
**Definition form:** `defgeneric`

## Inputs (generic lambda list)

`(ichiran/dict::obj)`

## Outputs

Docstring: most popular text representation (kanji or kana)

## Dependencies at generic dispatch site

_(none detected)_


## Methods

### method (ICHIRAN/DICT::SEGMENT)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::segment)`

**Dependencies:**

- `ichiran/dict:segment-text`
- `ichiran/dict:segment-word`
- `ichiran/dict:text`

### method (ICHIRAN/DICT::ENTRY)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::obj)`

**Dependencies:**

- `ichiran/dict:n-kanji`
- `ichiran/dict:seq`
- `ichiran/dict:text`

### method (T)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::obj)`

**Dependencies:**

- `ichiran/dict:text`


## Source-walked references

- `ichiran/dict:entry`
- `ichiran/dict:kana-text`
- `ichiran/dict:kanji-text`
- `ichiran/dict:n-kanji`
- `ichiran/dict:ord`
- `ichiran/dict:segment`
- `ichiran/dict:segment-text`
- `ichiran/dict:segment-word`
- `ichiran/dict:seq`
- `ichiran/dict:text`
