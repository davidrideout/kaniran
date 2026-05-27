# destem

**Package:** `ichiran/characters`  
**Source:** `characters.lisp:340`  
**Definition form:** `defun`

## Inputs

`(ichiran/characters::word ichiran/characters::stem &optional
  (ichiran/characters::char-class :kana))`

## Outputs

Declared ftype: `(function
                  (t t &optional
                   (member :number :nonword :traditional :kana :kanji-char
                           :kanji :hiragana :katakana-uniq :katakana))
                  (values t &optional))`

## Dependencies (ichiran symbols)

_(none detected)_

## Source-walked references

- `ichiran/characters:*char-class-regex-mapping*`
