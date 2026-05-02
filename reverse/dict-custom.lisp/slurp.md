# slurp (generic function)

**Package:** `ichiran/custom`  
**Source:** `dict-custom.lisp`  
**Definition form:** `defgeneric`

## Inputs (generic lambda list)

`(ichiran/custom::source)`

## Outputs

Docstring: Read custom data from the source file

## Dependencies at generic dispatch site

_(none detected)_


## Methods

### method (ICHIRAN/CUSTOM::WARD-CSV)

**Source:** `dict-custom.lisp`  
**Inputs:** `(ichiran/custom::loader)`

**Dependencies:**

- `ichiran/custom:csv-options`
- `ichiran/custom:make-ward`
- `ichiran/custom:romanize-municipality`
- `ichiran/custom:source-file`

### method (ICHIRAN/CUSTOM::MUNICIPALITY-CSV) :AFTER

**Source:** `dict-custom.lisp`  
**Inputs:** `(ichiran/custom::loader)`

**Dependencies:**

- `ichiran/custom:entries`
- `ichiran/custom:municipality-type`

### method (ICHIRAN/CUSTOM::CSV-LOADER)

**Source:** `dict-custom.lisp`  
**Inputs:** `(ichiran/custom::loader)`

**Dependencies:**

- `ichiran/custom:csv-options`
- `ichiran/custom:process-entry`
- `ichiran/custom:source-file`

### method (ICHIRAN/CUSTOM::XML-LOADER)

**Source:** `dict-custom.lisp`  
**Inputs:** `(ichiran/custom::loader)`

**Dependencies:**

- `ichiran/custom:entries`
- `ichiran/custom:make-xml-entry`
- `ichiran/custom:source-file`
- `ichiran/dict:node-text`

### method (T) :AROUND

**Source:** `dict-custom.lisp`  
**Inputs:** `(ichiran/custom::source)`

**Dependencies:**

- `ichiran/custom:entries`


## Source-walked references

- `ichiran/custom:*municipality-types-order*`
- `ichiran/custom:city-reading`
- `ichiran/custom:city-romanized`
- `ichiran/custom:city-text`
- `ichiran/custom:collect`
- `ichiran/custom:content`
- `ichiran/custom:csv-loader`
- `ichiran/custom:csv-options`
- `ichiran/custom:definition`
- `ichiran/custom:e`
- `ichiran/custom:else`
- `ichiran/custom:entries`
- `ichiran/custom:entry`
- `ichiran/custom:for`
- `ichiran/custom:id`
- `ichiran/custom:in`
- `ichiran/custom:loader`
- `ichiran/custom:make-ward`
- `ichiran/custom:make-xml-entry`
- `ichiran/custom:municipality-csv`
- `ichiran/custom:municipality-type`
- `ichiran/custom:nseq`
- `ichiran/custom:parsed`
- `ichiran/custom:process-entry`
- `ichiran/custom:reading`
- `ichiran/custom:romanize-municipality`
- `ichiran/custom:row`
- `ichiran/custom:seq`
- `ichiran/custom:source`
- `ichiran/custom:source-file`
- `ichiran/custom:ward-csv`
- `ichiran/custom:ward-reading`
- `ichiran/custom:ward-text`
- `ichiran/custom:with`
- `ichiran/custom:xml-loader`
- `ichiran/dict:node-text`
