//! Port of `ichiran/dict:match-glosses` (`dict.lisp:1920`).
//!
//! Resolves `(text, reading)` to candidate entry seqs, then for each
//! candidate scans its glosses in DB order looking for either an
//! `update_gloss` regex match (priority 1, returns the original gloss)
//! or a match where every supplied `word` appears in the normalized
//! gloss (priority 2). First hit across `(candidate, gloss)` wins; when
//! nothing matches the first candidate is returned with `found_p =
//! false`. Empty candidates returns [`None`].

use crate::conn::kani_context::KaniranContext;

use super::get_candidates::get_candidates;
use super::get_glosses::get_glosses;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchValue {
    /// Upstream returned `seq` alone — either the all-words-match arm
    /// (`found_p = true`) or the no-match fallback arm using
    /// `(car candidates)` (`found_p = false`).
    Seq(i32),
    /// Upstream returned `(list seq match)` — the `update-gloss`
    /// regex arm. `match` is the original (un-normalized) gloss
    /// string.
    SeqAndGloss(i32, String),
}

pub async fn match_glosses(
    ctx: &KaniranContext,
    text: &str,
    reading: Option<&str>,
    words: &[&str],
    normalize: Option<&dyn Fn(&str) -> String>,
    update_gloss: Option<&fancy_regex::Regex>,
) -> Result<Option<(MatchValue, bool)>, sqlx::Error> {
    let candidates = get_candidates(ctx, text, reading).await?;
    if candidates.is_empty() {
        return Ok(None);
    }
    // dict.lisp:1923 — `(nwords (mapcar normalize words))`
    let nwords: Vec<String> = words
        .iter()
        .map(|w| match normalize {
            Some(f) => f(w),
            None => (*w).to_string(),
        })
        .collect();

    // dict.lisp:1925 — `(loop for (seq . glosses) in (get-glosses candidates) ...)`
    let groups = get_glosses(ctx, &candidates).await?;
    for (seq, glosses) in groups {
        // dict.lisp:1926 — `(loop for gloss in (nreverse glosses) ...)`
        let dbo_glosses: Vec<String> = glosses.into_iter().rev().collect();
        let mut inner: Option<MatchValue> = None;
        for gloss in &dbo_glosses {
            // dict.lisp:1927 — `for ngloss = (funcall normalize gloss)`
            let ngloss = match normalize {
                Some(f) => f(gloss),
                None => gloss.clone(),
            };
            // dict.lisp:1928-1929 —
            // `when (and update-gloss (ppcre:scan update-gloss ngloss))
            //    do (return gloss)`
            if let Some(rg) = update_gloss {
                if rg
                    .is_match(&ngloss)
                    .expect("match-glosses: fancy_regex runtime error")
                {
                    inner = Some(MatchValue::SeqAndGloss(seq, gloss.clone()));
                    break;
                }
            }
            // dict.lisp:1930 —
            // `thereis (loop for word in nwords always (search word ngloss))`
            if nwords.iter().all(|w| ngloss.contains(w.as_str())) {
                inner = Some(MatchValue::Seq(seq));
                break;
            }
        }
        if let Some(m) = inner {
            return Ok(Some((m, true)));
        }
    }
    // dict.lisp:1934 — `(values (car candidates) nil)`
    Ok(Some((MatchValue::Seq(candidates[0]), false)))
}

#[cfg(test)]
mod tests {
    //! Unit tests against the real .103 PG via `KaniranContext::from_env()`.
    //! REPL probe pinned 2026-05-22 against `(match-glosses ...)` after
    //! `(ichiran/conn:with-db nil ...)`. Coverage:
    //!   `("Chinese" "character")` → words-match arm `(Seq, true)`.
    //!   `("chinese" "character")` no-normalize → fallback `(Seq, false)`.
    //!   `("chinese" "character")` + `string-downcase` → words-match `(Seq, true)`.
    //!   `("zzzz")` + update-gloss `^(?i)chinese character` → `(SeqAndGloss, true)`.
    //!   `("zzzz")` + update-gloss that misses + words that miss → fallback `(Seq, false)`.
    //!   bogus reading → `None`.
    //!   empty words → matches the first DB-order gloss (`all` on empty is vacuously true).
    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    /// REPL: `(match-glosses "漢字" "かんじ" '("Chinese" "character"))`
    /// → `matched=1213170 found-p=T`.
    #[tokio::test]
    async fn words_match_returns_seq_true() {
        let ctx = ctx_from_env().await;
        let out = match_glosses(&ctx, "漢字", Some("かんじ"), &["Chinese", "character"], None, None)
            .await
            .unwrap();
        assert_eq!(out, Some((MatchValue::Seq(1213170), true)));
    }

    /// REPL: `(match-glosses "漢字" "かんじ" '("zzzzz"))`
    /// → `matched=1213170 found-p=NIL` — fallback to first candidate.
    #[tokio::test]
    async fn no_word_match_fallback_to_first_candidate() {
        let ctx = ctx_from_env().await;
        let out = match_glosses(&ctx, "漢字", Some("かんじ"), &["zzzzz"], None, None)
            .await
            .unwrap();
        assert_eq!(out, Some((MatchValue::Seq(1213170), false)));
    }

    /// REPL: `(match-glosses "漢字" "ZZZZZ" '("x"))`
    /// → `matched=NIL found-p=NIL` — no candidates from `(text, reading)`.
    #[tokio::test]
    async fn empty_candidates_returns_none() {
        let ctx = ctx_from_env().await;
        let out = match_glosses(&ctx, "漢字", Some("ZZZZZ"), &["x"], None, None)
            .await
            .unwrap();
        assert_eq!(out, None);
    }

    /// REPL: `(match-glosses "漢字" "かんじ" '("zzzz")
    ///         :update-gloss '(:sequence :case-insensitive-p :start-anchor "chinese character"))`
    /// → `matched=(1213170 "Chinese character") found-p=T`.
    #[tokio::test]
    async fn update_gloss_match_returns_seq_and_gloss() {
        let ctx = ctx_from_env().await;
        let rg = fancy_regex::Regex::new("(?i)^chinese character").unwrap();
        let out =
            match_glosses(&ctx, "漢字", Some("かんじ"), &["zzzz"], None, Some(&rg))
                .await
                .unwrap();
        assert_eq!(
            out,
            Some((MatchValue::SeqAndGloss(1213170, "Chinese character".to_string()), true)),
        );
    }

    /// REPL: `(match-glosses "漢字" "かんじ" '("zzzz")
    ///         :update-gloss '(:sequence "XXXX"))`
    /// → `matched=1213170 found-p=NIL` — neither update-gloss nor words match.
    #[tokio::test]
    async fn update_gloss_miss_and_words_miss_fallback() {
        let ctx = ctx_from_env().await;
        let rg = fancy_regex::Regex::new("XXXX").unwrap();
        let out =
            match_glosses(&ctx, "漢字", Some("かんじ"), &["zzzz"], None, Some(&rg))
                .await
                .unwrap();
        assert_eq!(out, Some((MatchValue::Seq(1213170), false)));
    }

    /// REPL: `(match-glosses "漢字" "かんじ" '("chinese" "character") :normalize 'string-downcase)`
    /// → `matched=1213170 found-p=T` — words become matchable after normalize.
    #[tokio::test]
    async fn normalize_string_downcase_enables_match() {
        let ctx = ctx_from_env().await;
        let downcase: &dyn Fn(&str) -> String = &|s: &str| s.to_lowercase();
        let out = match_glosses(
            &ctx,
            "漢字",
            Some("かんじ"),
            &["chinese", "character"],
            Some(downcase),
            None,
        )
        .await
        .unwrap();
        assert_eq!(out, Some((MatchValue::Seq(1213170), true)));
    }

    /// REPL: same as above but no `:normalize` — case-sensitive
    /// `search` doesn't find lowercase `chinese` in `Chinese character`.
    /// → `matched=1213170 found-p=NIL`.
    #[tokio::test]
    async fn no_normalize_lowercase_falls_back() {
        let ctx = ctx_from_env().await;
        let out = match_glosses(
            &ctx,
            "漢字",
            Some("かんじ"),
            &["chinese", "character"],
            None,
            None,
        )
        .await
        .unwrap();
        assert_eq!(out, Some((MatchValue::Seq(1213170), false)));
    }

    /// REPL: `(match-glosses "漢字" "かんじ" '())`
    /// → `matched=1213170 found-p=T` — empty words is vacuously
    /// satisfied by the first DB-order gloss.
    #[tokio::test]
    async fn empty_words_matches_first_gloss() {
        let ctx = ctx_from_env().await;
        let out = match_glosses(&ctx, "漢字", Some("かんじ"), &[], None, None)
            .await
            .unwrap();
        assert_eq!(out, Some((MatchValue::Seq(1213170), true)));
    }
}
