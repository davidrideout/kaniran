# word-conjugations (generic function)

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

_(none detected)_

### method (ICHIRAN/DICT::COMPOUND-TEXT)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::word)`

**Dependencies:**

- `ichiran/dict:word-conjugations`
- `ichiran/dict:words`

### method (ICHIRAN/DICT::PROXY-TEXT)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::obj)`

**Dependencies:**

- `ichiran/dict:source`
- `ichiran/dict:word-conjugations`

### method (ICHIRAN/DICT::SIMPLE-TEXT)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::simple-text)`

**Dependencies:**

_(none detected)_


## Source-walked references

- `ichiran/dict:compound-text`
- `ichiran/dict:counter-text`
- `ichiran/dict:proxy-text`
- `ichiran/dict:source`
- `ichiran/dict:words`
