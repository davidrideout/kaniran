//! Port of `ichiran/dict:root-diff` (`dict-errata.lisp:95`).
//!
//! Counts how many leading characters of `base_text` and `reading`
//! sit outside their shared right-aligned tail.

pub fn root_diff(base_text: &str, reading: &str) -> (usize, usize) {
    let base_chars: Vec<char> = base_text.chars().collect();
    let reading_chars: Vec<char> = reading.chars().collect();
    let lb = base_chars.len();
    let lr = reading_chars.len();
    let mut ib = lb;
    let mut ir = lr;
    while ib > 0 && ir > 0 {
        ib -= 1;
        ir -= 1;
        if base_chars[ib] != reading_chars[ir] {
            return (ib + 1, ir + 1);
        }
    }
    if lr >= lb {
        (0, lr - lb)
    } else {
        (lb - lr, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_diff_fixtures() {
        // REPL (.103, `ichiran/dict::root-diff`), 2026-05-31.
        let cases: &[(&str, &str, (usize, usize))] = &[
            ("食べる", "食べる", (0, 0)),
            ("尋ねる", "たずねる", (1, 2)),
            ("ね", "たずね", (0, 2)),
            ("ABCDE", "XYZ", (5, 3)),
            ("abc", "", (3, 0)),
            ("", "abc", (0, 3)),
            ("食べた", "たべた", (1, 1)),
            ("思う", "おもう", (1, 2)),
            ("学校", "がっこう", (2, 4)),
            ("本", "ほん", (1, 2)),
            ("言葉", "ことば", (2, 3)),
        ];
        for (base, reading, expected) in cases {
            assert_eq!(
                root_diff(base, reading),
                *expected,
                "base={base:?} reading={reading:?}",
            );
        }
    }
}
