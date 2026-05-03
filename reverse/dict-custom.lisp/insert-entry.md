# insert-entry (generic function)

**Package:** `ichiran/custom`  
**Source:** `dict-custom.lisp`  
**Definition form:** `defgeneric`

## Inputs (generic lambda list)

`(ichiran/custom::source ichiran/custom::entry ichiran/custom::seq)`

## Outputs

Docstring: Insert entry into database

## Dependencies at generic dispatch site

_(none detected)_


## Methods

### method (ICHIRAN/CUSTOM::WARD-CSV T T)

**Source:** `dict-custom.lisp`  
**Inputs:** `(ichiran/custom::loader ichiran/custom::entry ichiran/custom::seq)`

**Dependencies:**

- `ichiran/custom:as-xml`
- `ichiran/custom:ward-definition`
- `ichiran/custom:ward-reading`
- `ichiran/custom:ward-text`
- `ichiran/dict:load-entry`

### method (ICHIRAN/CUSTOM::MUNICIPALITY-CSV T T)

**Source:** `dict-custom.lisp`  
**Inputs:** `(ichiran/custom::loader ichiran/custom::entry ichiran/custom::seq)`

**Dependencies:**

- `ichiran/custom:as-xml`
- `ichiran/custom:municipality-definition`
- `ichiran/custom:municipality-reading`
- `ichiran/custom:municipality-text`
- `ichiran/dict:load-entry`


## Source-walked references

- `ichiran/custom:*silent-p*`
- `ichiran/custom:as-xml`
- `ichiran/custom:municipality-csv`
- `ichiran/custom:municipality-definition`
- `ichiran/custom:municipality-reading`
- `ichiran/custom:municipality-text`
- `ichiran/custom:ward-csv`
- `ichiran/custom:ward-definition`
- `ichiran/custom:ward-reading`
- `ichiran/custom:ward-text`
- `ichiran/dict:load-entry`
