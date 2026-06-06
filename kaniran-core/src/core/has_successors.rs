//! Port of `ichiran:has-successors` (`deromanize.lisp:13-19`).
//!
//! Collects every proper prefix (char-wise, length 1 up to but not
//! including the full length) of each input string into a membership set.

use std::collections::HashSet;

pub fn has_successors(strings: &[&str]) -> HashSet<String> {
    let mut hash = HashSet::new();
    for s in strings {
        // (loop for end from 1 below (length s) for ss = (subseq s 0 end) ...)
        // subseq indexes by character, so prefixes are taken char-wise.
        let chars: Vec<char> = s.chars().collect();
        for end in 1..chars.len() {
            let ss: String = chars[..end].iter().collect();
            hash.insert(ss);
        }
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(items: &[&str]) -> HashSet<String> {
        items.iter().map(|item| item.to_string()).collect()
    }

    #[test]
    fn has_successors_fixtures() {
        // REPL fixtures (.103, ichiran::has-successors), 2026-05-25.
        // (input, expected proper-prefix set).
        let cases: &[(&[&str], &[&str])] = &[
            // "cha"->{c,ch}, "chi"->{c,ch}, "ba"->{b}
            (&["cha", "chi", "ba"], &["c", "ch", "b"]),
            // single-char and empty strings yield no proper prefixes
            (&["a", "x", ""], &[]),
            // "kya"->{k,ky}, "kyo"->{k,ky}, "sha"->{s,sh}
            (&["kya", "kyo", "sha"], &["k", "ky", "s", "sh"]),
        ];
        for (input, expected) in cases {
            assert_eq!(has_successors(input), set(expected), "input={input:?}");
        }
    }
}
