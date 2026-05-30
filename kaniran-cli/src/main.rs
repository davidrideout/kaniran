//! `kaniran-cli` — Rust transliteration of `ichiran/cli` (`cli.lisp`).
//!
//! ```lisp
//! (defun main ()
//!   (load-connection-from-env)
//!   (multiple-value-bind (options free-args) (... (opts:get-opts) ...)
//!     (cond
//!       ((getf options :help) (opts:describe ...))
//!       ((getf options :eval) ...)
//!       ((getf options :info)
//!        (let ((input (join " " free-args)))
//!          (multiple-value-bind (r info) (romanize input :with-info t)
//!            (princ r) (print-romanize-info info))))
//!       ((getf options :full)
//!        (let* ((input (join " " free-args))
//!               (limit-value (getf options :limit))
//!               (result (romanize* input :limit limit-value)))
//!          (princ (jsown:to-json result))))
//!       (t (let ((input (join " " free-args)))
//!            (princ (romanize input :with-info t))))))
//!   (terpri) (finish-output))
//! ```
//!
//! `load-connection-from-env` becomes [`KaniranContext::from_env`]; option
//! parsing is clap rather than `unix-opts`, so `-h/--help` and parse errors
//! are clap's. The `-e/--eval` branch (`(eval (read-from-string input))`)
//! has no Rust equivalent and is omitted. `build` / `setup-debugger` are
//! image-build glue with no runtime counterpart.

use std::io::Write;

use clap::Parser;
use serde_json::{json, Value};

use kaniran_core::characters::text_utils::join;
use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::core::_star_hepburn_traditional_star_::hepburn_traditional;
use kaniran_core::core::generic_romanization_class::RomanizationMethod;
use kaniran_core::core::kani_romanize_method::KaniRomanizeMethod;
use kaniran_core::core::romanize::romanize;
use kaniran_core::core::romanize_star_::{romanize_star_, RomanizeStarSegment};
use kaniran_core::dict::word_info_gloss_json::word_info_gloss_json;

// cli.lisp:8-31 (opts:define-opts) — :eval (cli.lisp:13) omitted; :help
// (cli.lisp:9) is clap's built-in.
#[derive(Parser)]
#[command(
    name = "kaniran-cli",
    about = "Command line interface for Ichiran",
    long_about = "Command line interface for Ichiran\n\nBy default calls ichiran:romanize, other options change this behavior"
)]
struct Cli {
    /// print dictionary info
    #[arg(short = 'i', long = "with-info")]
    info: bool,
    /// full split info (as JSON)
    #[arg(short = 'f', long = "full")]
    full: bool,
    /// limit segmentations to the specified number (useful only with -f or --full) [Example: ichiran-cli -f -l 5 "一覧は最高だぞ"]
    #[arg(short = 'l', long = "limit", default_value_t = 1, value_name = "LIMIT")]
    limit: usize,
    /// input
    input: Vec<String>,
}

// cli.lisp:44 (print-romanize-info)
fn print_romanize_info(info: &[(String, String)]) {
    for (word, gloss) in info {
        print!("\n\n* {word}  {gloss}");
    }
}

// cli.lisp:41 (defmethod jsown:to-json ((word-info word-info))) +
// cli.lisp:87 (jsown:to-json result): jsown over the romanize* nested list.
// A misc split is its bare string; a word split is the list of
// (word-list score) pairs; each word is the triple (romanized word prop).
// The word-info renders via word-info-gloss-json (the cli.lisp method) and
// the prop is the default (constantly nil) wordprop-fn's nil, which jsown
// renders as [].
async fn romanize_star_to_json(
    ctx: &KaniranContext,
    result: &[RomanizeStarSegment<()>],
) -> Result<Value, sqlx::Error> {
    let mut parts = Vec::with_capacity(result.len());
    for segment in result {
        match segment {
            RomanizeStarSegment::Misc(split_text) => parts.push(Value::String(split_text.clone())),
            RomanizeStarSegment::Word(alternatives) => {
                let mut pairs = Vec::with_capacity(alternatives.len());
                for (word_list, score) in alternatives {
                    let mut words = Vec::with_capacity(word_list.len());
                    for (romanized, word, _prop) in word_list {
                        let gloss = word_info_gloss_json(ctx, word, false).await?;
                        words.push(json!([romanized, gloss, []]));
                    }
                    pairs.push(json!([words, score]));
                }
                parts.push(Value::Array(pairs));
            }
        }
    }
    Ok(Value::Array(parts))
}

// cli.lisp:48 (main)
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = Cli::parse();
    // (load-connection-from-env)
    let ctx = KaniranContext::from_env().await?;
    // method defaults to *default-romanization-method* (= *hepburn-traditional*).
    let method =
        KaniRomanizeMethod::Method(RomanizationMethod::TraditionalHepburn(hepburn_traditional()));
    // (join " " free-args)
    let input = join(" ", &options.input);
    if options.info {
        // ((getf options :info) …)
        let (r, info) = romanize(&ctx, &input, method, true).await?;
        print!("{r}");
        print_romanize_info(&info);
    } else if options.full {
        // ((getf options :full) …)
        let result = romanize_star_(&ctx, &input, method, Some(options.limit), |_, _| ()).await?;
        let json = romanize_star_to_json(&ctx, &result).await?;
        print!("{}", serde_json::to_string(&json)?);
    } else {
        // (t …)
        let (r, _) = romanize(&ctx, &input, method, true).await?;
        print!("{r}");
    }
    // (terpri) (finish-output)
    println!();
    std::io::stdout().flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    //! Ground truth from `(princ (jsown:to-json (romanize* input :limit 1)))`
    //! with the cli.lisp word-info to-json method installed, on .103
    //! (2026-05-26) after `(init-suffixes t t)`. jsown emits `\uXXXX`; the
    //! expected strings are the raw-UTF-8 round-trip serde_json produces
    //! (identical JSON). Single-reading words at limit 1 → no `/`-joined
    //! readings and one alternative, so the bytes are deterministic. Local DB
    //! per project policy; run with `-- --test-threads=1`.
    use super::*;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
    }

    fn method() -> KaniRomanizeMethod<'static> {
        KaniRomanizeMethod::Method(RomanizationMethod::TraditionalHepburn(hepburn_traditional()))
    }

    #[tokio::test]
    async fn full_json_matches_cli() {
        let ctx = ctx().await;
        // (input, limit, expected jsown:to-json output)
        let cases: &[(&str, usize, &str)] = &[
            // single word split, one alternative.
            ("世界", 1, r#"[[[[["sekai",{"reading":"世界 【せかい】","text":"世界","kana":"せかい","score":325,"seq":1373860,"gloss":[{"pos":"[n]","gloss":"the world; society; the universe"},{"pos":"[n]","gloss":"sphere; circle; world"},{"pos":"[adj-no]","gloss":"world-renowned; world-famous"},{"pos":"[n]","gloss":"realm governed by one Buddha; space","field":"{Buddh}","info":"original meaning"}],"conj":[]},[]]],325]]]"#),
            // misc + word + misc: latin prefix, word split, "! " trailer.
            ("Hello 世界！", 1, r#"["Hello ",[[[["sekai",{"reading":"世界 【せかい】","text":"世界","kana":"せかい","score":325,"seq":1373860,"gloss":[{"pos":"[n]","gloss":"the world; society; the universe"},{"pos":"[n]","gloss":"sphere; circle; world"},{"pos":"[adj-no]","gloss":"world-renowned; world-famous"},{"pos":"[n]","gloss":"realm governed by one Buddha; space","field":"{Buddh}","info":"original meaning"}],"conj":[]},[]]],325]],"! "]"#),
            // another single word split (counter-adjacent noun).
            ("三人", 1, r#"[[[[["sannin",{"reading":"三人 【さんにん】","text":"三人","kana":"さんにん","score":325,"seq":1301000,"gloss":[{"pos":"[n]","gloss":"three people"}],"conj":[]},[]]],325]]]"#),
        ];
        for (input, limit, expected) in cases {
            let result = romanize_star_(&ctx, input, method(), Some(*limit), |_, _| ())
                .await
                .unwrap();
            let json = romanize_star_to_json(&ctx, &result).await.unwrap();
            assert_eq!(serde_json::to_string(&json).unwrap(), *expected, "input={input}");
        }
    }
}
