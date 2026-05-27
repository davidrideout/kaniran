# diff-content

**Package:** `ichiran/maintenance`  
**Source:** `ichiran.lisp:138`  
**Definition form:** `defun`

## Inputs

`(ichiran/maintenance::old ichiran/maintenance::new &key
  (ichiran/maintenance::short t))`

## Outputs

Declared ftype: `(function (t t &key (:short t))
                  (values (or simple-string (member :gone :new)) &optional))`

## Dependencies (ichiran symbols)

- `ichiran/characters:split-by-regex`

## Source-walked references

- `ichiran/characters:split-by-regex`
