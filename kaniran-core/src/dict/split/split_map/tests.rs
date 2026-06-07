mod _star_split_map_star_ {
    use crate::dict::split::split_map::*;

    #[test]
    fn registered_count_matches_upstream_split_map() {
        // dict-split.lisp registers 174 entries via def-simple-split /
        // def-de-split / def-toori-split / def-do-split /
        // def-shi-split outside the *segsplit-map* let-binding.
        assert_eq!(REGISTERED_COUNT, 174);
    }
}
