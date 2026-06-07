//! Port of `ichiran:romanize` (`romanize.lisp:257-271`).
//!
//! Romanizes `input` and joins the parts into one string; with `with_info`
//! also returns each romanization paired with its word-info string.

use super::join_parts::join_parts;
use super::kani_romanize_method::KaniRomanizeMethod;
use super::romanize_word_info::romanize_word_info;
use crate::characters::char_class::{basic_split, SegmentKind};
use crate::characters::kana::normalize;
use crate::characters::kana::NormalizationContext;
use crate::conn::kani_context::KaniranContext;
use crate::dict::simple_segment::simple_segment;
use crate::dict::word_info_str::word_info_str;

pub async fn romanize(
    ctx: &KaniranContext,
    input: &str,
    method: KaniRomanizeMethod<'_>,
    with_info: bool,
) -> Result<(String, Vec<(String, String)>), sqlx::Error> {
    // (normalize input :context method) — characters.lisp:230 tests (eql context :kana)
    let context = match method {
        KaniRomanizeMethod::Kana => NormalizationContext::Kana,
        KaniRomanizeMethod::Method(_) => NormalizationContext::Default,
    };
    let input = normalize(input, context);
    let mut definitions: Vec<(String, String)> = Vec::new();
    let mut parts: Vec<String> = Vec::new();
    for (split_type, split_text) in basic_split(&input) {
        if split_type == SegmentKind::Word {
            for word in simple_segment(ctx, &split_text, None).await? {
                let rom = romanize_word_info(&word, method);
                if with_info {
                    definitions.push((rom.clone(), word_info_str(ctx, &word).await?));
                }
                parts.push(rom);
            }
        } else {
            parts.push(split_text);
        }
    }
    // (nreverse definitions) — push-to-back already collects in encounter order
    Ok((join_parts(&parts), definitions))
}

#[cfg(test)]
mod tests {
    //! REPL fixtures (.103, `ichiran:romanize`, 2026-05-24). Run with
    //! `cargo test ... -- --test-threads=1` per the DB-test convention.
    use super::*;
    use crate::core::_star_hepburn_traditional_star_::hepburn_traditional;
    use crate::core::generic_romanization_class::RomanizationMethod;

    async fn ctx() -> std::sync::Arc<KaniranContext> {
        KaniranContext::from_env()
            .await
            .expect("KaniranContext::from_env — DATABASE_URL / kaniran.toml required")
    }

    fn traditional() -> KaniRomanizeMethod<'static> {
        KaniRomanizeMethod::Method(RomanizationMethod::TraditionalHepburn(hepburn_traditional()))
    }

    #[tokio::test]
    async fn romanize_joined_string() {
        // Each row: (input, default-method joined string, :kana joined string).
        // The default-method string converts 。/！ to ". "/"! " (punctuation
        // marks); the :kana string keeps full-width punctuation because
        // normalize's :kana context omits *punctuation-marks*.
        let ctx = ctx().await;
        let cases: &[(&str, &str, &str)] = &[
            (
                "富士山は日本で最も高い山である。",
                "fujisan wa nihon de mottomo takai yama de aru. ",
                "ふじさん は にほん で もっとも たかい やま で ある。",
            ),
            (
                "2020年に東京オリンピックが開催された。",
                "nisen nijūnen ni tōkyō orimpikku ga kaisai sareta. ",
                "にせんにじゅうねん に とうきょう オリンピック が かいさい された。",
            ),
            (
                "彼女は新しい仮説を提唱した。",
                "kanojo wa atarashii kasetsu wo teishō shita. ",
                "かのじょ は あたらしい かせつ を ていしょう した。",
            ),
            (
                "ABCは試験的な略語です。",
                "ABC wa shikenteki na ryakugo desu. ",
                "ABC は しけんてき な りゃくご です。",
            ),
            ("Hello 世界！", "Hello sekai! ", "Hello せかい！"),
        ];
        for (input, expected_default, expected_kana) in cases {
            let (default_str, _) = romanize(&ctx, input, traditional(), false).await.unwrap();
            assert_eq!(&default_str, expected_default, "default method, input={input:?}");
            let (kana_str, _) = romanize(&ctx, input, KaniRomanizeMethod::Kana, false)
                .await
                .unwrap();
            assert_eq!(&kana_str, expected_kana, ":kana method, input={input:?}");
        }
    }

    #[tokio::test]
    async fn romanize_with_info_collects_definitions_in_order() {
        // REPL `(romanize "富士山は日本で最も高い山である。" :with-info t)`:
        // 9 definitions, one per romanized part (the trailing ". " misc split
        // contributes no definition), paired with the word-info-str headword.
        let ctx = ctx().await;
        let (joined, definitions) = romanize(
            &ctx,
            "富士山は日本で最も高い山である。",
            traditional(),
            true,
        )
        .await
        .unwrap();
        assert_eq!(joined, "fujisan wa nihon de mottomo takai yama de aru. ");
        let roms: Vec<&str> = definitions.iter().map(|(rom, _)| rom.as_str()).collect();
        assert_eq!(
            roms,
            vec!["fujisan", "wa", "nihon", "de", "mottomo", "takai", "yama", "de", "aru"]
        );
        // word-info-str headwords pair with the rom in encounter order.
        assert!(definitions[0].1.starts_with("富士山"), "got {:?}", definitions[0].1);
        assert!(definitions[2].1.starts_with("日本"), "got {:?}", definitions[2].1);
        assert!(definitions[6].1.starts_with("山"), "got {:?}", definitions[6].1);
    }

    #[tokio::test]
    async fn romanize_with_info_false_yields_empty_definitions() {
        // with-info nil: the joined string is unchanged and no definitions
        // are collected.
        let ctx = ctx().await;
        let (joined, definitions) =
            romanize(&ctx, "彼女は新しい仮説を提唱した。", traditional(), false)
                .await
                .unwrap();
        assert_eq!(joined, "kanojo wa atarashii kasetsu wo teishō shita. ");
        assert!(definitions.is_empty());
    }
}
