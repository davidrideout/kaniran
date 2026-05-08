# *char-scanners* (global variable)

**Package:** `ichiran/characters`  
**Source:** `characters.lisp:151`  
**Type of value:** `cons`

## Value

```lisp
((:KATAKANA . #<FUNCTION #1=(LAMBDA (STRING CL-PPCRE::START CL-PPCRE::END) :IN CL-PPCRE::CREATE-SCANNER-AUX) {10081A04AB}>) (:KATAKANA-UNIQ . #<FUNCTION #1# {10081A052B}>) (:HIRAGANA . #<FUNCTION #1# {10081A05AB}>) (:KANJI . #<FUNCTION #1# {10081A062B}>) (:KANJI-CHAR . #<FUNCTION #1# {10081A06AB}>) (:KANA . #<FUNCTION #1# {10081A072B}>) (:TRADITIONAL . #<FUNCTION #1# {10081A07AB}>) (:NONWORD . #<FUNCTION #1# {10081A082B}>) (:NUMBER . #<FUNCTION #1# {10081A08AB}>))
```

_(value is not round-trippable via `read` — likely contains closures, classes, or other unreadable shapes)_

## Source-walked references

- `ichiran/characters:*char-class-regex-mapping*`
