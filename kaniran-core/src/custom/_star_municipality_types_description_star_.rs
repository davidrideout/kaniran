//! Port of `ichiran/custom:*municipality-types-description*` (`dict-custom.lisp:107`).
//!
//! 道's upstream entry is the bare cons `(#\道)` with `nil` cdr,
//! ported as `None`.

pub static MUNICIPALITY_TYPES_DESCRIPTION: &[(char, Option<&str>)] = &[
    ('都', Some("Metropolis")),
    ('道', None),
    ('府', Some("Prefecture")),
    ('県', Some("Prefecture")),
    ('市', Some("(city)")),
    ('町', Some("(town)")),
    ('村', Some("(village)")),
    ('区', Some("Ward")),
];
