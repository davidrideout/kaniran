mod get_suffix_description {
    use crate::dict::grammar::suffix::init::*;

    fn ctx() -> std::sync::Arc<KaniranContext> {
        crate::test_support::shared_ctx()
    }

    /// Looks up a description by sequence across all four paths: the
    /// sequence's class has a description, the class has none, the
    /// sequence is a direct description key, and a miss on both tables.
    #[test]
    fn get_suffix_description_paths() {
        let ctx = ctx();
        let cases: &[(i32, Option<&str>)] = &[
            // Sequence belongs to a suffix class that has a description.
            (2013800, Some("indicates completion (to finish ...)")), // chau
            (2017560, Some("want to... / would like to...")),        // tai
            (2028920, Some("topic marker particle")),                // ha
            (1006610, Some("looking like ... / seeming ...")),       // sou
            // Sequence belongs to a suffix class that has no description.
            (2141080, None), // sou+
            // Sequence has no class but is itself a description key.
            (2826528, Some("polite prefix")),
            (2028980, Some("at / in / by")),
            (1002980, Some("from / because")),
            // Sequence appears in neither table.
            (1005530, None),
            (99999999, None),
        ];
        for (seq, expected) in cases {
            assert_eq!(get_suffix_description(&ctx, *seq), *expected, "seq={seq}");
        }
    }
}
