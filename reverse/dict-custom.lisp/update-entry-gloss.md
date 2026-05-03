# update-entry-gloss (generic function)

**Package:** `ichiran/custom`  
**Source:** `dict-custom.lisp`  
**Definition form:** `defgeneric`

## Inputs (generic lambda list)

`(ichiran/custom::source ichiran/custom::entry ichiran/custom::seq
  ichiran/custom::gloss)`

## Outputs

Docstring: Update entry gloss in database

## Dependencies at generic dispatch site

_(none detected)_


## Methods

### method (ICHIRAN/CUSTOM::MUNICIPALITY-CSV T T T)

**Source:** `dict-custom.lisp`  
**Inputs:** `(ichiran/custom::loader ichiran/custom::entry ichiran/custom::seq
              ichiran/custom::gloss)`

**Dependencies:**

- `ichiran/custom:municipality-definition`
- `ichiran/custom:municipality-reading`
- `ichiran/custom:municipality-text`


## Source-walked references

- `ichiran/custom:*silent-p*`
- `ichiran/custom:municipality-csv`
- `ichiran/custom:municipality-definition`
- `ichiran/custom:municipality-reading`
- `ichiran/custom:municipality-text`
