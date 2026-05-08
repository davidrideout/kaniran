# seq (generic function)

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

- `ichiran/dict:seq`
- `ichiran/dict:source`

### method (ICHIRAN/DICT::COMPOUND-TEXT)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::obj)`

**Dependencies:**

- `ichiran/dict:seq`
- `ichiran/dict:words`

### method (ICHIRAN/DICT::PROXY-TEXT)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::obj)`

**Dependencies:**

- `ichiran/dict:seq`
- `ichiran/dict:source`

### method (ICHIRAN/DICT::CONJUGATION)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::conjugation)`

**Dependencies:**

_(none detected)_

### method (ICHIRAN/DICT::RESTRICTED-READINGS)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::restricted-readings)`

**Dependencies:**

_(none detected)_

### method (ICHIRAN/DICT::SENSE-PROP)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::sense-prop)`

**Dependencies:**

_(none detected)_

### method (ICHIRAN/DICT::SENSE)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::sense)`

**Dependencies:**

_(none detected)_

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

### method (ICHIRAN/DICT::ENTRY)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::entry)`

**Dependencies:**

_(none detected)_


## Source-walked references

- `ichiran/dict:compound-text`
- `ichiran/dict:counter-text`
- `ichiran/dict:proxy-text`
- `ichiran/dict:source`
- `ichiran/dict:words`
