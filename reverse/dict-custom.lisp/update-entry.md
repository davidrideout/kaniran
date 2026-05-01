# update-entry (generic function)

**Package:** `ichiran/custom`  
**Source:** `dict-custom.lisp`  
**Definition form:** `defgeneric`

## Inputs (generic lambda list)

`(ichiran/custom::source ichiran/custom::entry ichiran/custom::seq)`

## Outputs

Docstring: Update entry in database

## Dependencies at generic dispatch site

_(none detected)_


## Methods

### method (ICHIRAN/CUSTOM::WARD-CSV T T)

**Source:** `dict-custom.lisp`  
**Inputs:** `(ichiran/custom::loader ichiran/custom::entry ichiran/custom::seq)`

**Dependencies:**

- `ichiran/custom:ward-definition`
- `ichiran/custom:ward-reading`
- `ichiran/custom:ward-text`
- `ichiran/dict:add-new-sense`

### method (ICHIRAN/CUSTOM::MUNICIPALITY-CSV T T)

**Source:** `dict-custom.lisp`  
**Inputs:** `(ichiran/custom::loader ichiran/custom::entry ichiran/custom::seq)`

**Dependencies:**

- `ichiran/custom:municipality-definition`
- `ichiran/custom:municipality-reading`
- `ichiran/custom:municipality-text`
- `ichiran/dict:add-new-sense`

