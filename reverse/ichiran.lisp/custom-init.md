# custom-init

**Package:** `ichiran/maintenance`  
**Source:** `ichiran.lisp:40`  
**Definition form:** `defun`

## Inputs

`(ichiran/maintenance::dict-connection &key ichiran/maintenance::jmdict-path
  ichiran/maintenance::jmdict-data ichiran/maintenance::kanjidic-path)`

## Outputs

Declared ftype: `(function
                  (t &key (:jmdict-path t) (:jmdict-data t) (:kanjidic-path t))
                  *)`

## Dependencies (ichiran symbols)

- `ichiran/conn:get-spec`
- `ichiran/maintenance:full-init`
