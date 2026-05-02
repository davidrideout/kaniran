# collect-entries

**Package:** `ichiran/maintenance`  
**Source:** `ichiran.lisp:107`  
**Definition form:** `defun`

## Inputs

`(ichiran/maintenance::seq-set &key
  (ichiran/maintenance::conn ichiran/conn:*connection*))`

## Outputs

Declared ftype: `(function (t &key (:conn t))
                  (values list hash-table &optional))`

## Dependencies (ichiran symbols)

- `ichiran/conn:get-spec`
- `ichiran/dict:seq`

## Source-walked references

- `ichiran/conn:*connection*`
- `ichiran/conn:with-db`
- `ichiran/dict:entry`
- `ichiran/dict:seq`
- `ichiran/maintenance:collect`
- `ichiran/maintenance:conn`
- `ichiran/maintenance:entry`
- `ichiran/maintenance:for`
- `ichiran/maintenance:hash`
- `ichiran/maintenance:in`
- `ichiran/maintenance:seq`
- `ichiran/maintenance:seq-set`
