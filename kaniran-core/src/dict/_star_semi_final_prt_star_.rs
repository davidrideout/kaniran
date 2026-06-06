//! Port of `ichiran/dict:*semi-final-prt*` (`dict-errata.lisp:1196`).
//!
//! Particles that are final but also have other uses; the final-prt
//! list plus さ/し/な/ね/わ.

use std::sync::OnceLock;

use super::_star_final_prt_star_::FINAL_PRT;

static CACHE: OnceLock<Vec<i32>> = OnceLock::new();

pub fn semi_final_prt() -> &'static [i32] {
    CACHE
        .get_or_init(|| {
            let mut out: Vec<i32> = FINAL_PRT.to_vec();
            out.extend_from_slice(&[
                2029120, // さ
                2086640, // し
                2029110, // な
                2029080, // ね
                2029100, // わ
            ]);
            out
        })
        .as_slice()
}
