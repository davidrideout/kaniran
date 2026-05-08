# common (generic function)

**Package:** `ichiran/dict`  
**Source:** `dict-counters.lisp`  
**Definition form:** `defgeneric`

## Inputs (generic lambda list)

`(sb-pcl::object)`

## Outputs

_unknown — no docstring_

## Dependencies at generic dispatch site

_(none detected)_


## Methods

### method (ICHIRAN/DICT::COUNTER-TEXT)

**Source:** `dict-counters.lisp`  
**Inputs:** `(ichiran/dict::obj)`

**Dependencies:**

- `ichiran/dict:common`
- `ichiran/dict:counter-common`
- `ichiran/dict:source`

### method (ICHIRAN/DICT::COMPOUND-TEXT)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::obj)`

**Dependencies:**

- `ichiran/dict:common`
- `ichiran/dict:primary`

### method (ICHIRAN/DICT::PROXY-TEXT)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::obj)`

**Dependencies:**

- `ichiran/dict:common`
- `ichiran/dict:source`

### method (ICHIRAN/DICT::ENTRY)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::obj &aux
              (ichiran/dict::seq (ichiran/dict::seq ichiran/dict::obj)))`

**Dependencies:**

- `ichiran/dict:seq`

### method (ICHIRAN/DICT::KANA-TEXT)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::kana-text)`

**Dependencies:**

_(none detected)_

### method (ICHIRAN/DICT::KANJI-TEXT)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::kanji-text)`

**Dependencies:**

_(none detected)_


## Source-walked references

- `ichiran/dict:compound-text`
- `ichiran/dict:counter-common`
- `ichiran/dict:counter-text`
- `ichiran/dict:entry`
- `ichiran/dict:kana-text`
- `ichiran/dict:kanji-text`
- `ichiran/dict:primary`
- `ichiran/dict:proxy-text`
- `ichiran/dict:seq`
- `ichiran/dict:source`
