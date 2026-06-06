//! Port of `ichiran/characters:geminate` (`characters.lisp:311-314`).
//!
//! Replace the last character of `txt` with the small tsu `っ`. Empty
//! input is left unchanged.
pub fn geminate(txt: &mut String) {
    if txt.pop().is_some() {
        txt.push('っ');
    }
}
