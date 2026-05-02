# romanize*

**Package:** `ichiran`  
**Source:** `romanize.lisp:273`  
**Definition form:** `defun`

## Inputs

`(ichiran::input &key (method ichiran:*default-romanization-method*)
  (ichiran::limit 5) (ichiran::wordprop-fn (constantly nil)))`

## Outputs

Declared ftype: `(function (t &key (:method t) (:limit t) (:wordprop-fn t))
                  (values list &optional))`

## Dependencies (ichiran symbols)

- `ichiran/characters:basic-split`
- `ichiran/characters:normalize`
- `ichiran/dict:dict-segment`
- `ichiran:romanize-word-info`

## Source-walked references

- `ichiran/characters:basic-split`
- `ichiran/characters:normalize`
- `ichiran/dict:dict-segment`
- `ichiran:*default-romanization-method*`
- `ichiran:collect`
- `ichiran:for`
- `ichiran:in`
- `ichiran:input`
- `ichiran:limit`
- `ichiran:pair`
- `ichiran:prop`
- `ichiran:romanize-word-info`
- `ichiran:romanized`
- `ichiran:score`
- `ichiran:split-text`
- `ichiran:split-type`
- `ichiran:word`
- `ichiran:word-list`
- `ichiran:wordprop-fn`
