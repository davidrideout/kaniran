# process-entry (generic function)

**Package:** `ichiran/custom`  
**Source:** `dict-custom.lisp`  
**Definition form:** `defgeneric`

## Inputs (generic lambda list)

`(ichiran/custom::source ichiran/custom::entry)`

## Outputs

Docstring: Converts a source chunk into one or several entries

## Dependencies at generic dispatch site

_(none detected)_


## Methods

### method (ICHIRAN/CUSTOM::MUNICIPALITY-CSV T)

**Source:** `dict-custom.lisp`  
**Inputs:** `(ichiran/custom::loader ichiran/custom::row)`

**Dependencies:**

- `ichiran/characters:as-hiragana`
- `ichiran/characters:normalize`
- `ichiran/custom:make-municipality`
- `ichiran/custom:municipality-short`
- `ichiran/custom:romanize-municipality`

### method (T T)

**Source:** `dict-custom.lisp`  
**Inputs:** `(ichiran/custom::source ichiran/custom::entry)`

**Dependencies:**

_(none detected)_


## Source-walked references

- `ichiran/characters:as-hiragana`
- `ichiran/characters:normalize`
- `ichiran/custom:definition`
- `ichiran/custom:entry`
- `ichiran/custom:id`
- `ichiran/custom:loader`
- `ichiran/custom:make-municipality`
- `ichiran/custom:muni`
- `ichiran/custom:muni-short`
- `ichiran/custom:municipality-csv`
- `ichiran/custom:municipality-short`
- `ichiran/custom:pref`
- `ichiran/custom:prefecture`
- `ichiran/custom:prefecture-p`
- `ichiran/custom:reading`
- `ichiran/custom:rmuni`
- `ichiran/custom:romanize-municipality`
- `ichiran/custom:row`
- `ichiran/custom:rpref`
- `ichiran/custom:short`
- `ichiran/custom:source`
