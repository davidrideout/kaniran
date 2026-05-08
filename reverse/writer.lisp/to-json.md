# to-json (generic function)

**Package:** `ichiran/dict`  
**Source:** `writer.lisp`  
**Definition form:** `defgeneric`

## Inputs (generic lambda list)

`(jsown::object)`

## Outputs

Docstring: Writes the given object to json in a generic way.

## Dependencies at generic dispatch site

_(none detected)_


## Methods

### method (ICHIRAN/DICT:WORD-INFO)

**Source:** `cli.lisp`  
**Inputs:** `(ichiran/dict:word-info)`

**Dependencies:**

- `ichiran/dict:to-json`
- `ichiran/dict:word-info-gloss-json`

### method ((EQL NIL))

**Source:** `writer.lisp`  
**Inputs:** `(jsown::n)`

**Dependencies:**

_(none detected)_

### method ((EQL :EMPTY-LIST))

**Source:** `writer.lisp`  
**Inputs:** `(jsown::empty-list)`

**Dependencies:**

_(none detected)_

### method ((EQL :N))

**Source:** `writer.lisp`  
**Inputs:** `(jsown::false)`

**Dependencies:**

_(none detected)_

### method ((EQL :NULL))

**Source:** `writer.lisp`  
**Inputs:** `(jsown::false)`

**Dependencies:**

_(none detected)_

### method ((EQL :F))

**Source:** `writer.lisp`  
**Inputs:** `(jsown::false)`

**Dependencies:**

_(none detected)_

### method ((EQL :FALSE))

**Source:** `writer.lisp`  
**Inputs:** `(jsown::false)`

**Dependencies:**

_(none detected)_

### method ((EQL :TRUE))

**Source:** `writer.lisp`  
**Inputs:** `(jsown::true)`

**Dependencies:**

_(none detected)_

### method ((EQL :T))

**Source:** `writer.lisp`  
**Inputs:** `(jsown::true)`

**Dependencies:**

_(none detected)_

### method ((EQL T))

**Source:** `writer.lisp`  
**Inputs:** `(jsown::true)`

**Dependencies:**

_(none detected)_

### method (SYMBOL)

**Source:** `writer.lisp`  
**Inputs:** `(jsown::s)`

**Dependencies:**

- `ichiran/dict:to-json`

### method (HASH-TABLE)

**Source:** `writer.lisp`  
**Inputs:** `(jsown::table)`

**Dependencies:**

- `ichiran/dict:to-json`

### method (ARRAY)

**Source:** `writer.lisp`  
**Inputs:** `(array)`

**Dependencies:**

- `ichiran/dict:to-json`

### method (LIST)

**Source:** `writer.lisp`  
**Inputs:** `(list)`

**Dependencies:**

_(none detected)_

### method (FLOAT)

**Source:** `writer.lisp`  
**Inputs:** `(float)`

**Dependencies:**

_(none detected)_

### method (RATIO)

**Source:** `writer.lisp`  
**Inputs:** `(ratio)`

**Dependencies:**

- `ichiran/dict:to-json`

### method (NUMBER)

**Source:** `writer.lisp`  
**Inputs:** `(number)`

**Dependencies:**

_(none detected)_

### method (STRING)

**Source:** `writer.lisp`  
**Inputs:** `(string)`

**Dependencies:**

_(none detected)_

### method (JSOWN:JSON-ENCODED-CONTENT)

**Source:** `writer.lisp`  
**Inputs:** `(jsown::content)`

**Dependencies:**

_(none detected)_


## Source-walked references

_(none detected)_
