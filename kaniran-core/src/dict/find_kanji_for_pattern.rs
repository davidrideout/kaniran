//! Port of `ichiran/dict:find-kanji-for-pattern` (`dict.lisp:1882`).
//!
//! For each `kana_text` row matching `pattern`, collects its
//! `get-kanji` surface (when non-nil) and its `text`, then returns both
//! lists with duplicates removed keeping the first occurrence.

use std::collections::HashSet;

use super::find_word_kana_pattern::find_word_kana_pattern;
use super::get_kanji::get_kanji;
use super::kani_word::KaniWordDispatchEnum;
use super::counters::methods::text;
use crate::conn::kani_context::KaniranContext;

pub async fn find_kanji_for_pattern(
    ctx: &KaniranContext,
    pattern: &str,
) -> Result<(Vec<String>, Vec<String>), sqlx::Error> {
    let mut kanji: Vec<String> = Vec::new();
    let mut kana: Vec<String> = Vec::new();
    // (loop for r in (find-word-kana-pattern pattern) …)
    for r in find_word_kana_pattern(ctx, pattern).await? {
        let r = KaniWordDispatchEnum::Kana(r);
        // for k = (get-kanji r) / when k collect k into kanji
        if let Some(k) = get_kanji(ctx, &r).await? {
            kanji.push(k);
        }
        // collect (text r) into kana
        kana.push(text(&r).into_owned());
    }
    // (values (remove-duplicates kanji :test 'equal :from-end t)
    //         (remove-duplicates kana  :test 'equal :from-end t))
    Ok((remove_duplicates_from_end(kanji), remove_duplicates_from_end(kana)))
}

// `(remove-duplicates … :test 'equal :from-end t)` — keep the first
// occurrence of each value, preserving order.
fn remove_duplicates_from_end(items: Vec<String>) -> Vec<String> {
    let mut seen: HashSet<String> = HashSet::new();
    items.into_iter().filter(|item| seen.insert(item.clone())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    /// REPL fixtures (.103, `ichiran/dict::find-kanji-for-pattern`), 2026-05-25.
    /// `^つくえ$` is a single reading with a single kanji; `^がっこう$`
    /// pins kanji order by distinct `common` (学校 = 1 before 楽校 = null);
    /// `^あれ$` exercises the `when k` skip (its fourth `あれ` row has no
    /// kanji) plus the kana dedup (four rows collapse to one `あれ`);
    /// `^xyzzlkj$` returns two empty lists.
    #[tokio::test]
    async fn find_kanji_for_pattern_fixtures() {
        let ctx = ctx_from_env().await;
        let cases: &[(&str, Vec<&str>, Vec<&str>)] = &[
            ("^つくえ$", vec!["机"], vec!["つくえ"]),
            ("^がっこう$", vec!["学校", "楽校"], vec!["がっこう"]),
            ("^あれ$", vec!["荒れ", "彼", "有れ"], vec!["あれ"]),
            ("^xyzzlkj$", vec![], vec![]),
        ];
        for (pattern, expected_kanji, expected_kana) in cases {
            let (kanji, kana) = find_kanji_for_pattern(&ctx, pattern).await.unwrap();
            assert_eq!(&kanji, expected_kanji, "pattern={pattern:?} (kanji)");
            assert_eq!(&kana, expected_kana, "pattern={pattern:?} (kana)");
        }
    }

    #[test]
    fn dedup_keeps_first_occurrence() {
        // `:from-end t` keeps the first occurrence and preserves order.
        let input = vec![
            "橋".to_string(),
            "端".to_string(),
            "橋".to_string(),
            "箸".to_string(),
            "端".to_string(),
        ];
        assert_eq!(
            remove_duplicates_from_end(input),
            vec!["橋".to_string(), "端".to_string(), "箸".to_string()]
        );
    }
}
