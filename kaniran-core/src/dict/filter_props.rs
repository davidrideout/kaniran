//! Port of `ichiran/dict:filter-props` (`dict.lisp:1627`).
//!
//! Removes Passive (conj-type 6) props on `v1`/`v1s`/`vk` words whose
//! text isn't a れる form. `text` may be nil, one string, or a list.

use super::conj_prop_dao::ConjProp;
use super::is_rareru::is_rareru;

#[derive(Clone, Copy)]
pub enum FilterPropsText<'a> {
    None,
    One(&'a str),
    Many(&'a [&'a str]),
}

pub fn filter_props<'a>(props: &'a [ConjProp], text: FilterPropsText<'_>) -> Vec<&'a ConjProp> {
    let mut result = Vec::new();
    for prop in props {
        // (and text (= (conj-type prop) 6) (find (pos prop) '("v1" "v1s" "vk")) (not …))
        let drop_passive = match text {
            // text nil → (and text …) short-circuits → keep prop
            FilterPropsText::None => false,
            FilterPropsText::One(text) => {
                prop.conj_type == 6
                    && ["v1", "v1s", "vk"].contains(&prop.pos.as_str())
                    && !is_rareru(text)
            }
            // empty list is nil → (and text …) short-circuits → keep prop
            FilterPropsText::Many(text) => {
                !text.is_empty()
                    && prop.conj_type == 6
                    && ["v1", "v1s", "vk"].contains(&prop.pos.as_str())
                    && !text.iter().any(|text| is_rareru(text))
            }
        };
        if !drop_passive {
            result.push(prop);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prop(conj_id: i32, conj_type: i32, pos: &str) -> ConjProp {
        ConjProp {
            id: conj_id,
            conj_id,
            conj_type,
            pos: pos.to_string(),
            neg: None,
            fml: None,
        }
    }

    fn ids(props: &[&ConjProp]) -> Vec<i32> {
        props.iter().map(|prop| prop.conj_id).collect()
    }

    /// REPL fixtures (.103, `ichiran/dict::filter-props`), 2026-05-24.
    /// `props` = passive v1 (1), passive v5r (2, pos out of set), plain v1
    /// (3, conj-type ≠ 6), passive v1s (4), passive vk (5). Each row drops
    /// the passive v1/v1s/vk props (1,4,5) only when text is non-nil and
    /// not a rareru form. Covers nil, single string (rareru / non-rareru /
    /// empty-but-truthy), and list (with / without a rareru member /
    /// empty-list-is-nil).
    #[test]
    fn filter_props_fixtures() {
        let props = vec![
            prop(1, 6, "v1"),
            prop(2, 6, "v5r"),
            prop(3, 1, "v1"),
            prop(4, 6, "v1s"),
            prop(5, 6, "vk"),
        ];
        let some = ["食べる", "見られる"];
        let none = ["食べる", "飲む"];
        let cases: &[(FilterPropsText, Vec<i32>)] = &[
            (FilterPropsText::None, vec![1, 2, 3, 4, 5]),
            (FilterPropsText::One("食べる"), vec![2, 3]),
            (FilterPropsText::One("食べられる"), vec![1, 2, 3, 4, 5]),
            (FilterPropsText::One(""), vec![2, 3]),
            (FilterPropsText::Many(&none), vec![2, 3]),
            (FilterPropsText::Many(&some), vec![1, 2, 3, 4, 5]),
            (FilterPropsText::Many(&[]), vec![1, 2, 3, 4, 5]),
        ];
        for (text, expected) in cases {
            assert_eq!(ids(&filter_props(&props, *text)), *expected);
        }
        // empty props → empty result
        assert!(filter_props(&[], FilterPropsText::One("食べる")).is_empty());
    }
}
