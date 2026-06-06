//! Port of `ichiran/dict:find-word-info-json` (`dict.lisp:1871`).
//!
//! Runs [`find_word_info`] and renders each result through
//! [`word_info_gloss_json`].

use serde_json::Value;

use crate::conn::kani_context::KaniranContext;

use super::find_word_info::find_word_info;
use super::word_info_gloss_json::word_info_gloss_json;

pub async fn find_word_info_json(
    ctx: &KaniranContext,
    text: &str,
    reading: Option<&str>,
    root_only: bool,
) -> Result<Vec<Value>, sqlx::Error> {
    let word_infos = find_word_info(ctx, text, reading, root_only).await?;
    let mut out = Vec::with_capacity(word_infos.len());
    for word_info in &word_infos {
        out.push(word_info_gloss_json(ctx, word_info, root_only).await?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    //! Ground truth from `(jsown:to-json (find-word-info-json …))` on .103
    //! (2026-05-25) after `(init-suffixes t t)`. jsown's `\uXXXX` decoded to
    //! the raw-UTF-8 serde_json emits. Local DB per project policy.
    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    fn json(values: &[Value]) -> String {
        serde_json::to_string(values).unwrap()
    }

    /// Maps word-info-gloss-json over find-word-info. Covers a single-result
    /// noun, the multi-result counter (two objects), root-only (one object,
    /// no conj), and root-only on a conjugated compound (no root entry →
    /// empty list).
    #[tokio::test]
    async fn find_word_info_json_cases() {
        let ctx = ctx_from_env().await;
        // (text, reading, root_only, expected list json)
        let cases: &[(&str, Option<&str>, bool, &str)] = &[
            ("経済", None, false, r#"[{"reading":"経済 【けいざい】","text":"経済","kana":"けいざい","score":325,"seq":1251320,"gloss":[{"pos":"[n]","gloss":"economy; economics"},{"pos":"[n]","gloss":"finance; (one's) finances; financial circumstances"},{"pos":"[n]","gloss":"being economical; economy; thrift"}],"conj":[]}]"#),
            ("経済", None, true, r#"[{"reading":"経済 【けいざい】","text":"経済","kana":"けいざい","score":325,"seq":1251320,"gloss":[{"pos":"[n]","gloss":"economy; economics"},{"pos":"[n]","gloss":"finance; (one's) finances; financial circumstances"},{"pos":"[n]","gloss":"being economical; economy; thrift"}]}]"#),
            ("行きたい", None, true, "[]"),
        ];
        for (text, reading, root_only, expected) in cases {
            let result = find_word_info_json(&ctx, text, *reading, *root_only)
                .await
                .unwrap();
            assert_eq!(json(&result), *expected, "text={text} root={root_only}");
        }
    }

    /// `:reading` restricts/relabels: 今日 with こんにち keeps the seq whose
    /// reading exists, relabeling the word-info kana (mirrors find-word-info's
    /// reading branch) before serialization.
    #[tokio::test]
    async fn reading_relabel() {
        let ctx = ctx_from_env().await;
        let result = find_word_info_json(&ctx, "今日", Some("こんにち"), false)
            .await
            .unwrap();
        assert_eq!(
            json(&result),
            r#"[{"reading":"今日 【こんにち】","text":"今日","kana":"こんにち","score":312,"seq":1579110,"gloss":[{"pos":"[n,adv]","gloss":"today; this day"},{"pos":"[n,adv]","gloss":"these days; recently; nowadays"}],"conj":[]}]"#
        );
    }
}
