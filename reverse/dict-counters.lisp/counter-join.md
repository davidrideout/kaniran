# counter-join (generic function)

**Package:** `ichiran/dict`  
**Source:** `dict-counters.lisp`  
**Definition form:** `defgeneric`

## Inputs (generic lambda list)

`(ichiran/dict::counter ichiran/dict::n ichiran/dict::number-kana
  ichiran/dict::counter-kana)`

## Outputs

Docstring: Construct counter kana text

## Dependencies at generic dispatch site

_(none detected)_


## Methods

### method (ICHIRAN/DICT::COUNTER-TEXT T T T)

**Source:** `dict-counters.lisp`  
**Inputs:** `(ichiran/dict::obj ichiran/dict::n ichiran/dict::number-kana
              ichiran/dict::counter-kana &aux
              (ichiran/dict::digit (ichiran/dict::get-digit ichiran/dict::n))
              (ichiran/dict::head
               (gethash (char ichiran/dict::counter-kana 0)
                        ichiran/characters:*char-class-hash*))
              (ichiran/dict::digit-opts
               (assoc ichiran/dict::digit
                      (ichiran/dict::digit-opts ichiran/dict::obj)))
              (ichiran/dict::off
               (assoc :off (ichiran/dict::digit-opts ichiran/dict::obj))))`

**Dependencies:**

- `ichiran/characters:geminate`
- `ichiran/characters:rendaku`
- `ichiran/dict:counter-foreign`
- `ichiran/dict:digit-opts`
- `ichiran/dict:get-digit`

### method (T T T T)

**Source:** `dict-counters.lisp`  
**Inputs:** `(ichiran/dict::counter ichiran/dict::n ichiran/dict::number-kana
              ichiran/dict::counter-kana)`

**Dependencies:**

_(none detected)_

