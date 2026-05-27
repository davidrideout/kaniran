# get-segment-score (generic function)

**Package:** `ichiran/dict`  
**Source:** `dict.lisp`  
**Definition form:** `defgeneric`

## Inputs (generic lambda list)

`(ichiran/dict::seg)`

## Outputs

Docstring: Like segment-score but also works for segment-list and synergies

## Dependencies at generic dispatch site

_(none detected)_


## Methods

### method (ICHIRAN/DICT::SYNERGY)

**Source:** `dict-grammar.lisp`  
**Inputs:** `(ichiran/dict::syn)`

**Dependencies:**

- `ichiran/dict:synergy-score`

### method (ICHIRAN/DICT::SEGMENT)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::seg)`

**Dependencies:**

- `ichiran/dict:segment-score`

### method (ICHIRAN/DICT::SEGMENT-LIST)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::seg-list)`

**Dependencies:**

- `ichiran/dict:segment-list-segments`
- `ichiran/dict:segment-score`


## Source-walked references

- `ichiran/dict:segment`
- `ichiran/dict:segment-list`
- `ichiran/dict:segment-list-segments`
- `ichiran/dict:segment-score`
- `ichiran/dict:synergy`
- `ichiran/dict:synergy-score`
