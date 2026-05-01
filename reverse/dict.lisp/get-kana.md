# get-kana (generic function)

**Package:** `ichiran/dict`  
**Source:** `dict.lisp`  
**Definition form:** `defgeneric`

## Inputs (generic lambda list)

`(ichiran/dict::obj)`

## Outputs

Docstring: most popular kana representation

## Dependencies at generic dispatch site

_(none detected)_


## Methods

### method (ICHIRAN/DICT::COUNTER-AGE)

**Source:** `dict-counters.lisp`  
**Inputs:** `(ichiran/dict::obj)`

**Dependencies:**

- `ichiran/dict:number-value`

### method (ICHIRAN/DICT::COUNTER-PEOPLE)

**Source:** `dict-counters.lisp`  
**Inputs:** `(ichiran/dict::obj)`

**Dependencies:**

- `ichiran/dict:number-value`

### method (ICHIRAN/DICT::COUNTER-DAYS-KUN)

**Source:** `dict-counters.lisp`  
**Inputs:** `(ichiran/dict::obj)`

**Dependencies:**

- `ichiran/dict:number-value`

### method (ICHIRAN/DICT::COUNTER-HIFUMI)

**Source:** `dict-counters.lisp`  
**Inputs:** `(ichiran/dict::obj &aux
              (ichiran/dict::value
               (ichiran/dict::number-value ichiran/dict::obj)))`

**Dependencies:**

- `ichiran/dict:counter-kana`
- `ichiran/dict:digit-set`
- `ichiran/dict:number-value`

### method (ICHIRAN/DICT::COUNTER-TSU)

**Source:** `dict-counters.lisp`  
**Inputs:** `(ichiran/dict::obj)`

**Dependencies:**

- `ichiran/dict:number-value`

### method (ICHIRAN/DICT::NUMBER-TEXT)

**Source:** `dict-counters.lisp`  
**Inputs:** `(ichiran/dict::obj)`

**Dependencies:**

- `ichiran/dict:number-value`
- `ichiran/numbers:number-to-kana`

### method (ICHIRAN/DICT::COUNTER-TEXT) :AROUND

**Source:** `dict-counters.lisp`  
**Inputs:** `(ichiran/dict::obj)`

**Dependencies:**

- `ichiran/dict:counter-suffix`

### method (ICHIRAN/DICT::COUNTER-TEXT)

**Source:** `dict-counters.lisp`  
**Inputs:** `(ichiran/dict::obj)`

**Dependencies:**

- `ichiran/dict:counter-join`
- `ichiran/dict:counter-kana`
- `ichiran/dict:number-value`
- `ichiran/numbers:number-to-kana`

### method (ICHIRAN/DICT::COMPOUND-TEXT)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::compound-text)`

**Dependencies:**

_(none detected)_

### method (ICHIRAN/DICT::PROXY-TEXT)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::proxy-text)`

**Dependencies:**

_(none detected)_

### method (ICHIRAN/DICT::KANA-TEXT)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::obj)`

**Dependencies:**

_(none detected)_

### method (ICHIRAN/DICT::KANJI-TEXT)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::obj)`

**Dependencies:**

- `ichiran/dict:best-kana-conj`
- `ichiran/dict:get-kanji-kana-old`

### method (ICHIRAN/DICT::SIMPLE-TEXT) :AROUND

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::obj)`

**Dependencies:**

- `ichiran/dict:get-hint`
- `ichiran/dict:hintedp`

### method (ICHIRAN/DICT::ENTRY)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::obj)`

**Dependencies:**

- `ichiran/dict:seq`

