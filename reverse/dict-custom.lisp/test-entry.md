# test-entry (generic function)

**Package:** `ichiran/custom`  
**Source:** `dict-custom.lisp`  
**Definition form:** `defgeneric`

## Inputs (generic lambda list)

`(ichiran/custom::source ichiran/custom::entry)`

## Outputs

Docstring: Tests if the entry should be inserted into database  
  
Returns 2 values, whether the entry should be either added or updated, and which SEQ to update if any.  


## Dependencies at generic dispatch site

_(none detected)_


## Methods

### method (T ICHIRAN/CUSTOM::WARD)

**Source:** `dict-custom.lisp`  
**Inputs:** `(ichiran/custom::loader ichiran/custom::entry)`

**Dependencies:**

- `ichiran/custom:get-words`
- `ichiran/custom:ward-reading`
- `ichiran/custom:ward-text`
- `ichiran/dict:match-glosses`

### method (T ICHIRAN/CUSTOM::MUNICIPALITY)

**Source:** `dict-custom.lisp`  
**Inputs:** `(ichiran/custom::loader ichiran/custom::entry)`

**Dependencies:**

- `ichiran/custom:get-words`
- `ichiran/custom:municipality-prefecture`
- `ichiran/custom:municipality-reading`
- `ichiran/custom:municipality-text`
- `ichiran/custom:municipality-type`
- `ichiran/dict:match-glosses`

### method (T T)

**Source:** `dict-custom.lisp`  
**Inputs:** `(ichiran/custom::source ichiran/custom::entry)`

**Dependencies:**

_(none detected)_

