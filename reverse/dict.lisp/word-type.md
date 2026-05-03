# word-type (generic function)

**Package:** `ichiran/dict`  
**Source:** `dict.lisp`  
**Definition form:** `defgeneric`

## Inputs (generic lambda list)

`(ichiran/dict::obj)`

## Outputs

Docstring: returns :kanji or :kana or :gap

## Dependencies at generic dispatch site

_(none detected)_


## Methods

### method (ICHIRAN/DICT::COUNTER-TEXT)

**Source:** `dict-counters.lisp`  
**Inputs:** `(ichiran/dict::obj)`

**Dependencies:**

- `ichiran/characters:count-char-class`

### method (ICHIRAN/DICT::COMPOUND-TEXT)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::obj)`

**Dependencies:**

- `ichiran/dict:primary`
- `ichiran/dict:word-type`

### method (ICHIRAN/DICT::PROXY-TEXT)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::obj)`

**Dependencies:**

- `ichiran/dict:source`
- `ichiran/dict:word-type`

### method (ICHIRAN/DICT::KANA-TEXT)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::obj)`

**Dependencies:**

_(none detected)_

### method (ICHIRAN/DICT::KANJI-TEXT)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::obj)`

**Dependencies:**

_(none detected)_

### method (T)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::obj)`

**Dependencies:**

_(none detected)_


## Source-walked references

- `ichiran/characters:count-char-class`
- `ichiran/dict:compound-text`
- `ichiran/dict:counter-text`
- `ichiran/dict:kana-text`
- `ichiran/dict:kanji-text`
- `ichiran/dict:primary`
- `ichiran/dict:proxy-text`
- `ichiran/dict:source`
