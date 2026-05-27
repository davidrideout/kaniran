# as-xml (generic function)

**Package:** `ichiran/custom`  
**Source:** `dict-custom.lisp`  
**Definition form:** `defgeneric`

## Inputs (generic lambda list)

`(ichiran/custom::entry)`

## Outputs

Docstring: Representation of entry as XML to be loaded by load-entry

## Dependencies at generic dispatch site

_(none detected)_


## Methods

### method (ICHIRAN/CUSTOM::WARD)

**Source:** `dict-custom.lisp`  
**Inputs:** `(ichiran/custom::entry)`

**Dependencies:**

- `ichiran/custom:as-xml-simple`
- `ichiran/custom:ward-definition`
- `ichiran/custom:ward-reading`
- `ichiran/custom:ward-text`

### method (ICHIRAN/CUSTOM::MUNICIPALITY)

**Source:** `dict-custom.lisp`  
**Inputs:** `(ichiran/custom::entry)`

**Dependencies:**

- `ichiran/custom:as-xml-simple`
- `ichiran/custom:municipality-definition`
- `ichiran/custom:municipality-reading`
- `ichiran/custom:municipality-text`


## Source-walked references

- `ichiran/custom:as-xml-simple`
- `ichiran/custom:municipality`
- `ichiran/custom:municipality-definition`
- `ichiran/custom:municipality-reading`
- `ichiran/custom:municipality-text`
- `ichiran/custom:ward`
- `ichiran/custom:ward-definition`
- `ichiran/custom:ward-reading`
- `ichiran/custom:ward-text`
