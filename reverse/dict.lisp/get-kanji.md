# get-kanji (generic function)

**Package:** `ichiran/dict`  
**Source:** `dict.lisp`  
**Definition form:** `defgeneric`

## Inputs (generic lambda list)

`(ichiran/dict::obj)`

## Outputs

Docstring: most popular kanji representation

## Dependencies at generic dispatch site

_(none detected)_


## Methods

### method (ICHIRAN/DICT::COUNTER-TEXT)

**Source:** `dict-counters.lisp`  
**Inputs:** `(ichiran/dict::obj)`

**Dependencies:**

- `ichiran/dict:counter-text`
- `ichiran/dict:number-value`
- `ichiran/numbers:number-to-kanji`

### method (ICHIRAN/DICT::KANA-TEXT)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::obj)`

**Dependencies:**

- `ichiran/dict:best-kanji-conj`

### method (ICHIRAN/DICT::KANJI-TEXT)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::obj)`

**Dependencies:**

_(none detected)_

### method (ICHIRAN/DICT::ENTRY)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::obj)`

**Dependencies:**

- `ichiran/dict:n-kanji`
- `ichiran/dict:seq`


## Source-walked references

- `ichiran/dict:best-kanji-conj`
- `ichiran/dict:bk`
- `ichiran/dict:counter-text`
- `ichiran/dict:entry`
- `ichiran/dict:kana-text`
- `ichiran/dict:kanji-text`
- `ichiran/dict:n-kanji`
- `ichiran/dict:number-value`
- `ichiran/dict:obj`
- `ichiran/dict:ord`
- `ichiran/dict:seq`
- `ichiran/numbers:number-to-kanji`
