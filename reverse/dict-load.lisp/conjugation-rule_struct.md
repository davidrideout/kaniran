# conjugation-rule (defstruct)

**Package:** `ichiran/dict`  
**Source:** `dict-load.lisp:262`  
**Metaclass:** `structure-class`

**Conc-name:** `CR-`  
**Default constructor:** `NIL`  
**All constructors:** `((ichiran/dict::make-conjugation-rule 0
                                                             (ichiran/dict::pos
                                                              ichiran/dict::conj
                                                              ichiran/dict::neg
                                                              ichiran/dict::fml
                                                              ichiran/dict::onum
                                                              ichiran/dict::stem
                                                              ichiran/dict::okuri
                                                              ichiran/dict::euphr
                                                              ichiran/dict::euphk)))`  
**Predicate:** `CONJUGATION-RULE-P`  
**Copier:** `COPY-CONJUGATION-RULE`  
**Include:** `NIL`  

## Slots

| name | default | type | accessor |
|---|---|---|---|
| POS | `nil` | `t` | `cr-pos` |
| CONJ | `nil` | `t` | `cr-conj` |
| NEG | `nil` | `t` | `cr-neg` |
| FML | `nil` | `t` | `cr-fml` |
| ONUM | `nil` | `t` | `cr-onum` |
| STEM | `nil` | `t` | `cr-stem` |
| OKURI | `nil` | `t` | `cr-okuri` |
| EUPHR | `nil` | `t` | `cr-euphr` |
| EUPHK | `nil` | `t` | `cr-euphk` |

