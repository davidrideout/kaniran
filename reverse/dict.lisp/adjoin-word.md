# adjoin-word (generic function)

**Package:** `ichiran/dict`  
**Source:** `dict.lisp`  
**Definition form:** `defgeneric`

## Inputs (generic lambda list)

`(ichiran/dict::word1 ichiran/dict::word2 &key s-sql:text ichiran/dict::kana
  ichiran/dict::score-mod ichiran/dict::score-base &allow-other-keys)`

## Outputs

Docstring: make compound word from 2 words

## Dependencies at generic dispatch site

_(none detected)_


## Methods

### method (ICHIRAN/DICT::COMPOUND-TEXT ICHIRAN/DICT::SIMPLE-TEXT)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::word1 ichiran/dict::word2 &key s-sql:text
              ichiran/dict::kana ichiran/dict::score-mod &allow-other-keys)`

**Dependencies:**

_(none detected)_

### method (ICHIRAN/DICT::SIMPLE-TEXT ICHIRAN/DICT::SIMPLE-TEXT)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::word1 ichiran/dict::word2 &key s-sql:text
              ichiran/dict::kana ichiran/dict::score-mod
              ichiran/dict::score-base)`

**Dependencies:**

_(none detected)_

### method (T T) :AROUND

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::word1 ichiran/dict::word2 &key s-sql:text
              ichiran/dict::kana ichiran/dict::score-mod
              ichiran/dict::score-base)`

**Dependencies:**

- `ichiran/dict:get-kana`
- `ichiran/dict:get-text`


## Source-walked references

- `ichiran/dict:compound-text`
- `ichiran/dict:get-kana`
- `ichiran/dict:get-text`
- `ichiran/dict:score-base`
- `ichiran/dict:score-mod`
- `ichiran/dict:simple-text`
- `ichiran/dict:words`
