# verify (generic function)

**Package:** `ichiran/dict`  
**Source:** `dict-counters.lisp`  
**Definition form:** `defgeneric`

## Inputs (generic lambda list)

`(ichiran/dict::counter ichiran/dict::unique)`

## Outputs

Docstring: Verify if counter is valid

## Dependencies at generic dispatch site

_(none detected)_


## Methods

### method (ICHIRAN/DICT::COUNTER-DAYS-ON T)

**Source:** `dict-counters.lisp`  
**Inputs:** `(ichiran/dict::counter ichiran/dict::unique)`

**Dependencies:**

- `ichiran/dict:number-value`

### method (ICHIRAN/DICT::COUNTER-TSU T)

**Source:** `dict-counters.lisp`  
**Inputs:** `(ichiran/dict::counter ichiran/dict::unique)`

**Dependencies:**

- `ichiran/dict:number-value`

### method (T T)

**Source:** `dict-counters.lisp`  
**Inputs:** `(ichiran/dict::counter ichiran/dict::unique)`

**Dependencies:**

- `ichiran/dict:counter-allowed`
- `ichiran/dict:number-value`


## Source-walked references

- `ichiran/dict:counter`
- `ichiran/dict:counter-allowed`
- `ichiran/dict:counter-days-on`
- `ichiran/dict:counter-tsu`
- `ichiran/dict:n`
- `ichiran/dict:number-value`
- `ichiran/dict:unique`
