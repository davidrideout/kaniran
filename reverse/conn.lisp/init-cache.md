# init-cache (generic function)

**Package:** `ichiran/conn`  
**Source:** `conn.lisp`  
**Definition form:** `defgeneric`

## Inputs (generic lambda list)

`(ichiran/conn::cache-name)`

## Outputs

Docstring: Should return a value to initialize cache with

## Dependencies at generic dispatch site

_(none detected)_


## Methods

### method ((EQL :COUNTERS))

**Source:** `dict-counters.lisp`  
**Inputs:** `(#:cache-var0)`

**Dependencies:**

- `ichiran/characters:test-word`
- `ichiran/dict:get-counter-readings`
- `ichiran/dict:seq`

### method ((EQL :IS-ARCH))

**Source:** `dict.lisp`  
**Inputs:** `(#:cache-var0)`

**Dependencies:**

_(none detected)_

### method ((EQL :NO-CONJ-DATA))

**Source:** `dict.lisp`  
**Inputs:** `(#:cache-var0)`

**Dependencies:**

_(none detected)_


## Source-walked references

- `ichiran/conn:cache-name`
