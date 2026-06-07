mod _star_suffix_description_star_ {
    use crate::dict::grammar::suffix::constants::*;

    /// The description table holds 47 entries; spot-checks a few class
    /// keys, a few seq keys, and two misses.
    #[test]
    fn matches_introspected_value() {
        let map = suffix_description();
        assert_eq!(map.len(), 47);

        // class keywords
        assert_eq!(
            map.get(&SuffixDescKey::Class("chau".to_string())).copied(),
            Some("indicates completion (to finish ...)"),
        );
        assert_eq!(
            map.get(&SuffixDescKey::Class("ha".to_string())).copied(),
            Some("topic marker particle"),
        );
        // trailing space is intentional and must be preserved
        assert_eq!(
            map.get(&SuffixDescKey::Class("kuru".to_string())).copied(),
            Some("indicates action that had been continuing up till now / came to be "),
        );
        assert_eq!(
            map.get(&SuffixDescKey::Class("tasou".to_string())).copied(),
            Some("seem to want to... (tai+sou)"),
        );

        // seq keys
        assert_eq!(
            map.get(&SuffixDescKey::Seq(2826528)).copied(),
            Some("polite prefix")
        );
        assert_eq!(
            map.get(&SuffixDescKey::Seq(2028980)).copied(),
            Some("at / in / by")
        );
        assert_eq!(
            map.get(&SuffixDescKey::Seq(1002980)).copied(),
            Some("from / because")
        );

        // miss
        assert_eq!(
            map.get(&SuffixDescKey::Class("nonexistent".to_string()))
                .copied(),
            None
        );
        assert_eq!(map.get(&SuffixDescKey::Seq(0)).copied(), None);
    }

    #[test]
    fn class_seq_partition() {
        let map = suffix_description();
        let class_count = map
            .keys()
            .filter(|k| matches!(k, SuffixDescKey::Class(_)))
            .count();
        let seq_count = map
            .keys()
            .filter(|k| matches!(k, SuffixDescKey::Seq(_)))
            .count();
        assert_eq!(class_count, 39);
        assert_eq!(seq_count, 8);
    }
}

mod _star_suffix_list_star_ {
    use crate::dict::grammar::suffix::constants::*;

    /// Every published keyword resolves through `lookup_suffix_fn`.
    #[test]
    fn registered_keys_resolve() {
        // Plain suffix keywords.
        for key in [
            "suru", "ra", "tai", "ren", "ren-", "neg", "te", "teiru", "teiru+", "te+space",
            "kudasai", "teren", "teii", "rou", "adv", "iadj", "tosuru", "kurai", "chau", "to",
            "sa", "sou", "sou+", "sugiru", "garu", "desu", "desho", "rashii",
        ] {
            assert!(lookup_suffix_fn(key).is_some(), "missing key: {}", key);
        }
        // Abbreviated-form suffix keywords.
        for key in [
            "nai",
            "nai-x",
            "nai-n",
            "nakereba",
            "shimashou",
            "dewanai",
            "teba",
            "reba",
            "keba",
            "geba",
            "neba",
            "beba",
            "meba",
            "seba",
            "ii",
        ] {
            assert!(lookup_suffix_fn(key).is_some(), "missing key: {}", key);
        }
    }

    #[test]
    fn unregistered_keys_return_none() {
        assert!(lookup_suffix_fn("").is_none());
        assert!(lookup_suffix_fn("unknown").is_none());
    }

    #[test]
    fn entry_count_matches_upstream() {
        assert_eq!(SUFFIX_LIST.len(), 43);
    }
}
