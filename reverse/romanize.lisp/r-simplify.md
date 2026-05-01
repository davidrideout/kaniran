# r-simplify (generic function)

**Package:** `ichiran`  
**Source:** `romanize.lisp`  
**Definition form:** `defgeneric`

## Inputs (generic lambda list)

`(method ichiran::str)`

## Outputs

Docstring: Simplify the result of transliteration

## Dependencies at generic dispatch site

_(none detected)_


## Methods

### method (ICHIRAN:KUNREI-SIKI T)

**Source:** `romanize.lisp`  
**Inputs:** `(method ichiran::str)`

**Dependencies:**

- `ichiran/characters:simplify-ngrams`

### method (ICHIRAN::TRADITIONAL-HEPBURN T)

**Source:** `romanize.lisp`  
**Inputs:** `(method ichiran::str)`

**Dependencies:**

_(none detected)_

### method (ICHIRAN:SIMPLIFIED-HEPBURN T)

**Source:** `romanize.lisp`  
**Inputs:** `(method ichiran::str)`

**Dependencies:**

- `ichiran/characters:simplify-ngrams`
- `ichiran:simplifications`

### method (ICHIRAN:GENERIC-HEPBURN T)

**Source:** `romanize.lisp`  
**Inputs:** `(method ichiran::str)`

**Dependencies:**

_(none detected)_

### method (T T)

**Source:** `romanize.lisp`  
**Inputs:** `(method ichiran::str)`

**Dependencies:**

_(none detected)_

