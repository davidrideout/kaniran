use super::super::matching::match_readings;
use super::*;
use std::sync::Arc;

// --- to_json ---
fn to_json_ctx() -> Arc<KaniranContext> {
    KaniranContext::from_env()
        
        .expect("DATABASE_URL / kaniran.toml required")
}

#[cfg(feature = "postgres")]
fn kanji(to_json_ctx: &KaniranContext, text: &str) -> Kanji {
    tokio::runtime::Runtime::new()
        .expect("tokio runtime")
        .block_on(
            sqlx::query_as::<_, Kanji>("SELECT * FROM kanji WHERE text = $1")
                .bind(text)
                .fetch_one(to_json_ctx.pool.as_ref().expect("postgres pool")),
        )
        .expect("kanji row exists")
}

/// Serializes a kanji to JSON. A missing freq/grade column always emits as
/// JSON null: 人 (both present), 薔 (grade null), 鬱 (freq null), 檸 (both
/// null). Also covers `irr_perc` with total=0 → `--.--%` (薔, 檸) vs nonzero
/// (人, 鬱); readings ordered by type then sample; suffix readings (人's り/と);
/// and kun readings carrying okurigana (鬱).
#[cfg(feature = "postgres")]
#[test]
fn to_json_fixtures() {
    let to_json_ctx = to_json_ctx();
    let cases: &[(&str, &str)] = &[
        (
            "人",
            r#"{"text":"人","rc":9,"rn":9,"strokes":2,"total":345,"irr":5,"irr_perc":"1.45%","readings":[{"text":"じん","rtext":"jin","type":"ja_on","okuri":[],"sample":174,"perc":"50.43%"},{"text":"にん","rtext":"nin","type":"ja_on","okuri":[],"sample":96,"perc":"27.83%"},{"text":"ひと","rtext":"hito","type":"ja_kun","okuri":[],"sample":47,"perc":"13.62%"},{"text":"り","rtext":"ri","type":"ja_kun","okuri":[],"sample":15,"perc":"4.35%","suffixp":true},{"text":"と","rtext":"to","type":"ja_kun","okuri":[],"sample":8,"perc":"2.32%","suffixp":true}],"meanings":["person"],"freq":5,"grade":1}"#,
        ),
        (
            "薔",
            r#"{"text":"薔","rc":140,"rn":140,"strokes":16,"total":0,"irr":0,"irr_perc":"--.--%","readings":[{"text":"ば","rtext":"ba","type":"ja_on","okuri":[],"sample":0,"perc":"--.--%"},{"text":"しょう","rtext":"shou","type":"ja_on","okuri":[],"sample":0,"perc":"--.--%"},{"text":"しょく","rtext":"shoku","type":"ja_on","okuri":[],"sample":0,"perc":"--.--%"},{"text":"そう","rtext":"sou","type":"ja_on","okuri":[],"sample":0,"perc":"--.--%"},{"text":"みずたで","rtext":"mizutade","type":"ja_kun","okuri":[],"sample":0,"perc":"--.--%"}],"meanings":["a kind of grass"],"freq":2356,"grade":null}"#,
        ),
        (
            "鬱",
            r#"{"text":"鬱","rc":192,"rn":75,"strokes":29,"total":3,"irr":0,"irr_perc":"0.00%","readings":[{"text":"うつ","rtext":"utsu","type":"ja_on","okuri":[],"sample":2,"perc":"66.67%"},{"text":"うっ","rtext":"u","type":"ja_kun","okuri":["する"],"sample":1,"perc":"33.33%"},{"text":"ふさ","rtext":"fusa","type":"ja_kun","okuri":["ぐ"],"sample":0,"perc":"0.00%"},{"text":"しげ","rtext":"shige","type":"ja_kun","okuri":["る"],"sample":0,"perc":"0.00%"}],"meanings":["gloom","depression","melancholy","luxuriant"],"freq":null,"grade":8}"#,
        ),
        (
            "檸",
            r#"{"text":"檸","rc":75,"rn":75,"strokes":18,"total":0,"irr":0,"irr_perc":"--.--%","readings":[{"text":"ねい","rtext":"nei","type":"ja_on","okuri":[],"sample":0,"perc":"--.--%"},{"text":"どう","rtext":"dou","type":"ja_on","okuri":[],"sample":0,"perc":"--.--%"}],"meanings":["lemon tree"],"freq":null,"grade":null}"#,
        ),
    ];
    for (text, expected) in cases {
        let kanji = kanji(&to_json_ctx, text);
        let js = to_json(&to_json_ctx, &kanji).unwrap();
        let actual = serde_json::to_string(&js).unwrap();
        assert_eq!(actual.as_str(), *expected, "text={text}");
    }
}

// --- reading_info_json ---
fn reading_info_json_ctx() -> Arc<KaniranContext> {
    KaniranContext::from_env()
        
        .expect("DATABASE_URL / kaniran.toml required")
}

#[cfg(feature = "postgres")]
fn reading(reading_info_json_ctx: &KaniranContext, id: i32) -> Reading {
    tokio::runtime::Runtime::new()
        .expect("tokio runtime")
        .block_on(
            sqlx::query_as::<_, Reading>("SELECT * FROM reading WHERE id = $1")
                .bind(id)
                .fetch_one(reading_info_json_ctx.pool.as_ref().expect("postgres pool")),
        )
        .expect("reading row exists")
}

/// Serializes a single reading to JSON. Covers: multi-element okurigana array
/// (3329 む, 8 entries) vs single (5014 で, 397 び) vs empty (575 え, 315 いち);
/// prefix and suffix flags in every combination; the percentage with total=0 →
/// `--.--%` (575), sample=0 with total>0 → `0.00%` (575), and nonzero (315 →
/// `100.00%`); and romanization keeping long vowels (もう→`mou`, not `mō`).
#[cfg(feature = "postgres")]
#[test]
fn reading_info_json_fixtures() {
    let reading_info_json_ctx = reading_info_json_ctx();
    let cases: &[(i32, i32, &str)] = &[
        (
            3329,
            345,
            r#"{"text":"む","rtext":"mu","type":"ja_kun","okuri":["かう","い","ける","き","け","く","かい","こう"],"sample":37,"perc":"10.72%","prefixp":true,"suffixp":true}"#,
        ),
        (
            5014,
            200,
            r#"{"text":"で","rtext":"de","type":"ja_kun","okuri":["る"],"sample":78,"perc":"39.00%","suffixp":true}"#,
        ),
        (
            5702,
            100,
            r#"{"text":"もう","rtext":"mou","type":"ja_kun","okuri":["し","す"],"sample":22,"perc":"22.00%","prefixp":true}"#,
        ),
        (
            397,
            50,
            r#"{"text":"び","rtext":"bi","type":"ja_kun","okuri":["き"],"sample":15,"perc":"30.00%","suffixp":true}"#,
        ),
        (
            575,
            0,
            r#"{"text":"え","rtext":"e","type":"ja_na","okuri":[],"sample":0,"perc":"--.--%"}"#,
        ),
        (
            575,
            120,
            r#"{"text":"え","rtext":"e","type":"ja_na","okuri":[],"sample":0,"perc":"0.00%"}"#,
        ),
        (
            315,
            314,
            r#"{"text":"いち","rtext":"ichi","type":"ja_on","okuri":[],"sample":314,"perc":"100.00%"}"#,
        ),
    ];
    for (id, total, expected) in cases {
        let reading = reading(&reading_info_json_ctx, *id);
        let js = reading_info_json(&reading_info_json_ctx, &reading, *total)
            
            .unwrap();
        let actual = serde_json::to_string(&js).unwrap();
        assert_eq!(actual.as_str(), *expected, "id={id} total={total}");
    }
}

// --- kanji_info_json ---
fn kanji_info_json_ctx() -> Arc<KaniranContext> {
    KaniranContext::from_env()
        
        .expect("DATABASE_URL / kaniran.toml required")
}

/// Looks up a kanji by character and serializes it to JSON. Covers a present
/// character (氷 — freq/grade set, mixed reading types, a kun reading with
/// okurigana), another present character (光), and a non-kanji argument
/// (`"z"`) returning `None`.
#[test]
fn kanji_info_json_fixtures() {
    let kanji_info_json_ctx = kanji_info_json_ctx();
    assert_eq!(
        serde_json::to_string(
            &kanji_info_json(&kanji_info_json_ctx, "氷")
                
                .unwrap()
                .unwrap()
        )
        .unwrap(),
        r#"{"text":"氷","rc":85,"rn":3,"strokes":5,"total":14,"irr":1,"irr_perc":"7.14%","readings":[{"text":"ひょう","rtext":"hyou","type":"ja_on","okuri":[],"sample":9,"perc":"64.29%"},{"text":"こおり","rtext":"koori","type":"ja_kun","okuri":[],"sample":3,"perc":"21.43%"},{"text":"ひ","rtext":"hi","type":"ja_kun","okuri":[],"sample":1,"perc":"7.14%"},{"text":"こお","rtext":"koo","type":"ja_kun","okuri":["る"],"sample":0,"perc":"0.00%"}],"meanings":["icicle","ice","hail","freeze","congeal"],"freq":1450,"grade":3}"#,
    );
    assert_eq!(
        serde_json::to_string(
            &kanji_info_json(&kanji_info_json_ctx, "光")
                
                .unwrap()
                .unwrap()
        )
        .unwrap(),
        r#"{"text":"光","rc":10,"rn":42,"strokes":6,"total":40,"irr":0,"irr_perc":"0.00%","readings":[{"text":"こう","rtext":"kou","type":"ja_on","okuri":[],"sample":36,"perc":"90.00%"},{"text":"ひか","rtext":"hika","type":"ja_kun","okuri":["る"],"sample":3,"perc":"7.50%"},{"text":"ひかり","rtext":"hikari","type":"ja_kun","okuri":[],"sample":1,"perc":"2.50%"}],"meanings":["ray","light"],"freq":527,"grade":2}"#,
    );
    assert!(kanji_info_json(&kanji_info_json_ctx, "z")
        
        .unwrap()
        .is_none());
}

// --- kanji_reading_json ---
fn kanji_reading_json_ctx() -> Arc<KaniranContext> {
    KaniranContext::from_env()
        
        .expect("DATABASE_URL / kaniran.toml required")
}

/// Serializes one kanji-reading match to JSON. A null-grade kanji (唖) keeps
/// stat_common=0 → `--.--%` and emits no grade field. Covers: link present
/// (人,学,月,本,唖) vs absent (々 iteration mark); rendaku tag (々,月,本) vs none;
/// geminated reading (学) vs none; stats present with grade (人,学,月,本), stats
/// absent (々), stats present with null grade (唖); and the original-reading
/// derivations — identity, rendaku strip (月), handakuten rendaku strip
/// (本: ぽん→ほん), geminated form (学: がっ→がく).
#[test]
fn kanji_reading_json_fixtures() {
    let kanji_reading_json_ctx = kanji_reading_json_ctx();
    let cases: &[(&str, &str, &str, bool, Option<&str>, &str)] = &[
        (
            "人",
            "ひと",
            "ja_kun",
            false,
            None,
            r#"{"kanji":"人","reading":"ひと","type":"ja_kun","link":true,"stats":true,"sample":47,"total":345,"perc":"13.62%","grade":1}"#,
        ),
        (
            "々",
            "びと",
            "ja_kun",
            true,
            None,
            r#"{"kanji":"々","reading":"びと","type":"ja_kun","rendaku":"RENDAKU"}"#,
        ),
        (
            "学",
            "がっ",
            "ja_on",
            false,
            Some("く"),
            r#"{"kanji":"学","reading":"がっ","type":"ja_on","link":true,"geminated":"く","stats":true,"sample":214,"total":216,"perc":"99.07%","grade":1}"#,
        ),
        (
            "月",
            "づき",
            "ja_kun",
            true,
            None,
            r#"{"kanji":"月","reading":"づき","type":"ja_kun","link":true,"rendaku":"RENDAKU","stats":true,"sample":18,"total":93,"perc":"19.35%","grade":1}"#,
        ),
        (
            "本",
            "ぽん",
            "ja_on",
            true,
            None,
            r#"{"kanji":"本","reading":"ぽん","type":"ja_on","link":true,"rendaku":"RENDAKU","stats":true,"sample":173,"total":177,"perc":"97.74%","grade":1}"#,
        ),
        (
            "唖",
            "あ",
            "ja_on",
            false,
            None,
            r#"{"kanji":"唖","reading":"あ","type":"ja_on","link":true,"stats":true,"sample":0,"total":0,"perc":"--.--%"}"#,
        ),
    ];
    for (kanji, reading, r#type, rendaku, geminated, expected) in cases {
        let js = kanji_reading_json(
            &kanji_reading_json_ctx,
            kanji,
            reading,
            r#type,
            *rendaku,
            *geminated,
        )
        
        .unwrap();
        let actual = serde_json::to_string(&js).unwrap();
        assert_eq!(
            actual.as_str(),
            *expected,
            "kanji={kanji} reading={reading}"
        );
    }
}

// --- process_match_json ---
fn process_match_json_ctx() -> Arc<KaniranContext> {
    KaniranContext::from_env()
        
        .expect("DATABASE_URL / kaniran.toml required")
}

/// Serializes a full match (a sequence of segments) to JSON. Covers: a kanji
/// segment followed by a non-kanji text run (見る → 見/み, る); a geminated
/// reading (学校 → 学/がっ gem く); a rendaku reading (三日月 → 月/づき); an
/// all-irregular word coalesced with a link (今日 → 今日/きょう); a single
/// irregular segment between two normal kanji (明日香 → 明, [日 irr], 香); the
/// iteration mark 々 which gets no link (人々 → 人, 々 rendaku); and a
/// geminate-then-rendaku pair (日本 → 日/にっ gem ち, 本/ぽん rendaku).
#[test]
fn process_match_json_fixtures() {
    let process_match_json_ctx = process_match_json_ctx();
    let cases: &[(&str, &str, &str)] = &[
        (
            "見る",
            "みる",
            r#"[{"kanji":"見","reading":"み","type":"ja_kun","link":true,"stats":true,"sample":135,"total":173,"perc":"78.03%","grade":1},{"text":"る"}]"#,
        ),
        (
            "学校",
            "がっこう",
            r#"[{"kanji":"学","reading":"がっ","type":"ja_on","link":true,"geminated":"く","stats":true,"sample":214,"total":216,"perc":"99.07%","grade":1},{"kanji":"校","reading":"こう","type":"ja_on","link":true,"stats":true,"sample":51,"total":51,"perc":"100.00%","grade":1}]"#,
        ),
        (
            "三日月",
            "みかづき",
            r#"[{"kanji":"三","reading":"み","type":"ja_kun","link":true,"stats":true,"sample":3,"total":89,"perc":"3.37%","grade":1},{"kanji":"日","reading":"か","type":"ja_kun","link":true,"stats":true,"sample":19,"total":263,"perc":"7.22%","grade":1},{"kanji":"月","reading":"づき","type":"ja_kun","link":true,"rendaku":"RENDAKU","stats":true,"sample":18,"total":93,"perc":"19.35%","grade":1}]"#,
        ),
        (
            "今日",
            "きょう",
            r#"[{"kanji":"今日","reading":"きょう","type":"irr","link":true}]"#,
        ),
        (
            "明日香",
            "あすか",
            r#"[{"kanji":"明","reading":"あ","type":"ja_kun","link":true,"stats":true,"sample":17,"total":85,"perc":"20.00%","grade":2},{"kanji":"日","reading":"す","type":"irr","link":true},{"kanji":"香","reading":"か","type":"ja_kun","link":true,"stats":true,"sample":0,"total":15,"perc":"0.00%","grade":8}]"#,
        ),
        (
            "人々",
            "ひとびと",
            r#"[{"kanji":"人","reading":"ひと","type":"ja_kun","link":true,"stats":true,"sample":47,"total":345,"perc":"13.62%","grade":1},{"kanji":"々","reading":"びと","type":"ja_kun","rendaku":"RENDAKU"}]"#,
        ),
        (
            "日本",
            "にっぽん",
            r#"[{"kanji":"日","reading":"にっ","type":"ja_on","link":true,"geminated":"ち","stats":true,"sample":90,"total":263,"perc":"34.22%","grade":1},{"kanji":"本","reading":"ぽん","type":"ja_on","link":true,"rendaku":"RENDAKU","stats":true,"sample":173,"total":177,"perc":"97.74%","grade":1}]"#,
        ),
    ];
    for (str, reading, expected) in cases {
        let match_ = match_readings(&process_match_json_ctx, str, reading)
            
            .unwrap()
            .expect("match-readings aligns");
        let result = process_match_json(&process_match_json_ctx, &match_)
            
            .unwrap();
        let actual = serde_json::to_string(&result).unwrap();
        assert_eq!(actual.as_str(), *expected, "str={str} reading={reading}");
    }
}

// --- match_readings_json ---
fn match_readings_json_ctx() -> Arc<KaniranContext> {
    KaniranContext::from_env()
        
        .expect("DATABASE_URL / kaniran.toml required")
}

/// Covers the two `None` short-circuits — no kanji in the word (みず/みず) and a
/// kanji word that cannot be aligned (日本/あ, 今日/"") — plus positive results:
/// an irregular fall-through when the reading matches no candidate (水/xyz →
/// 水 irr) and a kanji-plus-okurigana word (見る/みる). The exhaustive
/// JSON-shape coverage lives in [`super::super::process_match_json`]'s tests.
#[test]
fn match_readings_json_fixtures() {
    let match_readings_json_ctx = match_readings_json_ctx();

    assert!(
        match_readings_json(&match_readings_json_ctx, "みず", "みず")
            
            .unwrap()
            .is_none()
    );
    assert!(match_readings_json(&match_readings_json_ctx, "日本", "あ")
        
        .unwrap()
        .is_none());
    assert!(match_readings_json(&match_readings_json_ctx, "今日", "")
        
        .unwrap()
        .is_none());

    let positives: &[(&str, &str, &str)] = &[
        (
            "水",
            "xyz",
            r#"[{"kanji":"水","reading":"xyz","type":"irr","link":true}]"#,
        ),
        (
            "見る",
            "みる",
            r#"[{"kanji":"見","reading":"み","type":"ja_kun","link":true,"stats":true,"sample":135,"total":173,"perc":"78.03%","grade":1},{"text":"る"}]"#,
        ),
    ];
    for (str, reading, expected) in positives {
        let result = match_readings_json(&match_readings_json_ctx, str, reading)
            
            .unwrap()
            .expect("match aligns");
        let actual = serde_json::to_string(&result).unwrap();
        assert_eq!(actual.as_str(), *expected, "str={str} reading={reading}");
    }
}

// --- query_kanji_json_macro ---
fn query_kanji_json_macro_ctx() -> Arc<KaniranContext> {
    KaniranContext::from_env()
        
        .expect("DATABASE_URL / kaniran.toml required")
}

/// A single-row query serializes the kanji and appends caller-supplied extra
/// fields (here the row's text and id) after the base object, in order.
#[test]
fn query_kanji_json_single_row_extra_fields() {
    let query_kanji_json_macro_ctx = query_kanji_json_macro_ctx();
    let result = query_kanji_json(
        &query_kanji_json_macro_ctx,
        "select * from kanji where text = '檸'",
        |var| {
            vec![
                ("custom".to_owned(), Value::String(var.text.clone())),
                ("rid".to_owned(), Value::Number(var.id.into())),
            ]
        },
    )
    
    .unwrap();
    let expected = r#"[{"text":"檸","rc":75,"rn":75,"strokes":18,"total":0,"irr":0,"irr_perc":"--.--%","readings":[{"text":"ねい","rtext":"nei","type":"ja_on","okuri":[],"sample":0,"perc":"--.--%"},{"text":"どう","rtext":"dou","type":"ja_on","okuri":[],"sample":0,"perc":"--.--%"}],"meanings":["lemon tree"],"freq":null,"grade":null,"custom":"檸","rid":4193}]"#;
    assert_eq!(serde_json::to_string(&result).unwrap().as_str(), expected);
}

/// A multi-row query maps each row and applies the extra field to every
/// object; an empty result set yields an empty list.
#[test]
fn query_kanji_json_multi_and_empty() {
    let query_kanji_json_macro_ctx = query_kanji_json_macro_ctx();
    let multi = query_kanji_json(
        &query_kanji_json_macro_ctx,
        "select * from kanji where text in ('檸','薔') order by text",
        |_var| vec![("mark".to_owned(), Value::Bool(true))],
    )
    
    .unwrap();
    assert_eq!(multi.len(), 2);
    for obj in &multi {
        assert_eq!(obj["mark"], Value::Bool(true));
        assert!(obj.get("text").is_some(), "to-json fields present");
    }

    let empty = query_kanji_json(
        &query_kanji_json_macro_ctx,
        "select * from kanji where text = 'ZZZ'",
        |_var| vec![],
    )
    
    .unwrap();
    assert!(empty.is_empty());
}
