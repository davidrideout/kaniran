//! Port of `ichiran/dict:simplify-reading-list` (`dict.lisp:1704`).
//!
//! Collapse a list of readings into one display string per distinct
//! de-spaced reading, marking the word boundaries the spaces encoded.
//! For each reading the spaces are stripped and the boundary positions
//! recorded; readings sharing the same de-spaced text are merged. A
//! boundary every merged reading agrees on becomes a space; a boundary
//! only some readings carried becomes a `·` (`#\MIDDLE_DOT`, U+00B7).

/// Mirrors `(remove-duplicates positions)`. Order is unobservable here
/// (callers either sort the result or use it only for `count`), so this
/// keeps first occurrences.
fn remove_duplicates(positions: &[usize]) -> Vec<usize> {
    let mut out: Vec<usize> = Vec::new();
    for &position in positions {
        if !out.contains(&position) {
            out.push(position);
        }
    }
    out
}

pub fn simplify_reading_list(reading_list: &[String]) -> Vec<String> {
    // assoc entries are `(can cnt . spos)`: de-spaced text, count of
    // readings that mapped to it, and the accumulated boundary positions
    // (each reading's positions concatenated, cross-reading duplicates
    // kept so `count` below can measure agreement). Kept in encounter
    // order (Lisp builds the reverse and `nreverse`s; see the push arm).
    let mut assoc: Vec<(String, i32, Vec<usize>)> = Vec::new();
    for reading in reading_list {
        let mut can = String::new();
        let mut spos: Vec<usize> = Vec::new();
        let mut off: usize = 0;
        for (i, char) in reading.chars().enumerate() {
            if char == ' ' {
                spos.push(i - off);
                off += 1;
            } else {
                can.push(char);
            }
        }
        let spos = remove_duplicates(&spos);
        match assoc.iter_mut().find(|(entry_can, _, _)| *entry_can == can) {
            // dict.lisp:1713 — (setf (cddr a) (nconc spos (cddr a))) (incf (cadr a)).
            Some(entry) => {
                entry.2.extend(spos);
                entry.1 += 1;
            }
            // dict.lisp:1714 — (push (list* can 1 spos) assoc). Lisp pushes
            // onto the front and `nreverse`s at the end; appending here builds
            // the same encounter order directly.
            None => assoc.push((can, 1, spos)),
        }
    }
    let mut out: Vec<String> = Vec::with_capacity(assoc.len());
    for (can, cnt, spos) in &assoc {
        let mut sspos = remove_duplicates(spos);
        sspos.sort_unstable();
        let mut sspos = sspos.into_iter().peekable();
        let mut s = String::new();
        for (i, char) in can.chars().enumerate() {
            if sspos.peek() == Some(&i) {
                let count = spos.iter().filter(|&&position| position == i).count() as i32;
                s.push(if count == *cnt { ' ' } else { '\u{00B7}' });
                sspos.next();
            }
            s.push(char);
        }
        out.push(s);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn srl(readings: &[&str]) -> Vec<String> {
        let owned: Vec<String> = readings.iter().map(|reading| reading.to_string()).collect();
        simplify_reading_list(&owned)
    }

    #[test]
    fn simplify_reading_list_fixtures() {
        // REPL fixtures (.103, ichiran/dict::simplify-reading-list), 2026-05-23.
        let cases: &[(&[&str], &[&str])] = &[
            (&[], &[]),
            (&["aru"], &["aru"]),
            (&["tokoro ga"], &["tokoro ga"]),
            // two boundaries, single reading -> both agree -> spaces.
            (&["a b c"], &["a b c"]),
            // consecutive spaces collapse to one boundary (per-reading dedup).
            (&["a  b"], &["a b"]),
            // same de-spaced text, boundary disagrees -> MIDDLE_DOT.
            (&["tokoroga", "tokoro ga"], &["tokoro\u{00B7}ga"]),
            // same de-spaced text, boundary agrees -> space.
            (&["tokoro ga", "tokoro ga"], &["tokoro ga"]),
            // distinct de-spaced texts -> two outputs.
            (&["hito", "kuni"], &["hito", "kuni"]),
            // 2 of 3 readings split at the boundary (count<cnt) -> MIDDLE_DOT.
            (&["a b", "ab", "a b"], &["a\u{00B7}b"]),
            // leading space -> boundary at position 0.
            (&[" ab"], &[" ab"]),
            // trailing space -> boundary at length, never emitted.
            (&["ab "], &["ab"]),
            (&["a b c", "a b c", "a b c"], &["a b c"]),
            // same de-spaced "abc", two different boundaries, neither shared.
            (&["a bc", "ab c"], &["a\u{00B7}b\u{00B7}c"]),
        ];
        for (readings, expected) in cases {
            let actual = srl(readings);
            let expected: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
            assert_eq!(actual, expected, "readings={readings:?}");
        }
    }
}
