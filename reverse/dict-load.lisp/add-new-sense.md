# add-new-sense

**Package:** `ichiran/dict`  
**Source:** `dict-load.lisp:91`  
**Definition form:** `defun`

## Inputs

`(ichiran/dict::seq ichiran/dict::positions ichiran/dict::glosses &aux
                    (ichiran/dict::senses
                     (ichiran/dict::get-senses-raw ichiran/dict::seq)))`

## Outputs

Declared ftype: `(function (t t t) (values t &optional number))`

## Dependencies (ichiran symbols)

- `ichiran/dict:get-senses-raw`
- `ichiran/dict:id`
- `ichiran/dict:sense-exists-p`

## Source-walked references

- `ichiran/dict:for`
- `ichiran/dict:from`
- `ichiran/dict:get-senses-raw`
- `ichiran/dict:gloss`
- `ichiran/dict:glosses`
- `ichiran/dict:gord`
- `ichiran/dict:id`
- `ichiran/dict:in`
- `ichiran/dict:last-pos`
- `ichiran/dict:last-sense`
- `ichiran/dict:ord`
- `ichiran/dict:pos`
- `ichiran/dict:positions`
- `ichiran/dict:props`
- `ichiran/dict:s`
- `ichiran/dict:sense`
- `ichiran/dict:sense-exists-p`
- `ichiran/dict:sense-id`
- `ichiran/dict:sense-prop`
- `ichiran/dict:senses`
- `ichiran/dict:seq`
- `ichiran/dict:sord`
- `ichiran/dict:thereis`
