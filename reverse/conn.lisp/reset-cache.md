# reset-cache (generic function)

**Package:** `ichiran/conn`  
**Source:** `conn.lisp`  
**Definition form:** `defgeneric`

## Inputs (generic lambda list)

`(ichiran/conn::cache-name)`

## Outputs

_unknown — no docstring_

## Dependencies at generic dispatch site

_(none detected)_


## Methods

### method (T)

**Source:** `conn.lisp`  
**Inputs:** `(ichiran/conn::cache-name)`

**Dependencies:**

- `ichiran/conn:cache-lock`
- `ichiran/conn:cache-var`
- `ichiran/conn:get-cache`
- `ichiran/conn:init-cache`


## Source-walked references

- `ichiran/conn:cache`
- `ichiran/conn:cache-lock`
- `ichiran/conn:cache-name`
- `ichiran/conn:cache-var`
- `ichiran/conn:get-cache`
- `ichiran/conn:init-cache`
- `ichiran/conn:val`
