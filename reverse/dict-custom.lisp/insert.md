# insert (generic function)

**Package:** `ichiran/custom`  
**Source:** `dict-custom.lisp`  
**Definition form:** `defgeneric`

## Inputs (generic lambda list)

`(ichiran/custom::source)`

## Outputs

Docstring: Insert slurped data into database

## Dependencies at generic dispatch site

_(none detected)_


## Methods

### method (ICHIRAN/CUSTOM::XML-LOADER)

**Source:** `dict-custom.lisp`  
**Inputs:** `(ichiran/custom::loader)`

**Dependencies:**

- `ichiran/custom:entries`
- `ichiran/custom:xml-entry-content`
- `ichiran/custom:xml-entry-seq`
- `ichiran/dict:load-entry`

### method (T)

**Source:** `dict-custom.lisp`  
**Inputs:** `(ichiran/custom::source)`

**Dependencies:**

- `ichiran/custom:entries`
- `ichiran/custom:insert-entry`
- `ichiran/custom:test-entry`
- `ichiran/custom:update-entry`
- `ichiran/custom:update-entry-gloss`
- `ichiran/dict:next-seq`

