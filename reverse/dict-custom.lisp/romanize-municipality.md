# romanize-municipality

**Package:** `ichiran/custom`  
**Source:** `dict-custom.lisp:133`  
**Definition form:** `defun`

## Inputs

`(s-sql:text ichiran/custom::reading &key (ichiran/custom::include-type t))`

## Outputs

Declared ftype: `(function (t t &key (:include-type t))
                  (values simple-string &optional))`

## Dependencies (ichiran symbols)

- `ichiran/custom:municipality-short`
- `ichiran:romanize-word-geo`

## Source-walked references

- `ichiran/custom:*municipality-types-description*`
- `ichiran/custom:municipality-short`
- `ichiran/dict:text`
- `ichiran:romanize-word-geo`
