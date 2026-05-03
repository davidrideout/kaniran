# get-seq-changes

**Package:** `ichiran/maintenance`  
**Source:** `ichiran.lisp:121`  
**Definition form:** `defun`

## Inputs

`(ichiran/maintenance::old-conn ichiran/maintenance::new-conn &key
  ichiran/maintenance::regex ichiran/maintenance::seqs)`

## Outputs

Declared ftype: `(function (t t &key (:regex t) (:seqs t))
                  (values hash-table &optional))`

## Dependencies (ichiran symbols)

- `ichiran/conn:get-spec`
- `ichiran/maintenance:get-hardcoded-constants`

## Source-walked references

- `ichiran/maintenance:compare-queries`
- `ichiran/maintenance:get-hardcoded-constants`
