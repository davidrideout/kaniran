# load-jmdict

**Package:** `ichiran/dict`  
**Source:** `dict-load.lisp:168`  
**Definition form:** `defun`

## Inputs

`(&key (ichiran/dict::path ichiran/dict::*jmdict-path*)
  (ichiran/dict::load-extras t))`

## Outputs

Declared ftype: `(function (&key (:path t) (:load-extras t)) *)`

## Dependencies (ichiran symbols)

- `ichiran/dict:fix-entities`
- `ichiran/dict:init-tables`
- `ichiran/dict:load-entry`
- `ichiran/dict:load-extras`
- `ichiran/dict:recalc-entry-stats-all`

## Source-walked references

- `ichiran/conn:*connection*`
- `ichiran/dict:*jmdict-path*`
- `ichiran/dict:content`
- `ichiran/dict:fix-entities`
- `ichiran/dict:init-tables`
- `ichiran/dict:load-entry`
- `ichiran/dict:load-extras`
- `ichiran/dict:recalc-entry-stats-all`
- `ichiran/dict:source`
