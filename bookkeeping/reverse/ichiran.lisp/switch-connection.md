# switch-connection

**Package:** `ichiran/maintenance`  
**Source:** `ichiran.lisp:82`  
**Definition form:** `defun`

## Inputs

`(ichiran/maintenance::conn &key ichiran/maintenance::reset)`

## Outputs

Declared ftype: `(function (t &key (:reset t)) (values t &optional))`

## Dependencies (ichiran symbols)

- `ichiran/conn:get-spec`
- `ichiran/conn:init-all-caches`
- `ichiran/conn:switch-conn-vars`
- `ichiran/dict:init-suffixes`

## Source-walked references

- `ichiran/conn:*connection*`
- `ichiran/conn:init-all-caches`
- `ichiran/conn:switch-conn-vars`
- `ichiran/conn:with-db`
- `ichiran/dict:init-suffixes`
