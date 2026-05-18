//! Port of `ichiran/dict:*noun-particles*` (`dict-grammar.lisp:801`).
//!
//! Order and duplicates are preserved from upstream: `1005120`
//! appears twice (さえ and すら) because the entries are
//! per-meaning-cluster, not per-seq.

pub static NOUN_PARTICLES: &[i32] = &[
    2028920, // は
    2028930, // が
    2028990, // に
    2028980, // で
    2029000, // へ
    1007340, // だけ
    1579080, // ごろ
    1525680, // まで
    2028940, // も
    1582300, // など
    2215430, // には
    1469800, // の
    1009990, // のみ
    2029010, // を
    1005120, // さえ
    2034520, // でさえ
    1005120, // すら
    1008490, // と
    1008530, // とか
    1008590, // として
    2028950, // とは
    2028960, // や
    1009600, // にとって
];
