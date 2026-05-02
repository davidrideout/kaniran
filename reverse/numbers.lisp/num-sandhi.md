# num-sandhi (generic function)

**Package:** `ichiran/numbers`  
**Source:** `numbers.lisp`  
**Definition form:** `defgeneric`

## Inputs (generic lambda list)

`(ichiran/numbers::c1 ichiran/numbers::v1 ichiran/numbers::c2
  ichiran/numbers::v2 ichiran/numbers::s1 ichiran/numbers::s2)`

## Outputs

Docstring: join s1 and s2 taking digit classes into account

## Dependencies at generic dispatch site

_(none detected)_


## Methods

### method (T T T T T T)

**Source:** `numbers.lisp`  
**Inputs:** `(ichiran/numbers::c1 ichiran/numbers::v1 ichiran/numbers::c2
              ichiran/numbers::v2 ichiran/numbers::s1 ichiran/numbers::s2)`

**Dependencies:**

_(none detected)_

### method ((EQL :JD) (EQL 1) (EQL :P) T T T)

**Source:** `numbers.lisp`  
**Inputs:** `(ichiran/numbers::c1 ichiran/numbers::v1 ichiran/numbers::c2
              ichiran/numbers::v2 ichiran/numbers::s1 ichiran/numbers::s2)`

**Dependencies:**

- `ichiran/characters:geminate`

### method ((EQL :JD) (EQL 3) (EQL :P) T T T)

**Source:** `numbers.lisp`  
**Inputs:** `(ichiran/numbers::c1 ichiran/numbers::v1 ichiran/numbers::c2
              ichiran/numbers::v2 ichiran/numbers::s1 ichiran/numbers::s2)`

**Dependencies:**

- `ichiran/characters:rendaku`

### method ((EQL :JD) (EQL 6) (EQL :P) T T T)

**Source:** `numbers.lisp`  
**Inputs:** `(ichiran/numbers::c1 ichiran/numbers::v1 ichiran/numbers::c2
              ichiran/numbers::v2 ichiran/numbers::s1 ichiran/numbers::s2)`

**Dependencies:**

- `ichiran/characters:geminate`
- `ichiran/characters:rendaku`

### method ((EQL :JD) (EQL 8) (EQL :P) T T T)

**Source:** `numbers.lisp`  
**Inputs:** `(ichiran/numbers::c1 ichiran/numbers::v1 ichiran/numbers::c2
              ichiran/numbers::v2 ichiran/numbers::s1 ichiran/numbers::s2)`

**Dependencies:**

- `ichiran/characters:geminate`
- `ichiran/characters:rendaku`

### method ((EQL :P) (EQL 1) (EQL :P) T T T)

**Source:** `numbers.lisp`  
**Inputs:** `(ichiran/numbers::c1 ichiran/numbers::v1 ichiran/numbers::c2
              ichiran/numbers::v2 ichiran/numbers::s1 ichiran/numbers::s2)`

**Dependencies:**

- `ichiran/characters:geminate`

### method ((EQL :P) (EQL 2) (EQL :P) T T T)

**Source:** `numbers.lisp`  
**Inputs:** `(ichiran/numbers::c1 ichiran/numbers::v1 ichiran/numbers::c2
              ichiran/numbers::v2 ichiran/numbers::s1 ichiran/numbers::s2)`

**Dependencies:**

- `ichiran/characters:geminate`


## Source-walked references

- `ichiran/characters:geminate`
- `ichiran/characters:rendaku`
- `ichiran/numbers:c1`
- `ichiran/numbers:c2`
- `ichiran/numbers:s1`
- `ichiran/numbers:s2`
- `ichiran/numbers:v1`
- `ichiran/numbers:v2`
