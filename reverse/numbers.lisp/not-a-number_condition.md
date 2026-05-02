# not-a-number (define-condition)

**Package:** `ichiran`
**Source:** `numbers.lisp:67`
**Definition form:** `define-condition`
**Inherits:** `error`

## Definition

```lisp
(define-condition not-a-number (error)
  ((text :reader text :initarg :text)
   (reason :reader reason :initarg :reason))
  (:report (lambda (c s)
             (format s "~S is not a number: ~a"
                     (text c) (reason c)))))
```

Custom error condition raised when number parsing fails on a character
that is not a recognized digit.

## Slots

| name | reader | initarg | meaning |
|---|---|---|---|
| `text` | `text` | `:text` | the original input string being parsed |
| `reason` | `reason` | `:reason` | a human-readable explanation of why parsing failed |

## Reporter

Formats as: `"<text>" is not a number: <reason>`

Example: `"abc" is not a number: Invalid character: a`

## Raised by

- `ichiran:parse-number` (`numbers.lisp:74`) — raises `not-a-number` with
  `:text` set to the original input and `:reason` describing the offending
  character. The only call site in the codebase.

## Port note

In Rust this maps to either:

- A struct with `Display` and `std::error::Error` impls:

  ```rust
  #[derive(Debug)]
  pub struct NotANumber {
      pub text: String,
      pub reason: String,
  }
  impl std::fmt::Display for NotANumber {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
          write!(f, "{:?} is not a number: {}", self.text, self.reason)
      }
  }
  impl std::error::Error for NotANumber {}
  ```

- Or as a variant in a `thiserror` error enum if the `numbers::` module
  ends up with multiple error kinds.

The single-call-site usage means whichever the surrounding error story
lands on (Result return on `parse_number` is the obvious shape) is fine.
