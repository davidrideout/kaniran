/// Port of `ichiran/characters:match-diff` (`characters.lisp:326-357`).
///
/// Recursively align two strings into a sequence of [`MatchSegment`]s
/// that alternate between equal regions and pairs of differing regions,
/// returning also a score equal to the total length of the matched
/// portions (number of *characters*, not bytes). Used by the kanji
/// reading-matcher for any non-kanji string pair.
///
/// Returns `None` when either input is empty.
///
/// When one input has length 1 (and the inputs differ), the result is a
/// single `Diff(s1, s2)` segment with score 0 — even when the first
/// characters happen to match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchSegment {
    Equal(String),
    Diff(String, String),
}

pub fn match_diff(s1: &str, s2: &str) -> Option<(Vec<MatchSegment>, usize)> {
    let c1: Vec<char> = s1.chars().collect();
    let c2: Vec<char> = s2.chars().collect();
    let l1 = c1.len();
    let l2 = c2.len();
    if l1 == 0 || l2 == 0 {
        return None;
    }
    let m = mismatch(&c1, &c2);
    match m {
        None => Some((vec![MatchSegment::Equal(s1.to_string())], l1)),
        Some(_) if l1 == 1 || l2 == 1 => {
            Some((vec![MatchSegment::Diff(s1.to_string(), s2.to_string())], 0))
        }
        Some(0) => {
            let mut best: Option<(Vec<MatchSegment>, usize)> = None;
            for i in 1..l1 {
                for j in 1..l2 {
                    if c1[i] != c2[j] {
                        continue;
                    }
                    let s1_rest = safe_subseq(s1, i, None).expect("i < l1");
                    let s2_rest = safe_subseq(s2, j, None).expect("j < l2");
                    let Some((rest_match, rest_value)) = match_diff(&s1_rest, &s2_rest) else {
                        continue;
                    };
                    let is_better = best.as_ref().map_or(true, |(_, v)| rest_value > *v);
                    if is_better {
                        let head = MatchSegment::Diff(
                            safe_subseq(s1, 0, Some(i)).expect("i <= l1"),
                            safe_subseq(s2, 0, Some(j)).expect("j <= l2"),
                        );
                        let mut combined = vec![head];
                        combined.extend(rest_match);
                        best = Some((combined, rest_value));
                    }
                }
            }
            best
        }
        Some(m) if m == l1 => {
            // s1 is a strict prefix of s2. The upstream peels the last
            // matched character into the Diff, so the Equal segment is
            // l1-1 long and the Diff carries one s1 character versus
            // the trailing s2 portion.
            Some((
                vec![
                    MatchSegment::Equal(safe_subseq(s1, 0, Some(l1 - 1)).expect("l1 >= 1")),
                    MatchSegment::Diff(
                        safe_subseq(s1, l1 - 1, None).expect("l1 >= 1"),
                        safe_subseq(s2, l1 - 1, None).expect("l1-1 < l2"),
                    ),
                ],
                l1 - 1,
            ))
        }
        Some(m) if m == l2 => Some((
            vec![
                MatchSegment::Equal(safe_subseq(s2, 0, Some(l2 - 1)).expect("l2 >= 1")),
                MatchSegment::Diff(
                    safe_subseq(s1, l2 - 1, None).expect("l2-1 < l1"),
                    safe_subseq(s2, l2 - 1, None).expect("l2 >= 1"),
                ),
            ],
            l2 - 1,
        )),
        Some(m) => {
            let s1_rest = safe_subseq(s1, m, None).expect("m < l1");
            let s2_rest = safe_subseq(s2, m, None).expect("m < l2");
            match_diff(&s1_rest, &s2_rest).map(|(rest_match, rest_value)| {
                let head = MatchSegment::Equal(safe_subseq(s1, 0, Some(m)).expect("m <= l1"));
                let mut combined = vec![head];
                combined.extend(rest_match);
                (combined, rest_value + m)
            })
        }
    }
}

fn mismatch(a: &[char], b: &[char]) -> Option<usize> {
    let shared = a.len().min(b.len());
    for i in 0..shared {
        if a[i] != b[i] {
            return Some(i);
        }
    }
    if a.len() == b.len() {
        None
    } else {
        Some(shared)
    }
}

/// Port of `ichiran/characters:join` (`characters.lisp:365-370`).
///
/// Concatenate `items` with `separator` between each pair.
pub fn join<S: AsRef<str>>(separator: &str, items: &[S]) -> String {
    let mut out = String::new();
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            out.push_str(separator);
        }
        out.push_str(item.as_ref());
    }
    out
}

/// Port of `ichiran/characters:safe-subseq` (`characters.lisp:359-363`).
///
/// Bounds-checked substring over *character* positions. Returns `None`
/// on out-of-range or inverted (`start > end`) bounds; `end = None`
/// slices to the end of the string.
pub fn safe_subseq(s: &str, start: usize, end: Option<usize>) -> Option<String> {
    let len = s.chars().count();
    if start > len {
        return None;
    }
    let stop = match end {
        Some(e) if e > len || start > e => return None,
        Some(e) => e,
        None => len,
    };
    Some(s.chars().skip(start).take(stop - start).collect())
}

#[cfg(test)]
mod tests;
