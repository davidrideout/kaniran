//! Transliteration of `ichiran/dict:find-word-with-pos` (`dict-grammar.lisp:89`).
//!
//! Returns kana_text or kanji_text rows whose `text = word` and whose
//! seq is the seq of a sense flagged with one of the given `pos` tags
//! in `sense_prop`. The table is chosen by
//! [`crate::characters::test_word`] against [`CharClass::Kana`] — kana
//! inputs hit `kana_text`, anything else hits `kanji_text`. The
//! polymorphic Lisp return — a list of `kana-text` xor `kanji-text`
//! rows depending on the table chosen — is wrapped in a 2-variant enum
//! per CONVENTIONS §4.3. Same shape as
//! [`super::find_word_seq::WordSeqRows`] / [`super::find_word::FindWordRows`]
//! but kept as its own type because the three functions have distinct
//! semantics, and each type stays local to the function that owns it
//! per §1.
//!
//! Diverges from the upstream lambda list `(word &rest posi)` by:
//! - taking `&KaniranContext` for the database handle, replacing the
//!   upstream dynamic `*connection*` per
//!   [`crate::conn::kani_context`];
//! - taking `posi` as `&[&str]` instead of `&rest` packed positional —
//!   Rust has no `&rest` keyword.
//!
//! Empty `posi` returns the matching enum variant with an empty `Vec`.
//! Postgres' `sp.text = ANY($2)` against an empty array yields no rows,
//! mirroring the Lisp behavior where `(:in 'sp.text (:set))` filters
//! everything out.

use crate::characters::char_class_type::CharClass;
use crate::characters::test_word::test_word;
use crate::conn::kani_context::KaniranContext;
use crate::dict::kana_text_dao::KanaText;
use crate::dict::kanji_text_dao::KanjiText;

#[derive(Debug, Clone)]
pub enum WordWithPosRows {
    Kana(Vec<KanaText>),
    Kanji(Vec<KanjiText>),
}

pub async fn find_word_with_pos(
    ctx: &KaniranContext,
    word: &str,
    posi: &[&str],
) -> Result<WordWithPosRows, sqlx::Error> {
    // s-sql `:in 'sp.text (:set posi)` expands to multiple `?` binds;
    // Postgres' `sp.text = ANY($2)` is the array-bound equivalent. The
    // sqlx Encode impl for `&[&str]` over Postgres requires owned
    // String elements, so allocate a Vec<String> for the bind (see
    // dict/get_conj_data.rs:67 for the same pattern).
    let posi_owned: Vec<String> = posi.iter().map(|s| (*s).to_string()).collect();
    if test_word(word, CharClass::Kana) {
        let rows = sqlx::query_as::<_, KanaText>(
            "SELECT DISTINCT kt.* FROM kana_text kt \
             INNER JOIN sense_prop sp ON sp.seq = kt.seq AND sp.tag = 'pos' \
             WHERE kt.text = $1 AND sp.text = ANY($2)",
        )
        .bind(word)
        .bind(&posi_owned)
        .fetch_all(&ctx.pool)
        .await?;
        Ok(WordWithPosRows::Kana(rows))
    } else {
        let rows = sqlx::query_as::<_, KanjiText>(
            "SELECT DISTINCT kt.* FROM kanji_text kt \
             INNER JOIN sense_prop sp ON sp.seq = kt.seq AND sp.tag = 'pos' \
             WHERE kt.text = $1 AND sp.text = ANY($2)",
        )
        .bind(word)
        .bind(&posi_owned)
        .fetch_all(&ctx.pool)
        .await?;
        Ok(WordWithPosRows::Kanji(rows))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Kanji input → `kanji_text` dispatch with a single matching row.
    /// REPL: `(find-word-with-pos "区別" "vs")` → 1 KANJI-TEXT row
    /// id=13731, seq=1244250, common=10, best_kana=くべつ.
    #[tokio::test]
    async fn kanji_single_match() {
        let ctx = KaniranContext::from_env().await.unwrap();
        let rows = find_word_with_pos(&ctx, "区別", &["vs"]).await.unwrap();
        let kanji = match rows {
            WordWithPosRows::Kanji(v) => v,
            WordWithPosRows::Kana(_) => panic!("expected Kanji variant"),
        };
        assert_eq!(kanji.len(), 1);
        let row = &kanji[0];
        assert_eq!(row.id, 13731);
        assert_eq!(row.seq, 1244250);
        assert_eq!(row.text, "区別");
        assert_eq!(row.ord, 0);
        assert_eq!(row.common, Some(10));
        assert_eq!(row.common_tags, "[ichi1][news1][nf10]");
        assert!(row.conjugate_p);
        assert!(!row.nokanji);
        assert_eq!(row.best_kana.as_deref(), Some("くべつ"));
    }

    /// Pure-katakana input → `test_word :kana` true → `kana_text`
    /// dispatch. REPL: `(find-word-with-pos "ジョギング" "vs")` →
    /// 1 KANA-TEXT row id=9654, seq=1066360, best_kanji = :NULL (the
    /// Lisp `:NULL` sentinel maps to Rust `None`).
    #[tokio::test]
    async fn kana_single_match() {
        let ctx = KaniranContext::from_env().await.unwrap();
        let rows = find_word_with_pos(&ctx, "ジョギング", &["vs"]).await.unwrap();
        let kana = match rows {
            WordWithPosRows::Kana(v) => v,
            WordWithPosRows::Kanji(_) => panic!("expected Kana variant"),
        };
        assert_eq!(kana.len(), 1);
        let row = &kana[0];
        assert_eq!(row.id, 9654);
        assert_eq!(row.seq, 1066360);
        assert_eq!(row.text, "ジョギング");
        assert_eq!(row.ord, 0);
        assert_eq!(row.common, Some(0));
        assert_eq!(row.common_tags, "[gai1][ichi1]");
        assert!(row.conjugate_p);
        assert!(!row.nokanji);
        assert_eq!(row.best_kanji, None);
    }

    /// Kanji word with no matching pos → empty `Kanji` result. REPL:
    /// `(find-word-with-pos "青空" "vs")` → 0 rows.
    #[tokio::test]
    async fn kanji_no_match() {
        let ctx = KaniranContext::from_env().await.unwrap();
        let rows = find_word_with_pos(&ctx, "青空", &["vs"]).await.unwrap();
        match rows {
            WordWithPosRows::Kanji(v) => assert!(v.is_empty()),
            WordWithPosRows::Kana(_) => panic!("expected Kanji variant"),
        }
    }

    /// `adj-i` pos tag. REPL: `(find-word-with-pos "赤い" "adj-i")` →
    /// 1 KANJI-TEXT row id=31416, seq=1383240.
    #[tokio::test]
    async fn kanji_adj_i_match() {
        let ctx = KaniranContext::from_env().await.unwrap();
        let rows = find_word_with_pos(&ctx, "赤い", &["adj-i"]).await.unwrap();
        let kanji = match rows {
            WordWithPosRows::Kanji(v) => v,
            WordWithPosRows::Kana(_) => panic!("expected Kanji variant"),
        };
        assert_eq!(kanji.len(), 1);
        assert_eq!(kanji[0].id, 31416);
        assert_eq!(kanji[0].seq, 1383240);
        assert_eq!(kanji[0].common, Some(15));
        assert_eq!(kanji[0].best_kana.as_deref(), Some("あかい"));
    }

    /// `adj-na` pos tag. REPL: `(find-word-with-pos "好き" "adj-na")` →
    /// 1 KANJI-TEXT row id=17991, seq=1277450.
    #[tokio::test]
    async fn kanji_adj_na_match() {
        let ctx = KaniranContext::from_env().await.unwrap();
        let rows = find_word_with_pos(&ctx, "好き", &["adj-na"]).await.unwrap();
        let kanji = match rows {
            WordWithPosRows::Kanji(v) => v,
            WordWithPosRows::Kana(_) => panic!("expected Kanji variant"),
        };
        assert_eq!(kanji.len(), 1);
        assert_eq!(kanji[0].id, 17991);
        assert_eq!(kanji[0].seq, 1277450);
        assert_eq!(kanji[0].common, Some(0));
        assert_eq!(kanji[0].best_kana.as_deref(), Some("すき"));
    }

    /// `pn` (pronoun) tag with a polysemous word → many rows. REPL:
    /// `(find-word-with-pos "私" "pn")` → 13 KANJI-TEXT rows. Pinned
    /// `(seq, id)` set captured from the REPL; row order is unspecified
    /// by the SQL (no ORDER BY upstream), so sort before comparison.
    #[tokio::test]
    async fn kanji_pn_thirteen_rows() {
        let ctx = KaniranContext::from_env().await.unwrap();
        let rows = find_word_with_pos(&ctx, "私", &["pn"]).await.unwrap();
        let kanji = match rows {
            WordWithPosRows::Kanji(v) => v,
            WordWithPosRows::Kana(_) => panic!("expected Kanji variant"),
        };
        assert_eq!(kanji.len(), 13);
        let mut got: Vec<(i32, i32)> = kanji.iter().map(|r| (r.seq, r.id)).collect();
        got.sort();
        let expected: Vec<(i32, i32)> = vec![
            (1311110, 22264),
            (1311125, 22265),
            (1347580, 26861),
            (2015370, 108229),
            (2079310, 114743),
            (2217330, 129111),
            (2217340, 129112),
            (2842390, 197077),
            (2845454, 199954),
            (2858221, 211749),
            (2858384, 211905),
            (2858397, 211916),
            (2864027, 217322),
        ];
        assert_eq!(got, expected);
        for row in &kanji {
            assert_eq!(row.text, "私");
        }
    }

    /// ASCII input → not all kana → `kanji_text` dispatch, 0 rows.
    /// REPL: `(find-word-with-pos "nonsense" "vs")` → 0 rows.
    #[tokio::test]
    async fn ascii_kanji_no_match() {
        let ctx = KaniranContext::from_env().await.unwrap();
        let rows = find_word_with_pos(&ctx, "nonsense", &["vs"]).await.unwrap();
        match rows {
            WordWithPosRows::Kanji(v) => assert!(v.is_empty()),
            WordWithPosRows::Kana(_) => panic!("expected Kanji variant"),
        }
    }

    /// Multiple posi (exercise the `&rest` arity). REPL:
    /// `(find-word-with-pos "食べる" "v1" "vs")` → 1 KANJI-TEXT row
    /// id=28271, seq=1358280 (matches the `v1` pos).
    #[tokio::test]
    async fn multi_pos_match() {
        let ctx = KaniranContext::from_env().await.unwrap();
        let rows = find_word_with_pos(&ctx, "食べる", &["v1", "vs"]).await.unwrap();
        let kanji = match rows {
            WordWithPosRows::Kanji(v) => v,
            WordWithPosRows::Kana(_) => panic!("expected Kanji variant"),
        };
        assert_eq!(kanji.len(), 1);
        assert_eq!(kanji[0].id, 28271);
        assert_eq!(kanji[0].seq, 1358280);
        assert_eq!(kanji[0].common, Some(25));
        assert_eq!(kanji[0].best_kana.as_deref(), Some("たべる"));
    }

    /// Kana word with multiple posi → `kana_text` dispatch, single row.
    /// REPL: `(find-word-with-pos "する" "vs-i" "vs-s")` →
    /// 1 KANA-TEXT row id=22268, seq=1157170.
    #[tokio::test]
    async fn kana_multi_pos_match() {
        let ctx = KaniranContext::from_env().await.unwrap();
        let rows = find_word_with_pos(&ctx, "する", &["vs-i", "vs-s"]).await.unwrap();
        let kana = match rows {
            WordWithPosRows::Kana(v) => v,
            WordWithPosRows::Kanji(_) => panic!("expected Kana variant"),
        };
        assert_eq!(kana.len(), 1);
        assert_eq!(kana[0].id, 22268);
        assert_eq!(kana[0].seq, 1157170);
        assert_eq!(kana[0].common, Some(0));
        assert_eq!(kana[0].best_kanji.as_deref(), Some("為る"));
    }

    /// Polysemous kana word with three posi — exercises both the
    /// multi-posi `ANY` and the multi-row `SELECT DISTINCT` paths.
    /// REPL: `(find-word-with-pos "そう" "adv" "n" "aux-v")` → 26
    /// KANA-TEXT rows. Pinned `(seq, id)` set; sort before comparison.
    #[tokio::test]
    async fn kana_three_pos_twentysix_rows() {
        let ctx = KaniranContext::from_env().await.unwrap();
        let rows = find_word_with_pos(&ctx, "そう", &["adv", "n", "aux-v"]).await.unwrap();
        let kana = match rows {
            WordWithPosRows::Kana(v) => v,
            WordWithPosRows::Kanji(_) => panic!("expected Kana variant"),
        };
        assert_eq!(kana.len(), 26);
        let mut got: Vec<(i32, i32)> = kana.iter().map(|r| (r.seq, r.id)).collect();
        got.sort();
        let expected: Vec<(i32, i32)> = vec![
            (1241450, 30916),
            (1398030, 47020),
            (1398670, 47082),
            (1399250, 47140),
            (1399540, 47168),
            (1399590, 47172),
            (1399990, 47213),
            (1400810, 47298),
            (2027990, 110259),
            (2033880, 110867),
            (2137720, 122367),
            (2249280, 136151),
            (2253390, 136639),
            (2406720, 153533),
            (2414580, 154361),
            (2414600, 154363),
            (2639080, 181268),
            (2681340, 185752),
            (2843362, 222959),
            (2843365, 222962),
            (2843386, 222983),
            (2843387, 222984),
            (2843388, 222985),
            (2843390, 222987),
            (2843391, 222988),
            (2844287, 224036),
        ];
        assert_eq!(got, expected);
    }
}
