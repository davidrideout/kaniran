//! Port of `ichiran/characters:geminate` (`characters.lisp:311-314`).
//!
//! Replace the last character of `txt` with the small tsu `っ` and
//! return the result. Empty input passes through unchanged.
//!
//! Diverges from the Lisp by always allocating a new `String` and
//! dropping the `:fresh` keyword (CONVENTIONS §4.6).

pub fn geminate(txt: &str) -> String {
    let mut chars: Vec<char> = txt.chars().collect();
    if let Some(last) = chars.last_mut() {
        *last = 'っ';
    }
    chars.into_iter().collect()
}
