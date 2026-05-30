//! Port of `ichiran/dict:get-senses-str` (`dict.lisp:1495`).
//!
//! ```lisp
//! (defun get-senses-str (seq)
//!   (with-output-to-string (s)
//!     (loop for (pos gloss props) in (get-senses seq)
//!           for i from 1
//!           for rpos = pos then (if (equal pos "[]") rpos pos)
//!           for inf = (cdr (assoc "s_inf" props :test 'equal))
//!           for rinf = (when inf (join "; " inf))
//!           for field = (cdr (assoc "field" props :test 'equal))
//!           for rfield = (when field (join "," field))
//!           when (> i 1) do (terpri s)
//!           do (format s "~a. ~a ~@[{~a} ~]~@[《~a》 ~]~a" i rpos rfield rinf gloss))))
//! ```

use std::fmt::Write;

use crate::characters::text_utils::join;
use crate::conn::kani_context::KaniranContext;
use crate::dict::get_senses::get_senses;

pub async fn get_senses_str(
    ctx: &KaniranContext,
    seq: i32,
) -> Result<String, sqlx::Error> {
    let senses = get_senses(ctx, seq).await?;
    let mut out = String::new();
    let mut rpos: &str = "";
    for (i, (pos, gloss, props)) in senses.iter().enumerate() {
        // dict.lisp:1499 (loop for rpos = pos then …) — first iter seeds rpos,
        // later iters keep the prior rpos when the current pos is "[]".
        if i == 0 {
            rpos = pos.as_str();
        } else {
            if pos != "[]" {
                rpos = pos.as_str();
            }
            out.push('\n');
        }
        let inf = props
            .iter()
            .find(|(tag, _)| tag == "s_inf")
            .map(|(_, vals)| join("; ", vals));
        let field = props
            .iter()
            .find(|(tag, _)| tag == "field")
            .map(|(_, vals)| join(",", vals));
        write!(out, "{}. {} ", i + 1, rpos).unwrap();
        if let Some(f) = &field {
            write!(out, "{{{}}} ", f).unwrap();
        }
        if let Some(s) = &inf {
            write!(out, "《{}》 ", s).unwrap();
        }
        out.push_str(gloss);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    //! All expected values pinned against .103 REPL runs of
    //! `(ichiran/dict::get-senses-str <seq>)`. Run with `--test-threads=1`.
    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    #[tokio::test]
    async fn unknown_seq_yields_empty_string() {
        // REPL: (get-senses-str 999999) => ""
        let ctx = ctx_from_env().await;
        let result = get_senses_str(&ctx, 999999).await.unwrap();
        assert_eq!(result, "");
    }

    #[tokio::test]
    async fn simple_single_sense() {
        // REPL: (get-senses-str 1582710) => "1. [n] Japan"
        let ctx = ctx_from_env().await;
        let result = get_senses_str(&ctx, 1582710).await.unwrap();
        assert_eq!(result, "1. [n] Japan");
    }

    #[tokio::test]
    async fn multi_value_pos() {
        // REPL: (get-senses-str 1577900) => "1. [adj-no,n] eternity"
        let ctx = ctx_from_env().await;
        let result = get_senses_str(&ctx, 1577900).await.unwrap();
        assert_eq!(result, "1. [adj-no,n] eternity");
    }

    #[tokio::test]
    async fn field_braced_before_gloss() {
        // REPL: (get-senses-str 1001390) =>
        //   "1. [n] {food} oden; dish of various ingredients, e.g. egg,
        //    daikon, potato, chikuwa, konnyaku stewed in soy-flavored dashi"
        let ctx = ctx_from_env().await;
        let result = get_senses_str(&ctx, 1001390).await.unwrap();
        assert_eq!(
            result,
            "1. [n] {food} oden; dish of various ingredients, e.g. egg, daikon, potato, chikuwa, konnyaku stewed in soy-flavored dashi"
        );
    }

    #[tokio::test]
    async fn multi_field_joined_by_comma() {
        // REPL: (get-senses-str 1014100) => "1. [n] {physics,chem} isotope"
        let ctx = ctx_from_env().await;
        let result = get_senses_str(&ctx, 1014100).await.unwrap();
        assert_eq!(result, "1. [n] {physics,chem} isotope");
    }

    #[tokio::test]
    async fn s_inf_in_double_angle_brackets() {
        // REPL: (get-senses-str 900000) =>
        //   "1. [suf] 《after the -masu stem of a verb》 to seem to want to (do something)"
        let ctx = ctx_from_env().await;
        let result = get_senses_str(&ctx, 900000).await.unwrap();
        assert_eq!(
            result,
            "1. [suf] 《after the -masu stem of a verb》 to seem to want to (do something)"
        );
    }

    #[tokio::test]
    async fn field_and_s_inf_both_present() {
        // REPL: (get-senses-str 1005660) =>
        //   "1. [n] {food} 《from the sound of the dish being prepared》 shabu-shabu; …"
        let ctx = ctx_from_env().await;
        let result = get_senses_str(&ctx, 1005660).await.unwrap();
        assert_eq!(
            result,
            "1. [n] {food} 《from the sound of the dish being prepared》 shabu-shabu; hot pot dish where thinly sliced meat is boiled quickly and then dipped in sauce"
        );
    }

    #[tokio::test]
    async fn multi_sense_separated_by_newline_no_trailing() {
        // REPL: (get-senses-str 1447690) =>
        //   "1. [n] Tokyo\n2. [n] Tokyo Metropolis"
        // sense 2 has pos "[]" → rpos inherits "[n]" from sense 1.
        let ctx = ctx_from_env().await;
        let result = get_senses_str(&ctx, 1447690).await.unwrap();
        assert_eq!(result, "1. [n] Tokyo\n2. [n] Tokyo Metropolis");
    }

    #[tokio::test]
    async fn three_senses_mixed_props() {
        // REPL: (get-senses-str 1011960) =>
        //   "1. [adv,adv-to,vs] dripping; trickling; drop by drop; in drops
        //    2. [adv,adv-to,vs] wet and heavy (snow, clay, etc.)
        //    3. [adv,adv-to] (moving) slowly"
        let ctx = ctx_from_env().await;
        let result = get_senses_str(&ctx, 1011960).await.unwrap();
        assert_eq!(
            result,
            "1. [adv,adv-to,vs] dripping; trickling; drop by drop; in drops\n2. [adv,adv-to,vs] wet and heavy (snow, clay, etc.)\n3. [adv,adv-to] (moving) slowly"
        );
    }

    #[tokio::test]
    async fn five_senses_with_s_inf_subset() {
        // REPL: (get-senses-str 1000090) — pinned 5-sense output.
        let ctx = ctx_from_env().await;
        let result = get_senses_str(&ctx, 1000090).await.unwrap();
        assert_eq!(
            result,
            "1. [n] 《sometimes used for zero》 circle\n2. [n] 《when marking a test, homework, etc.》 \"correct\"; \"good\"\n3. [unc] 《placeholder used to censor individual characters or indicate a space to be filled in》 *; _\n4. [n] period; full stop\n5. [n] handakuten (diacritic)"
        );
    }
}
