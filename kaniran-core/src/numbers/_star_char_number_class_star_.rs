//! Port of `ichiran/numbers:*char-number-class*` (`numbers.lisp:9`).
//!
//! Source-of-truth table mapping each numeric character to its
//! [`NumClass`] tag and value. The first column groups characters that
//! share the same classification (e.g. `"〇零"` are both
//! `(NumClass::Jd, 0)`, `"十拾"` are both `(NumClass::P, 1)`).

use super::kani_num_class::NumClass;

pub const CHAR_NUMBER_CLASS: &[(&str, NumClass, u8)] = &[
    ("〇零", NumClass::Jd, 0),
    ("一壱", NumClass::Jd, 1),
    ("二弐", NumClass::Jd, 2),
    ("三参", NumClass::Jd, 3),
    ("四", NumClass::Jd, 4),
    ("五", NumClass::Jd, 5),
    ("六", NumClass::Jd, 6),
    ("七", NumClass::Jd, 7),
    ("八", NumClass::Jd, 8),
    ("九", NumClass::Jd, 9),
    ("十拾", NumClass::P, 1),
    ("百", NumClass::P, 2),
    ("千", NumClass::P, 3),
    ("万", NumClass::P, 4),
    ("億", NumClass::P, 8),
    ("兆", NumClass::P, 12),
    ("京", NumClass::P, 16),
    ("0０", NumClass::Ad, 0),
    ("1１", NumClass::Ad, 1),
    ("2２", NumClass::Ad, 2),
    ("3３", NumClass::Ad, 3),
    ("4４", NumClass::Ad, 4),
    ("5５", NumClass::Ad, 5),
    ("6６", NumClass::Ad, 6),
    ("7７", NumClass::Ad, 7),
    ("8８", NumClass::Ad, 8),
    ("9９", NumClass::Ad, 9),
];
