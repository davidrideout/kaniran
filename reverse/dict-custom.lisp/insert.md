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


## Source-walked references

- `ichiran/conn:*connection*`
- `ichiran/custom:cur-seq`
- `ichiran/custom:entries`
- `ichiran/custom:entry`
- `ichiran/custom:for`
- `ichiran/custom:in`
- `ichiran/custom:insert-entry`
- `ichiran/custom:loader`
- `ichiran/custom:ok`
- `ichiran/custom:seq`
- `ichiran/custom:source`
- `ichiran/custom:test-entry`
- `ichiran/custom:update-entry`
- `ichiran/custom:update-entry-gloss`
- `ichiran/custom:with`
- `ichiran/custom:xml-entry-content`
- `ichiran/custom:xml-entry-seq`
- `ichiran/custom:xml-loader`
- `ichiran/dict:load-entry`
- `ichiran/dict:next-seq`
