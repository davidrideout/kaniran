# char-class (deftype)

**Package:** `ichiran/characters`
**Source:** `characters.lisp:147`
**Definition form:** `deftype`

## Definition

```lisp
(deftype char-class () '(member :katakana :katakana-uniq
                         :hiragana :kanji :kanji-char
                         :kana :traditional :nonword :number))
```

A type alias enumerating the legal character-class tags used throughout
`ichiran/characters` (and referenced by argument type declarations on
functions like `test-word` and `count-char-class`).

## Members

| tag | meaning |
|---|---|
| `:katakana` | any katakana character |
| `:katakana-uniq` | katakana character that is not also reachable via hiragana mapping |
| `:hiragana` | any hiragana character |
| `:kanji` | any kanji character |
| `:kanji-char` | a single kanji character (used for per-char classification) |
| `:kana` | hiragana or katakana |
| `:traditional` | traditional / radical character |
| `:nonword` | punctuation, whitespace, or other non-word characters |
| `:number` | digit (Western or kanji numeric) |

## Used by (callers that declare arguments of this type)

- `ichiran/characters:test-word` — `(word char-class)` — declared via `(declare (type char-class char-class))`
- Type referenced by the `*char-class-regex-mapping*` and `*char-scanners*` globals
- Selectors in `count-char-class`, `consecutive-char-groups`, `collect-char-class`

## Port note

`deftype` produces a type alias only — no runtime behavior to port. In
Rust this becomes an `enum CharClass` with the nine variants above. Any
function whose Lisp signature declares `(type char-class arg)` ports to
a Rust function taking `CharClass` instead of an open-ended keyword.
