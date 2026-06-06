//! Transliteration of `ichiran/dict:get-senses-raw` (`dict.lisp:1458`).
//!
//! Returns one [`RawSense`] per `sense` row attached to `seq`, ordered
//! by `sense.ord`, carrying the joined gloss string and the `(tag,
//! texts)` props (pos / s_inf / stagk / stagr / field).

use crate::conn::kani_context::KaniranContext;
use sqlx::Row;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSense {
    pub ord: i32,
    pub gloss: String,
    pub props: Vec<(String, Vec<String>)>,
}

const TAGS: &[&str] = &["pos", "s_inf", "stagk", "stagr", "field"];

pub async fn get_senses_raw(
    ctx: &KaniranContext,
    seq: i32,
) -> Result<Vec<RawSense>, sqlx::Error> {
    let gloss_rows = sqlx::query(
        "SELECT sense.ord AS ord, \
                string_agg(gloss.text, '; ' ORDER BY gloss.ord) AS gloss \
         FROM sense LEFT JOIN gloss ON gloss.sense_id = sense.id \
         WHERE sense.seq = $1 \
         GROUP BY sense.id \
         ORDER BY sense.ord",
    )
    .bind(seq)
    .fetch_all(&ctx.pool)
    .await?;

    let mut sense_list: Vec<RawSense> = Vec::with_capacity(gloss_rows.len());
    for row in gloss_rows {
        let ord: i32 = row.get("ord");
        let gloss: Option<String> = row.get("gloss");
        sense_list.push(RawSense {
            ord,
            gloss: gloss.unwrap_or_default(),
            props: Vec::new(),
        });
    }

    let prop_rows = sqlx::query(
        "SELECT sense.ord AS ord, sense_prop.tag AS tag, sense_prop.text AS text \
         FROM sense, sense_prop \
         WHERE sense.seq = $1 \
           AND sense_prop.sense_id = sense.id \
           AND sense_prop.tag = ANY($2) \
         ORDER BY sense.ord, sense_prop.tag, sense_prop.ord",
    )
    .bind(seq)
    .bind(TAGS)
    .fetch_all(&ctx.pool)
    .await?;

    let mut cur_sord: Option<i32> = None;
    let mut cur_tag: Option<String> = None;
    let mut cur_idx: Option<usize> = None;
    let mut bag: Vec<String> = Vec::new();

    for row in prop_rows {
        let sord: i32 = row.get("ord");
        let tag: String = row.get("tag");
        let text: String = row.get("text");

        let changed = cur_sord != Some(sord) || cur_tag.as_deref() != Some(tag.as_str());
        if changed {
            // dict.lisp:1479 (in-loop transition) — emit prior bag in
            // upstream insertion order (Lisp `(reverse bag)` flips
            // `push`-prepended order; Rust `Vec::push` is already in
            // insertion order so no reverse is applied).
            if let Some(idx) = cur_idx {
                let prev_tag = cur_tag.take().unwrap_or_default();
                let prev_bag = std::mem::take(&mut bag);
                sense_list[idx].props.insert(0, (prev_tag, prev_bag));
            }
            cur_sord = Some(sord);
            cur_tag = Some(tag);
            bag.clear();
            cur_idx = sense_list.iter().position(|s| s.ord == sord);
        }
        bag.push(text);
    }
    // dict.lisp:1483 (finally clause) — upstream emits `(cons curtag
    // bag)` without `reverse`, leaving the final group's texts in
    // reverse insertion order. The Rust `Vec::push` produced
    // insertion order, so reverse here to mirror the asymmetry.
    if let Some(idx) = cur_idx {
        let prev_tag = cur_tag.take().unwrap_or_default();
        bag.reverse();
        sense_list[idx].props.insert(0, (prev_tag, bag));
    }

    Ok(sense_list)
}

#[cfg(test)]
mod tests {
    //! All expected values pinned against .103 REPL runs of
    //! `(get-senses-raw <seq>)`. Test threads must be 1 —
    //! `cargo test --test-threads=1` per the project's DB-test
    //! convention.
    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    #[tokio::test]
    async fn unknown_seq_returns_empty() {
        // REPL: (get-senses-raw 999999) => NIL
        let ctx = ctx_from_env().await;
        let result = get_senses_raw(&ctx, 999999).await.unwrap();
        assert_eq!(result, Vec::<RawSense>::new());
    }

    #[tokio::test]
    async fn simple_single_sense() {
        // REPL: (get-senses-raw 1582710)
        // => ((:ORD 0 :GLOSS "Japan" :PROPS (("pos" "n"))))
        let ctx = ctx_from_env().await;
        let result = get_senses_raw(&ctx, 1582710).await.unwrap();
        assert_eq!(
            result,
            vec![RawSense {
                ord: 0,
                gloss: "Japan".to_string(),
                props: vec![("pos".to_string(), vec!["n".to_string()])],
            }]
        );
    }

    #[tokio::test]
    async fn multi_value_pos_single_sense() {
        // REPL: (get-senses-raw 1577900)
        // => ((:ORD 0 :GLOSS "eternity" :PROPS (("pos" "adj-no" "n"))))
        let ctx = ctx_from_env().await;
        let result = get_senses_raw(&ctx, 1577900).await.unwrap();
        assert_eq!(
            result,
            vec![RawSense {
                ord: 0,
                gloss: "eternity".to_string(),
                props: vec![(
                    "pos".to_string(),
                    vec!["adj-no".to_string(), "n".to_string()],
                )],
            }]
        );
    }

    #[tokio::test]
    async fn field_tag_present() {
        // REPL: (get-senses-raw 1001390)
        // => ((:ORD 0 :GLOSS "oden; dish of various ingredients, e.g.
        //      egg, daikon, potato, chikuwa, konnyaku stewed in
        //      soy-flavored dashi"
        //     :PROPS (("pos" "n") ("field" "food"))))
        let ctx = ctx_from_env().await;
        let result = get_senses_raw(&ctx, 1001390).await.unwrap();
        assert_eq!(
            result,
            vec![RawSense {
                ord: 0,
                gloss: "oden; dish of various ingredients, e.g. egg, daikon, potato, chikuwa, konnyaku stewed in soy-flavored dashi".to_string(),
                props: vec![
                    ("pos".to_string(), vec!["n".to_string()]),
                    ("field".to_string(), vec!["food".to_string()]),
                ],
            }]
        );
    }

    #[tokio::test]
    async fn stagk_tag_and_multiple_pos() {
        // REPL: (get-senses-raw 1000300)
        // => ((:ORD 0 :GLOSS "to treat; to handle; to deal with"
        //      :PROPS (("stagk" "遇う") ("pos" "v5u" "vt")))
        //     (:ORD 1 :GLOSS "to arrange; to decorate (with); to adorn
        //      (with); to dress (with); to garnish (with)"
        //      :PROPS (("pos" "vt" "v5u"))))
        let ctx = ctx_from_env().await;
        let result = get_senses_raw(&ctx, 1000300).await.unwrap();
        assert_eq!(
            result,
            vec![
                RawSense {
                    ord: 0,
                    gloss: "to treat; to handle; to deal with".to_string(),
                    props: vec![
                        ("stagk".to_string(), vec!["遇う".to_string()]),
                        (
                            "pos".to_string(),
                            vec!["v5u".to_string(), "vt".to_string()],
                        ),
                    ],
                },
                RawSense {
                    ord: 1,
                    gloss: "to arrange; to decorate (with); to adorn (with); to dress (with); to garnish (with)".to_string(),
                    props: vec![(
                        "pos".to_string(),
                        vec!["vt".to_string(), "v5u".to_string()],
                    )],
                },
            ]
        );
    }

    #[tokio::test]
    async fn final_group_bag_not_reversed_asymmetry() {
        // REPL: (get-senses-raw 1011960)
        // Pins the upstream asymmetry: sense 1's `stagr` is
        // ("ボタボタ" "ぼたぼた") — the in-loop `(reverse bag)`
        // path; sense 2's `stagr` is ("ぼたぼた" "ボタボタ") — the
        // `finally` path without reverse. Same two sense_prop.ord
        // 0/1 rows in both senses.
        let ctx = ctx_from_env().await;
        let result = get_senses_raw(&ctx, 1011960).await.unwrap();
        assert_eq!(
            result,
            vec![
                RawSense {
                    ord: 0,
                    gloss: "dripping; trickling; drop by drop; in drops".to_string(),
                    props: vec![(
                        "pos".to_string(),
                        vec!["adv".to_string(), "adv-to".to_string(), "vs".to_string()],
                    )],
                },
                RawSense {
                    ord: 1,
                    gloss: "wet and heavy (snow, clay, etc.)".to_string(),
                    props: vec![
                        (
                            "stagr".to_string(),
                            vec!["ボタボタ".to_string(), "ぼたぼた".to_string()],
                        ),
                        (
                            "pos".to_string(),
                            vec!["adv".to_string(), "adv-to".to_string(), "vs".to_string()],
                        ),
                    ],
                },
                RawSense {
                    ord: 2,
                    gloss: "(moving) slowly".to_string(),
                    props: vec![
                        (
                            "stagr".to_string(),
                            vec!["ぼたぼた".to_string(), "ボタボタ".to_string()],
                        ),
                        (
                            "pos".to_string(),
                            vec!["adv".to_string(), "adv-to".to_string()],
                        ),
                    ],
                },
            ]
        );
    }

    #[tokio::test]
    async fn sense_with_no_props_yields_empty_props() {
        // REPL: (get-senses-raw 1447690)
        // => ((:ORD 0 :GLOSS "Tokyo" :PROPS (("pos" "n")))
        //     (:ORD 1 :GLOSS "Tokyo Metropolis" :PROPS NIL))
        let ctx = ctx_from_env().await;
        let result = get_senses_raw(&ctx, 1447690).await.unwrap();
        assert_eq!(
            result,
            vec![
                RawSense {
                    ord: 0,
                    gloss: "Tokyo".to_string(),
                    props: vec![("pos".to_string(), vec!["n".to_string()])],
                },
                RawSense {
                    ord: 1,
                    gloss: "Tokyo Metropolis".to_string(),
                    props: Vec::new(),
                },
            ]
        );
    }
}
