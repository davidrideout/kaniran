//! Port of `ichiran/dict:find-substring-words` (`dict.lisp:501`).
//!
//! Builds the substring lookup cache: enumerates every length-bounded
//! substring of `str` whose `(start, end)` pair is not blocked by
//! `sticky` (positions that cannot serve as a word boundary), then
//! bulk-fetches the matching `kana_text` / `kanji_text` rows, bucketed
//! by substring text. Substrings absent from the database still get an
//! empty hash entry, signalling that the lookup already ran.

use crate::characters::char_class::CharClass;
use crate::characters::char_class::test_word;
use crate::conn::kani_context::KaniranContext;
use crate::dict::_star_max_word_length_star_::MAX_WORD_LENGTH;
use crate::dict::_star_substring_hash_star_::SubstringHash;
use crate::dict::find_word::FindWordRows;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kanji_text_dao::KanjiText;

pub async fn find_substring_words(
    ctx: &KaniranContext,
    str: &str,
    sticky: &[usize],
) -> Result<SubstringHash, sqlx::Error> {
    let mut substring_hash: SubstringHash = SubstringHash::new();
    let mut kana_keys: Vec<String> = Vec::new();
    let mut kanji_keys: Vec<String> = Vec::new();

    // dict.lisp:504-512 (loop for start ... loop for end ...). CONVENTIONS
    // §4.5: cl-ppcre / subseq index by character — collect the chars
    // once so the inner slice uses character offsets.
    let chars: Vec<char> = str.chars().collect();
    let n = chars.len();

    for start in 0..n {
        if sticky.contains(&start) {
            continue;
        }
        // (min (length str) (+ start *max-word-length*))
        let end_max = n.min(start + MAX_WORD_LENGTH);
        for end in (start + 1)..=end_max {
            if sticky.contains(&end) {
                continue;
            }
            // (subseq str start end) — character offsets per §4.5.
            let part: String = chars[start..end].iter().collect();
            // dict.lisp:510 — pre-populate hash with an empty entry,
            // then classify by kana vs. kanji.
            let is_kana = test_word(&part, CharClass::Kana);
            let empty = if is_kana {
                FindWordRows::Kana(Vec::new())
            } else {
                FindWordRows::Kanji(Vec::new())
            };
            substring_hash.insert(part.clone(), empty);
            if is_kana {
                kana_keys.push(part);
            } else {
                kanji_keys.push(part);
            }
        }
    }

    // dict.lisp:513 — (mapcar 'remove-duplicates (list kana-keys kanji-keys))
    // The upstream remove-duplicates keeps the last occurrence of each
    // string; downstream consumes the list as a SQL IN-set, so order
    // and which occurrence is dropped don't matter at the boundary —
    // sort+dedup is the cheap canonical form for the bulk query.
    kana_keys.sort();
    kana_keys.dedup();
    kanji_keys.sort();
    kanji_keys.dedup();

    // dict.lisp:514-518 — (loop for table in '(kana-text kanji-text)
    //   for keys in ... when keys do (query ...)). Unrolled by table
    //   here so the typed `query_as<KanaText>` / `query_as<KanjiText>`
    //   stays known at compile time.
    if !kana_keys.is_empty() {
        let rows: Vec<KanaText> = sqlx::query_as(
            "SELECT * FROM kana_text WHERE text = ANY($1)",
        )
        .bind(&kana_keys)
        .fetch_all(&ctx.pool)
        .await?;
        for kt in rows {
            // dict.lisp:517 — (push (cons table kt) (gethash (getf kt :text) substring-hash)).
            // CL `push` prepends, so each bucket is the reverse of the SQL
            // row order; `insert(0, …)` mirrors it. The order is
            // load-bearing: find-word returns the bucket in this order and
            // downstream homonym selection takes the last-iterated row.
            if let Some(FindWordRows::Kana(v)) = substring_hash.get_mut(&kt.text) {
                v.insert(0, kt);
            }
        }
    }
    if !kanji_keys.is_empty() {
        let rows: Vec<KanjiText> = sqlx::query_as(
            "SELECT * FROM kanji_text WHERE text = ANY($1)",
        )
        .bind(&kanji_keys)
        .fetch_all(&ctx.pool)
        .await?;
        for kt in rows {
            // dict.lisp:517 — prepend to mirror CL `push` (see kana loop).
            if let Some(FindWordRows::Kanji(v)) = substring_hash.get_mut(&kt.text) {
                v.insert(0, kt);
            }
        }
    }

    Ok(substring_hash)
}

#[cfg(test)]
mod tests {
    //! Per-key buckets cross-checked against the local ichiran Postgres
    //! (2026-05-25), the same DB these tests query. Each bucket is
    //! compared as a sorted `(seq, ord, common)` list: the populating
    //! query (`text = ANY(...)`) has no ORDER BY, so the bucket order is
    //! not stable — sorting both sides keeps the comparison
    //! order-independent. Test threads must be 1 — `cargo test --
    //! --test-threads=1` per the project's DB-test convention.
    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    /// A key's bucket as a sorted `(seq, ord, common)` list. Both this
    /// and the expected literal are in seq order so the unordered SQL
    /// bucket can't make the comparison flake.
    fn rows_sorted(h: &SubstringHash, key: &str) -> Vec<(i32, i32, Option<i32>)> {
        let mut out: Vec<(i32, i32, Option<i32>)> =
            match h.get(key).unwrap_or_else(|| panic!("missing key {key:?}")) {
                FindWordRows::Kana(v) => v.iter().map(|r| (r.seq, r.ord, r.common)).collect(),
                FindWordRows::Kanji(v) => v.iter().map(|r| (r.seq, r.ord, r.common)).collect(),
            };
        out.sort();
        out
    }

    fn keys_sorted(h: &SubstringHash) -> Vec<String> {
        let mut ks: Vec<String> = h.keys().cloned().collect();
        ks.sort();
        ks
    }

    fn is_kana(h: &SubstringHash, key: &str) -> bool {
        matches!(h.get(key), Some(FindWordRows::Kana(_)))
    }

    // 'こ' and 'ね' buckets are each shared by two tests — one copy here.
    fn ko_rows() -> Vec<(i32, i32, Option<i32>)> {
        vec![
            (1264740, 0, Some(0)),
            (1267110, 0, None),
            (1307770, 0, Some(1)),
            (1504770, 1, None),
            (1531190, 1, None),
            (1659920, 0, None),
            (1956240, 0, Some(28)),
            (2065150, 1, None),
            (2087990, 0, None),
            (2153770, 0, Some(0)),
            (2215030, 0, None),
            (2230390, 0, None),
            (2577750, 0, None),
            (2788170, 0, None),
            (2842951, 0, None),
            (2844354, 0, None),
        ]
    }

    fn ne_rows() -> Vec<(i32, i32, Option<i32>)> {
        vec![
            (1290020, 0, Some(5)),
            (1307780, 0, Some(0)),
            (1642760, 0, Some(15)),
            (2029080, 0, Some(0)),
            (2836242, 0, None),
            (2841117, 3, None),
            (2859162, 0, Some(0)),
            (10426293, 0, None),
        ]
    }

    #[tokio::test]
    async fn single_kanji_char_one_key() {
        // '猫' (no sticky): one kanji-classified key, two rows.
        let ctx = ctx_from_env().await;
        let h = find_substring_words(&ctx, "猫", &[]).await.unwrap();
        assert_eq!(keys_sorted(&h), vec!["猫".to_string()]);
        assert!(!is_kana(&h, "猫"), "'猫' should be kanji variant");
        assert_eq!(
            rows_sorted(&h, "猫"),
            vec![(1467640, 0, Some(7)), (2698030, 0, None)]
        );
    }

    #[tokio::test]
    async fn mixed_kana_kanji_three_keys() {
        // '猫が': が (7 kana), 猫 (2 kanji), 猫が (empty, kanji-classified
        // — the mixed string contains a kanji).
        let ctx = ctx_from_env().await;
        let h = find_substring_words(&ctx, "猫が", &[]).await.unwrap();
        assert_eq!(
            keys_sorted(&h),
            vec!["が".to_string(), "猫".to_string(), "猫が".to_string()]
        );
        assert!(is_kana(&h, "が"), "'が' should be kana variant");
        assert_eq!(
            rows_sorted(&h, "が"),
            vec![
                (1197760, 0, Some(40)),
                (1202270, 1, None),
                (2028930, 0, Some(0)),
                (2220800, 0, None),
                (2224630, 0, None),
                (2232110, 0, None),
                (2834041, 0, None),
            ]
        );
        assert!(!is_kana(&h, "猫"), "'猫' should be kanji variant");
        assert_eq!(
            rows_sorted(&h, "猫"),
            vec![(1467640, 0, Some(7)), (2698030, 0, None)]
        );
        assert!(!is_kana(&h, "猫が"), "mixed substring classified non-kana");
        assert!(rows_sorted(&h, "猫が").is_empty());
    }

    #[tokio::test]
    async fn sticky_end_blocks_substrings() {
        // '猫が' sticky=(1): every 1-char substring starts or ends at
        // pos 1, so only the length-2 key survives (empty bucket).
        let ctx = ctx_from_env().await;
        let h = find_substring_words(&ctx, "猫が", &[1]).await.unwrap();
        assert_eq!(keys_sorted(&h), vec!["猫が".to_string()]);
        assert!(rows_sorted(&h, "猫が").is_empty());
    }

    #[tokio::test]
    async fn sticky_start_and_end_block() {
        // 'ねこが' sticky=(0 3): start=0 and end=3 blocked, so only 'こ'
        // (start=1, end=2) survives.
        let ctx = ctx_from_env().await;
        let h = find_substring_words(&ctx, "ねこが", &[0, 3]).await.unwrap();
        assert_eq!(keys_sorted(&h), vec!["こ".to_string()]);
        assert_eq!(rows_sorted(&h, "こ"), ko_rows());
    }

    #[tokio::test]
    async fn sticky_interior_blocks_boundary_only() {
        // 'ねこが' sticky=(2): ね (8), こが (6), ねこが (empty). start=2
        // and end=2 are both blocked.
        let ctx = ctx_from_env().await;
        let h = find_substring_words(&ctx, "ねこが", &[2]).await.unwrap();
        assert_eq!(
            keys_sorted(&h),
            vec!["こが".to_string(), "ね".to_string(), "ねこが".to_string()]
        );
        assert_eq!(
            rows_sorted(&h, "こが"),
            vec![
                (1265180, 0, None),
                (1265190, 0, None),
                (10136364, 0, None),
                (10276500, 0, None),
                (12294787, 0, None),
                (12295833, 0, None),
            ]
        );
        assert_eq!(rows_sorted(&h, "ね"), ne_rows());
        assert!(rows_sorted(&h, "ねこが").is_empty());
    }

    #[tokio::test]
    async fn empty_string_empty_hash() {
        // REPL '' empty: n keys=0
        let ctx = ctx_from_env().await;
        let h = find_substring_words(&ctx, "", &[]).await.unwrap();
        assert!(h.is_empty());
    }

    #[tokio::test]
    async fn ascii_unknown_pre_seeds_empty_entry() {
        // 'x': one kanji-classified key, empty bucket — the pre-seeded
        // empty entry survives the no-row query.
        let ctx = ctx_from_env().await;
        let h = find_substring_words(&ctx, "x", &[]).await.unwrap();
        assert_eq!(keys_sorted(&h), vec!["x".to_string()]);
        assert!(!is_kana(&h, "x"), "'x' classified kanji (not in kana char set)");
        assert!(rows_sorted(&h, "x").is_empty());
    }

    #[tokio::test]
    async fn full_kana_three_keys() {
        // 'ねこ': こ (16), ね (8), ねこ (1).
        let ctx = ctx_from_env().await;
        let h = find_substring_words(&ctx, "ねこ", &[]).await.unwrap();
        assert_eq!(
            keys_sorted(&h),
            vec!["こ".to_string(), "ね".to_string(), "ねこ".to_string()]
        );
        assert_eq!(rows_sorted(&h, "こ"), ko_rows());
        assert_eq!(rows_sorted(&h, "ね"), ne_rows());
        assert_eq!(rows_sorted(&h, "ねこ"), vec![(1467640, 0, Some(7))]);
    }

    /// Order guard for the `insert(0, …)` prepend (dict.lisp:517 `push`):
    /// a multi-row bucket must be the *reverse* of the database's row
    /// order, not its fetch order. Compared unsorted, unlike the other
    /// tests here — that's the point. DB-agnostic: derives the expected
    /// order from the same query the populator runs, so it pins the
    /// reversal relationship rather than hard-coded seqs. '行って' has
    /// 3 kanji rows on the local DB.
    #[tokio::test]
    async fn bucket_is_reverse_of_fetch_order() {
        let ctx = ctx_from_env().await;
        let keys = vec!["行って".to_string()];
        let fetch: Vec<i32> =
            sqlx::query_scalar("SELECT seq FROM kanji_text WHERE text = ANY($1)")
                .bind(&keys)
                .fetch_all(&ctx.pool)
                .await
                .unwrap();
        assert!(fetch.len() > 1, "test needs a multi-row bucket");
        let h = find_substring_words(&ctx, "行って", &[]).await.unwrap();
        let bucket: Vec<i32> = match h.get("行って").unwrap() {
            FindWordRows::Kanji(v) => v.iter().map(|r| r.seq).collect(),
            FindWordRows::Kana(v) => v.iter().map(|r| r.seq).collect(),
        };
        let mut expected = fetch;
        expected.reverse();
        assert_eq!(bucket, expected, "bucket must be reverse of fetch order");
    }
}
