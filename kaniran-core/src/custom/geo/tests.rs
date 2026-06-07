use super::*;

// --- municipality_short ---
// REPL fixtures (.103, ichiran/custom::municipality-short), 2026-05-31.

#[test]
fn municipality_short_fixtures() {
    let cases: &[(&str, &str, &str, Option<&str>)] = &[
        ("東京都", "とうきょうと", "東京", Some("とうきょう")),
        ("北海道", "ほっかいどう", "北海道", Some("ほっかいどう")),
        ("大阪府", "おおさかふ", "大阪", Some("おおさか")),
        ("沖縄県", "おきなわけん", "沖縄", Some("おきなわ")),
        ("横浜市", "よこはまし", "横浜", Some("よこはま")),
        ("軽井沢町", "かるいざわまち", "軽井沢", Some("かるいざわ")),
        ("葉山町", "はやまちょう", "葉山", Some("はやま")),
        ("葉山町", "はやままち", "葉山", Some("はやま")),
        ("白川村", "しらかわむら", "白川", Some("しらかわ")),
        ("白川村", "しらかわそん", "白川", Some("しらかわ")),
        ("中央区", "ちゅうおうく", "中央", Some("ちゅうおう")),
        ("京都市", "きょうとし", "京都", Some("きょうと")),
        ("横浜市", "ﾖｺﾊﾏｼ", "横浜", Some("ﾖｺﾊﾏ")),
        ("横浜市", "よこはま", "横浜", None),
        ("道", "どう", "道", Some("どう")),
        ("市町村", "しちょうそん", "市町", Some("しちょう")),
    ];
    for (text, reading, expect_text, expect_reading) in cases {
        let (got_text, got_reading) = municipality_short(text, reading);
        assert_eq!(
            &got_text, expect_text,
            "text input={text:?} reading={reading:?}"
        );
        assert_eq!(
            got_reading.as_deref(),
            *expect_reading,
            "reading input={text:?} reading={reading:?}",
        );
    }
}

// --- romanize_municipality ---
// REPL fixtures (.103, ichiran/custom::romanize-municipality), 2026-05-31.

#[test]
fn romanize_municipality_fixtures() {
    let cases: &[(&str, &str, Option<bool>, &str)] = &[
        ("東京都", "とうきょうと", None, "Tokyo Metropolis"),
        ("東京都", "とうきょうと", Some(true), "Tokyo Metropolis"),
        ("東京都", "とうきょうと", Some(false), "Tokyo"),
        ("北海道", "ほっかいどう", None, "Hokkaido"),
        ("大阪府", "おおさかふ", None, "Osaka Prefecture"),
        ("沖縄県", "おきなわけん", None, "Okinawa Prefecture"),
        ("横浜市", "よこはまし", None, "Yokohama (city)"),
        ("鎌倉市", "かまくらし", None, "Kamakura (city)"),
        ("軽井沢町", "かるいざわまち", None, "Karuizawa (town)"),
        ("白川村", "しらかわむら", None, "Shirakawa (village)"),
        ("中央区", "ちゅうおうく", None, "Chuo Ward"),
        ("京都市", "きょうとし", Some(false), "Kyoto"),
        ("横浜市", "よこはま", None, " (city)"),
    ];
    for (text, reading, include_type, expected) in cases {
        let got = romanize_municipality(text, reading, *include_type);
        assert_eq!(
            &got, expected,
            "input text={text:?} reading={reading:?} include_type={include_type:?}",
        );
    }
}

// --- normalize_geo ---
// REPL fixtures (.103, ichiran/custom::normalize-geo), 2026-05-31.

#[test]
fn normalize_geo_fixtures() {
    let cases: &[(&str, &str)] = &[
        ("Tokyo", "tokyo"),
        ("Tōkyō", "tokyo"),
        ("Ōsaka", "osaka"),
        ("Hokkaido", "hokkaido"),
        ("HŪNŌ", "huno"),
        ("", ""),
        ("TŌKYŌ-DOU", "tokyo-dou"),
        ("Kyōto", "kyoto"),
        ("Shin'Ōsaka", "shin'osaka"),
    ];
    for (input, expected) in cases {
        assert_eq!(&normalize_geo(input), expected, "input={input}");
    }
}
