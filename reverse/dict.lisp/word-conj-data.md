# word-conj-data (generic function)

**Package:** `ichiran/dict`  
**Source:** `dict.lisp`  
**Definition form:** `defgeneric`

## Inputs (generic lambda list)

`(ichiran/dict::word)`

## Outputs

Docstring: conjugation data for word

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

- `ichiran/dict:word-conj-data`
- `ichiran/dict:words`

### method (ICHIRAN/DICT::SIMPLE-TEXT)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::word)`

**Dependencies:**

- `ichiran/dict:get-conj-data`
- `ichiran/dict:seq`
- `ichiran/dict:true-text`
- `ichiran/dict:word-conjugations`


## Source-walked references

- `ichiran/dict:compound-text`
- `ichiran/dict:counter-text`
- `ichiran/dict:get-conj-data`
- `ichiran/dict:obj`
- `ichiran/dict:seq`
- `ichiran/dict:simple-text`
- `ichiran/dict:true-text`
- `ichiran/dict:word`
- `ichiran/dict:word-conjugations`
- `ichiran/dict:words`
