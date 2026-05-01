# r-apply (generic function)

**Package:** `ichiran`  
**Source:** `romanize.lisp`  
**Definition form:** `defgeneric`

## Inputs (generic lambda list)

`(ichiran::modifier method ichiran::cc-tree)`

## Outputs

Docstring: Apply modifier to something

## Dependencies at generic dispatch site

_(none detected)_


## Methods

### method ((EQL :+YO) ICHIRAN:GENERIC-HEPBURN T)

**Source:** `romanize.lisp`  
**Inputs:** `(ichiran::modifier method ichiran::cc-tree)`

**Dependencies:**

_(none detected)_

### method ((EQL :+YU) ICHIRAN:GENERIC-HEPBURN T)

**Source:** `romanize.lisp`  
**Inputs:** `(ichiran::modifier method ichiran::cc-tree)`

**Dependencies:**

_(none detected)_

### method ((EQL :+YA) ICHIRAN:GENERIC-HEPBURN T)

**Source:** `romanize.lisp`  
**Inputs:** `(ichiran::modifier method ichiran::cc-tree)`

**Dependencies:**

_(none detected)_

### method ((EQL :SOKUON) ICHIRAN:GENERIC-HEPBURN T)

**Source:** `romanize.lisp`  
**Inputs:** `(ichiran::modifier method ichiran::cc-tree)`

**Dependencies:**

- `ichiran:leftmost-atom`
- `ichiran:romanize-core`

### method (SYMBOL ICHIRAN:GENERIC-ROMANIZATION T)

**Source:** `romanize.lisp`  
**Inputs:** `(ichiran::modifier method ichiran::cc-tree)`

**Dependencies:**

- `ichiran:kana-table`
- `ichiran:romanize-core`

### method ((EQL :SOKUON) T T)

**Source:** `romanize.lisp`  
**Inputs:** `(ichiran::modifier method ichiran::cc-tree)`

**Dependencies:**

- `ichiran:romanize-core`

### method ((EQL :LONG-VOWEL) T T)

**Source:** `romanize.lisp`  
**Inputs:** `(ichiran::modifier method ichiran::cc-tree)`

**Dependencies:**

- `ichiran:romanize-core`

### method (SYMBOL T T)

**Source:** `romanize.lisp`  
**Inputs:** `(ichiran::modifier method ichiran::cc-tree)`

**Dependencies:**

- `ichiran:romanize-core`

