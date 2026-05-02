# entry (dao-class)

**Package:** `ichiran/dict`  
**Source:** `dict.lisp:26`  
**Metaclass:** `dao-class`

**Table:** `ENTRY`  
**Primary key:** `(SEQ)`

## Inheritance

- Direct supers: `standard-object`
- Precedence list: `entry`, `standard-object`, `slot-object`, `t`

## Columns

| name | column | type | initargs | readers |
|---|---|---|---|---|
| SEQ | seq | `integer` | (SEQ) | (SEQ) |
| CONTENT | content | `string` | (CONTENT) | (CONTENT) |
| ROOT-P | root_p | `boolean` | (ROOT-P) | (ROOT-P) |
| N-KANJI | n_kanji | `integer` | (N-KANJI) | (N-KANJI) |
| N-KANA | n_kana | `integer` | (N-KANA) | (N-KANA) |
| PRIMARY-NOKANJI | primary_nokanji | `boolean` | NIL | (PRIMARY-NOKANJI) |

