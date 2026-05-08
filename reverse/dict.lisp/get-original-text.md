# get-original-text (generic function)

**Package:** `ichiran/dict`  
**Source:** `dict.lisp`  
**Definition form:** `defgeneric`

## Inputs (generic lambda list)

`(ichiran/dict::reading &key ichiran/dict::conj-data)`

## Outputs

Docstring: Returns unconjugated text(s) for reading

## Dependencies at generic dispatch site

_(none detected)_


## Methods

### method (ICHIRAN/DICT::PROXY-TEXT)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::reading &key ichiran/dict::conj-data)`

**Dependencies:**

- `ichiran/dict:get-original-text`
- `ichiran/dict:source`

### method (ICHIRAN/DICT::SIMPLE-TEXT)

**Source:** `dict.lisp`  
**Inputs:** `(ichiran/dict::reading &key ichiran/dict::conj-data)`

**Dependencies:**

- `ichiran/dict:get-original-text*`
- `ichiran/dict:text`
- `ichiran/dict:word-conj-data`
- `ichiran/dict:word-type`


## Source-walked references

- `ichiran/dict:conj-data`
- `ichiran/dict:get-original-text*`
- `ichiran/dict:kana-text`
- `ichiran/dict:kanji-text`
- `ichiran/dict:proxy-text`
- `ichiran/dict:reading`
- `ichiran/dict:seq`
- `ichiran/dict:simple-text`
- `ichiran/dict:source`
- `ichiran/dict:text`
- `ichiran/dict:word-conj-data`
- `ichiran/dict:word-type`
