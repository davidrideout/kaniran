# count-char-class

**Package:** `ichiran/characters`  
**Source:** `characters.lisp:194`  
**Definition form:** `defun`

## Inputs

`(ichiran/characters::word ichiran/characters::char-class)`

## Outputs

Declared ftype: `(function
                  (t
                   (member :number :nonword :traditional :kana :kanji-char
                           :kanji :hiragana :katakana-uniq :katakana))
                  (values unsigned-byte &optional))`

## Dependencies (ichiran symbols)

_(none detected)_

## Source-walked references

- `ichiran/characters:*char-class-regex-mapping*`
- `ichiran/characters:char-class`
- `ichiran/characters:cnt`
- `ichiran/characters:e`
- `ichiran/characters:regex`
- `ichiran/characters:s`
- `ichiran/characters:word`
