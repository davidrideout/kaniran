//! Port of `ichiran/dict:get-senses` (`dict.lisp:1487`).
//!
//! ```lisp
//! (defun get-senses (seq)
//!   (loop for sense in (get-senses-raw seq)
//!        for props = (getf sense :props)
//!        for gloss = (getf sense :gloss)
//!        for pos = (cdr (assoc "pos" props :test 'equal))
//!        for pos-str = (format nil "[~{~a~^,~}]" pos)
//!        collect (list pos-str gloss props)))
//! ```

use crate::conn::kani_context::KaniranContext;
use crate::dict::get_senses_raw::get_senses_raw;

pub type SenseEntry = (String, String, Vec<(String, Vec<String>)>);

pub async fn get_senses(
    ctx: &KaniranContext,
    seq: i32,
) -> Result<Vec<SenseEntry>, sqlx::Error> {
    let raw = get_senses_raw(ctx, seq).await?;
    let mut out: Vec<SenseEntry> = Vec::with_capacity(raw.len());
    for sense in raw {
        let pos_str = {
            let pos: &[String] = sense
                .props
                .iter()
                .find(|(tag, _)| tag == "pos")
                .map(|(_, vals)| vals.as_slice())
                .unwrap_or(&[]);
            format!("[{}]", pos.join(","))
        };
        out.push((pos_str, sense.gloss, sense.props));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    //! All expected values pinned against .103 REPL runs of
    //! `(ichiran/dict::get-senses <seq>)`. Run with `--test-threads=1`.
    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    #[tokio::test]
    async fn unknown_seq_returns_empty() {
        // REPL: (get-senses 999999) => NIL
        let ctx = ctx_from_env().await;
        let result = get_senses(&ctx, 999999).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn simple_single_sense() {
        // REPL: (get-senses 1582710)
        // => (("[n]" "Japan" (("pos" "n"))))
        let ctx = ctx_from_env().await;
        let result = get_senses(&ctx, 1582710).await.unwrap();
        assert_eq!(
            result,
            vec![(
                "[n]".to_string(),
                "Japan".to_string(),
                vec![("pos".to_string(), vec!["n".to_string()])],
            )]
        );
    }

    #[tokio::test]
    async fn multi_value_pos() {
        // REPL: (get-senses 1577900)
        // => (("[adj-no,n]" "eternity" (("pos" "adj-no" "n"))))
        let ctx = ctx_from_env().await;
        let result = get_senses(&ctx, 1577900).await.unwrap();
        assert_eq!(
            result,
            vec![(
                "[adj-no,n]".to_string(),
                "eternity".to_string(),
                vec![("pos".to_string(), vec!["adj-no".to_string(), "n".to_string()])],
            )]
        );
    }

    #[tokio::test]
    async fn field_tag_preserved_in_props() {
        // REPL: (get-senses 1001390)
        // => (("[n]"
        //       "oden; dish of various ingredients, e.g. egg, daikon,
        //        potato, chikuwa, konnyaku stewed in soy-flavored dashi"
        //       (("pos" "n") ("field" "food"))))
        let ctx = ctx_from_env().await;
        let result = get_senses(&ctx, 1001390).await.unwrap();
        assert_eq!(
            result,
            vec![(
                "[n]".to_string(),
                "oden; dish of various ingredients, e.g. egg, daikon, potato, chikuwa, konnyaku stewed in soy-flavored dashi".to_string(),
                vec![
                    ("pos".to_string(), vec!["n".to_string()]),
                    ("field".to_string(), vec!["food".to_string()]),
                ],
            )]
        );
    }

    #[tokio::test]
    async fn second_sense_no_pos_yields_empty_brackets() {
        // REPL: (get-senses 1447690)
        // => (("[n]" "Tokyo" (("pos" "n")))
        //     ("[]" "Tokyo Metropolis" NIL))
        let ctx = ctx_from_env().await;
        let result = get_senses(&ctx, 1447690).await.unwrap();
        assert_eq!(
            result,
            vec![
                (
                    "[n]".to_string(),
                    "Tokyo".to_string(),
                    vec![("pos".to_string(), vec!["n".to_string()])],
                ),
                (
                    "[]".to_string(),
                    "Tokyo Metropolis".to_string(),
                    Vec::new(),
                ),
            ]
        );
    }
}
