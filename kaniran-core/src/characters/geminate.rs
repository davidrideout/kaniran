//! Port of `ichiran/characters:geminate` (`characters.lisp:311-314`).
//!
//! Replace the last character of `txt` with the small tsu `っ`. Empty
//! input is left unchanged.
//!
//! The upstream signature is `(txt &key fresh)`. With `:fresh nil`
//! (default) it mutates `txt` in place; with `:fresh t` it copies first
//! and mutates the copy. The Rust port takes `&mut String` and always
//! mutates in place — equivalent to `:fresh nil`. Callers that need
//! `:fresh t` semantics clone before calling:
//!
//! ```ignore
//! let mut copy = original.to_string();
//! geminate(&mut copy);  // `original` is untouched
//! ```

pub fn geminate(txt: &mut String) {
    if txt.pop().is_some() {
        txt.push('っ');
    }
}
