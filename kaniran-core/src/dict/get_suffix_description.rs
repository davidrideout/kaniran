//! Port of `ichiran/dict:get-suffix-description` (`dict-grammar.lisp:160`).
//!
//! If `seq` is registered in `*suffix-class*`, key
//! `*suffix-description*` by that class; otherwise by `seq` itself
//! (`(or (gethash seq *suffix-class*) seq)`).

use super::_star_suffix_class_star_::suffix_class;
use super::_star_suffix_description_star_::{suffix_description, SuffixDescKey};
use crate::conn::kani_context::KaniranContext;

pub fn get_suffix_description(ctx: &KaniranContext, seq: i32) -> Option<&'static str> {
    let key = match suffix_class(ctx).get(&seq) {
        Some(class) => SuffixDescKey::Class(class.clone()),
        None => SuffixDescKey::Seq(seq),
    };
    suffix_description().get(&key).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
    }

    /// REPL (.103, after `(init-suffixes t)`):
    /// `(get-suffix-description seq)` across all four lookup paths —
    /// seq→class→desc, seq→class→no-desc, seq→direct-key→desc, and
    /// miss on both.
    #[tokio::test]
    async fn get_suffix_description_paths() {
        let ctx = ctx().await;
        let cases: &[(i32, Option<&str>)] = &[
            // seq in *suffix-class*, class has a description
            (2013800, Some("indicates completion (to finish ...)")), // :chau
            (2017560, Some("want to... / would like to...")),        // :tai
            (2028920, Some("topic marker particle")),                // :ha
            (1006610, Some("looking like ... / seeming ...")),       // :sou
            // seq in *suffix-class*, class has no description
            (2141080, None), // :sou+
            // seq not in *suffix-class*, seq is a direct *suffix-description* key
            (2826528, Some("polite prefix")),
            (2028980, Some("at / in / by")),
            (1002980, Some("from / because")),
            // in neither table
            (1005530, None),
            (99999999, None),
        ];
        for (seq, expected) in cases {
            assert_eq!(get_suffix_description(&ctx, *seq), *expected, "seq={seq}");
        }
    }
}
