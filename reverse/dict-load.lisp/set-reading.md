# set-reading (generic function)

**Package:** `ichiran/dict`  
**Source:** `dict-load.lisp`  
**Definition form:** `defgeneric`

## Inputs (generic lambda list)

`(ichiran/dict::obj)`

## Outputs

Docstring: find and set best associated reading (kana/kanji) for this object

## Dependencies at generic dispatch site

_(none detected)_


## Methods

### method (ICHIRAN/DICT::KANA-TEXT)

**Source:** `dict-load.lisp`  
**Inputs:** `(ichiran/dict::obj)`

**Dependencies:**

- `ichiran/dict:best-kanji`
- `ichiran/dict:nokanji`
- `ichiran/dict:seq`

### method (ICHIRAN/DICT::KANJI-TEXT)

**Source:** `dict-load.lisp`  
**Inputs:** `(ichiran/dict::obj)`

**Dependencies:**

- `ichiran/dict:best-kana`
- `ichiran/dict:nokanji`
- `ichiran/dict:seq`

