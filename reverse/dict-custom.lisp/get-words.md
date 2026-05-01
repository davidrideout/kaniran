# get-words (generic function)

**Package:** `ichiran/custom`  
**Source:** `dict-custom.lisp`  
**Definition form:** `defgeneric`

## Inputs (generic lambda list)

`(ichiran/custom::entry)`

## Outputs

_unknown — no docstring_

## Dependencies at generic dispatch site

_(none detected)_


## Methods

### method (ICHIRAN/CUSTOM::WARD)

**Source:** `dict-custom.lisp`  
**Inputs:** `(ichiran/custom::entry)`

**Dependencies:**

- `ichiran/custom:ward-city`
- `ichiran/custom:ward-definition`

### method (ICHIRAN/CUSTOM::MUNICIPALITY)

**Source:** `dict-custom.lisp`  
**Inputs:** `(ichiran/custom::entry)`

**Dependencies:**

- `ichiran/custom:municipality-definition`
- `ichiran/custom:municipality-prefecture`
- `ichiran/custom:municipality-type`

