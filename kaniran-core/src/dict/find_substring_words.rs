//! Port of `ichiran/dict:find-substring-words` (`dict.lisp:501`).
//!
//! Builds the per-call-tree lookup cache that
//! [`crate::dict::find_word::find_word`] short-circuits against —
//! enumerates every length-bounded substring of `str` whose `(start,
//! end)` pair is not blocked by `sticky` (positions that cannot serve
//! as a word boundary), then bulk-fetches the matching `kana_text` /
//! `kanji_text` rows from the database, bucketed by substring text.
//!
//! Substrings absent from the database still get a hash entry whose
//! value is an empty `FindWordRows` variant — that empty entry is the
//! signal `find-word` uses to know the lookup already ran and the
//! result is "no rows", short-circuiting the per-substring SQL query.
//!
//! Diverges from the upstream lambda list `(str &key sticky)` by:
//!
//! - taking `&KaniranContext` for the database handle, replacing the
//!   upstream dynamic `*connection*` per
//!   [`crate::conn::kani_context`];
//! - taking `sticky` as a borrowed `&[usize]` rather than the upstream
//!   `&key` keyword that defaults to `nil`. The data is a list of
//!   character positions; the default-empty case lifts to `&[]` at the
//!   call site (CONVENTIONS §4.4 — multi-valued keyword takes its
//!   natural type).
//!
//! Returns the populated [`SubstringHash`] by value so the caller can
//! wrap it in an [`Arc`](std::sync::Arc) and rebind the context via
//! [`crate::conn::kani_context::KaniranContext::with_substring_hash`]
//! before invoking the nested-find loop.

use crate::characters::char_class_type::CharClass;
use crate::characters::test_word::test_word;
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
            // dict.lisp:518 — (push (cons table kt) (gethash (getf kt :text) substring-hash))
            // The per-key variant was pre-seeded as `Kana(vec)` above
            // (test-word classified this substring as kana). Append
            // rather than prepend: SBCL's `push` reverses SQL row
            // order, but the SQL is unordered so the eventual
            // ordering is non-deterministic on both sides.
            if let Some(FindWordRows::Kana(v)) = substring_hash.get_mut(&kt.text) {
                v.push(kt);
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
            if let Some(FindWordRows::Kanji(v)) = substring_hash.get_mut(&kt.text) {
                v.push(kt);
            }
        }
    }

    Ok(substring_hash)
}

#[cfg(test)]
mod tests {
    //! All counts cross-checked against the .103 SBCL via
    //! `/tmp/probe_fsw.lisp` and `/tmp/probe_fsw2.lisp` (2026-05-17
    //! runs). Test threads must be 1 — `cargo test --test-threads=1`
    //! per the project's DB-test convention.
    use super::*;

    async fn ctx_from_env() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env() — DATABASE_URL / kaniran.toml required")
    }

    fn lens(h: &SubstringHash) -> Vec<(String, usize)> {
        let mut out: Vec<(String, usize)> = h
            .iter()
            .map(|(k, v)| {
                let n = match v {
                    FindWordRows::Kana(v) => v.len(),
                    FindWordRows::Kanji(v) => v.len(),
                };
                (k.clone(), n)
            })
            .collect();
        out.sort();
        out
    }

    #[tokio::test]
    async fn single_kanji_char_one_key() {
        // REPL '猫' (no sticky): n keys=1, '猫' -> 2 items (KANJI-TEXT)
        let ctx = ctx_from_env().await;
        let h = find_substring_words(&ctx, "猫", &[]).await.unwrap();
        assert_eq!(h.len(), 1);
        let v = h.get("猫").expect("missing key");
        match v {
            FindWordRows::Kanji(rows) => assert_eq!(rows.len(), 2),
            FindWordRows::Kana(_) => panic!("expected kanji variant"),
        }
    }

    #[tokio::test]
    async fn mixed_kana_kanji_three_keys() {
        // REPL '猫が' (no sticky):
        //   'が' -> 7 KANA-TEXT
        //   '猫'  -> 2 KANJI-TEXT
        //   '猫が' -> 0 (no DB row), classified kanji (mixed string)
        let ctx = ctx_from_env().await;
        let h = find_substring_words(&ctx, "猫が", &[]).await.unwrap();
        assert_eq!(
            lens(&h),
            vec![
                ("が".to_string(), 7),
                ("猫".to_string(), 2),
                ("猫が".to_string(), 0),
            ]
        );
        // Variant tags are pre-seeded by `test_word`.
        match h.get("が").unwrap() {
            FindWordRows::Kana(_) => {}
            _ => panic!("'が' should be kana variant"),
        }
        match h.get("猫").unwrap() {
            FindWordRows::Kanji(_) => {}
            _ => panic!("'猫' should be kanji variant"),
        }
        match h.get("猫が").unwrap() {
            FindWordRows::Kanji(_) => {}
            _ => panic!("mixed substring classified non-kana"),
        }
    }

    #[tokio::test]
    async fn sticky_end_blocks_substrings() {
        // REPL '猫が' (sticky=(1)): only key is '猫が' (length 2)
        // because all 1-char substrings either start or end at pos 1.
        let ctx = ctx_from_env().await;
        let h = find_substring_words(&ctx, "猫が", &[1]).await.unwrap();
        assert_eq!(lens(&h), vec![("猫が".to_string(), 0)]);
    }

    #[tokio::test]
    async fn sticky_start_and_end_block() {
        // REPL 'ねこが' (sticky=(0 3)): only 'こ' survives.
        //   start=0 blocked, start=1 + end=2 -> 'こ' (end=3 sticky).
        let ctx = ctx_from_env().await;
        let h = find_substring_words(&ctx, "ねこが", &[0, 3]).await.unwrap();
        assert_eq!(lens(&h), vec![("こ".to_string(), 16)]);
    }

    #[tokio::test]
    async fn sticky_interior_blocks_boundary_only() {
        // REPL 'ねこが' (sticky=(2)): 3 keys
        //   start=0,end=1 'ね' (kana, 8); start=0,end=3 'ねこが' (0);
        //   start=1,end=3 'こが' (6, kana). start=2 blocked.
        let ctx = ctx_from_env().await;
        let h = find_substring_words(&ctx, "ねこが", &[2]).await.unwrap();
        assert_eq!(
            lens(&h),
            vec![
                ("こが".to_string(), 6),
                ("ね".to_string(), 8),
                ("ねこが".to_string(), 0),
            ]
        );
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
        // REPL 'x': 1 key 'x' -> 0 items. Confirms pre-seeded
        // empty-variant entry survives the no-row query.
        let ctx = ctx_from_env().await;
        let h = find_substring_words(&ctx, "x", &[]).await.unwrap();
        assert_eq!(h.len(), 1);
        match h.get("x").unwrap() {
            FindWordRows::Kanji(v) => assert!(v.is_empty()),
            FindWordRows::Kana(_) => panic!("'x' classified kanji (not in kana char set)"),
        }
    }

    #[tokio::test]
    async fn full_kana_three_keys() {
        // REPL 'ねこ' (no sticky):
        //   'こ' -> 16, 'ね' -> 8, 'ねこ' -> 1
        let ctx = ctx_from_env().await;
        let h = find_substring_words(&ctx, "ねこ", &[]).await.unwrap();
        assert_eq!(
            lens(&h),
            vec![
                ("こ".to_string(), 16),
                ("ね".to_string(), 8),
                ("ねこ".to_string(), 1),
            ]
        );
    }
}
