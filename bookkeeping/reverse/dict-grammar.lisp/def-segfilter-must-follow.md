# def-segfilter-must-follow

**Package:** `ichiran/dict`  
**Source:** `dict-grammar.lisp:1049`  
**Definition form:** `defmacro`

## Inputs

`(ichiran/dict::name
  (ichiran/dict::segment-list-left ichiran/dict::segment-list-right)
  ichiran/dict::filter-left ichiran/dict::filter-right &key
  ichiran/dict::allow-first)`

## Outputs

Docstring: This segfilter is for when segments that satisfy filter-right MUST follow segments that  
   satisfy filter-left

## Dependencies (ichiran symbols)

_(none detected)_

## Source-walked references

- `ichiran/dict:classify`
- `ichiran/dict:defsegfilter`
- `ichiran/dict:make-segment-list-from`
- `ichiran/dict:segment-list-end`
- `ichiran/dict:segment-list-segments`
- `ichiran/dict:segment-list-start`
