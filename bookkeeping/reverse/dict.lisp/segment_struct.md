# segment (defstruct)

**Package:** `ichiran/dict`  
**Source:** `dict.lisp:674`  
**Metaclass:** `structure-class`

**Conc-name:** `SEGMENT-`  
**Default constructor:** `MAKE-SEGMENT`  
**All constructors:** `((ichiran/dict::make-segment . :default))`  
**Predicate:** `SEGMENT-P`  
**Copier:** `COPY-SEGMENT`  
**Include:** `NIL`  

## Slots

| name | default | type | accessor |
|---|---|---|---|
| START | `nil` | `t` | `segment-start` |
| END | `nil` | `t` | `segment-end` |
| WORD | `nil` | `t` | `segment-word` |
| SCORE | `nil` | `t` | `segment-score` |
| INFO | `nil` | `t` | `segment-info` |
| TOP | `nil` | `t` | `segment-top` |
| TEXT | `nil` | `t` | `segment-text` |


## Source-walked references

- `ichiran/dict:text`
