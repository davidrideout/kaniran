//! Fixture replay support.
//!
//! Fixtures are JSONL or parquet rows captured by `ichiran-extractor` /
//! the `:ichi-trace` package against the live ichiran image on `.103`.
//! Each row describes one call to a Lisp function:
//!
//! ```text
//! {"fn":"ICHIRAN/CHARACTERS:TEST-WORD","args":"(\"ねこ\" :HIRAGANA)","result":"(0 2 #() #())"}
//! ```
//!
//! The `args` and `result` fields hold the original values as Lisp source
//! text (lossless round-trip via `prin1` / our parser). Rust replays a
//! fixture by deserializing the envelope, parsing the Lisp text via
//! [`crate::kani::sexp::parse`], calling the port with the parsed
//! arguments, and comparing the produced value against the expected one.

use serde::{Deserialize, Serialize};

use crate::kani::sexp::{self, ParseError, Sexp};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Fixture {
    #[serde(rename = "fn")]
    pub function: String,
    pub args: String,
    pub result: String,
}

impl Fixture {
    pub fn from_jsonl_line(line: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(line)
    }

    pub fn parse_args(&self) -> Result<Sexp, ParseError> {
        sexp::parse(&self.args)
    }

    pub fn parse_result(&self) -> Result<Sexp, ParseError> {
        sexp::parse(&self.result)
    }
}

pub fn read_jsonl(path: &std::path::Path) -> std::io::Result<Vec<Fixture>> {
    use std::io::{BufRead, BufReader};
    let f = std::fs::File::open(path)?;
    let mut out = Vec::new();
    for (i, line) in BufReader::new(f).lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let fx = Fixture::from_jsonl_line(&line).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("line {}: {}", i + 1, e),
            )
        })?;
        out.push(fx);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_TEST_WORD: &str = r#"{"fn":"ICHIRAN/CHARACTERS:TEST-WORD","args":"(\"ねこ\" :HIRAGANA)","result":"(0 2 #() #())"}"#;
    const SAMPLE_MORA_LENGTH: &str = r#"{"fn":"ICHIRAN/CHARACTERS:MORA-LENGTH","args":"(\"きょう\")","result":"(2)"}"#;

    #[test]
    fn parses_jsonl_envelope() {
        let f = Fixture::from_jsonl_line(SAMPLE_TEST_WORD).unwrap();
        assert_eq!(f.function, "ICHIRAN/CHARACTERS:TEST-WORD");
        assert_eq!(f.args, r#"("ねこ" :HIRAGANA)"#);
        assert_eq!(f.result, "(0 2 #() #())");
    }

    #[test]
    fn parses_lisp_args_with_keyword() {
        let f = Fixture::from_jsonl_line(SAMPLE_TEST_WORD).unwrap();
        let args = f.parse_args().unwrap();
        let elems: Vec<_> = args.list_iter().unwrap().collect();
        assert_eq!(elems.len(), 2);
        assert_eq!(elems[0].as_str(), Some("ねこ"));
        assert_eq!(elems[1].as_keyword(), Some("HIRAGANA"));
    }

    #[test]
    fn parses_lisp_result_with_vector() {
        let f = Fixture::from_jsonl_line(SAMPLE_TEST_WORD).unwrap();
        let result = f.parse_result().unwrap();
        let elems: Vec<_> = result.list_iter().unwrap().collect();
        assert_eq!(elems.len(), 4);
        assert_eq!(elems[0].as_i64(), Some(0));
        assert_eq!(elems[1].as_i64(), Some(2));
        assert!(elems[2].is_vector());
        assert!(elems[3].is_vector());
    }

    #[test]
    fn parses_single_value_result() {
        let f = Fixture::from_jsonl_line(SAMPLE_MORA_LENGTH).unwrap();
        let result = f.parse_result().unwrap();
        let elems: Vec<_> = result.list_iter().unwrap().collect();
        assert_eq!(elems.len(), 1);
        assert_eq!(elems[0].as_i64(), Some(2));
    }
}
