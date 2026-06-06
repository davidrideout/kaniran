//! Port of `ichiran/kanji:calculate-perc` (`kanji.lisp:349`).
//!
//! Renders `sample / total` as a percentage string with two fractional
//! digits and a trailing `%`, or the literal `"--.--%"` when `total`
//! is zero.

pub fn calculate_perc(sample: i32, total: i32) -> String {
    if total == 0 {
        "--.--%".to_string()
    } else {
        format!("{:.2}%", 100.0 * sample as f64 / total as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_repl_captures() {
        // /tmp/probe-kanji.lisp on .103 — verified 2026-05-09.
        assert_eq!(calculate_perc(50, 100), "50.00%");
        assert_eq!(calculate_perc(1, 1000), "0.10%");
        assert_eq!(calculate_perc(0, 0), "--.--%");
        assert_eq!(calculate_perc(33, 100), "33.00%");
        assert_eq!(calculate_perc(1, 3), "33.33%");
        assert_eq!(calculate_perc(1, 7), "14.29%");
        assert_eq!(calculate_perc(5, 100), "5.00%");
        assert_eq!(calculate_perc(100, 100), "100.00%");
        assert_eq!(calculate_perc(3, 7), "42.86%");
    }
}
