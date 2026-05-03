# value-string (generic function)

**Package:** `ichiran/dict`  
**Source:** `dict-counters.lisp`  
**Definition form:** `defgeneric`

## Inputs (generic lambda list)

`(ichiran/dict::counter)`

## Outputs

Docstring: Value to be presented as string

## Dependencies at generic dispatch site

_(none detected)_


## Methods

### method (ICHIRAN/DICT::COUNTER-WARI)

**Source:** `dict-counters.lisp`  
**Inputs:** `(ichiran/dict::counter)`

**Dependencies:**

- `ichiran/dict:number-value`

### method (ICHIRAN/DICT::COUNTER-MONTHS)

**Source:** `dict-counters.lisp`  
**Inputs:** `(ichiran/dict::counter)`

**Dependencies:**

- `ichiran/dict:number-value`

### method (ICHIRAN/DICT::COUNTER-HALFHOUR)

**Source:** `dict-counters.lisp`  
**Inputs:** `(ichiran/dict::counter)`

**Dependencies:**

- `ichiran/dict:number-value`

### method (ICHIRAN/DICT::COUNTER-TEXT)

**Source:** `dict-counters.lisp`  
**Inputs:** `(ichiran/dict::counter &aux
              (ichiran/dict::value
               (ichiran/dict::number-value ichiran/dict::counter)))`

**Dependencies:**

- `ichiran/dict:counter-suffix-descriptions`
- `ichiran/dict:number-value`
- `ichiran/dict:ordinal-str`
- `ichiran/dict:ordinalp`


## Source-walked references

- `ichiran/dict:counter-halfhour`
- `ichiran/dict:counter-months`
- `ichiran/dict:counter-suffix-descriptions`
- `ichiran/dict:counter-text`
- `ichiran/dict:counter-wari`
- `ichiran/dict:number-value`
- `ichiran/dict:ordinal-str`
- `ichiran/dict:ordinalp`
