//! Audit `corpus/extracted/<pkg>/<sym>.parquet` fixtures against the
//! Rust transliterations. For each row, parse `args` and `result` via
//! `kani::sexp`, dispatch the args to the Rust function, compare the
//! produced value to the captured Lisp result.
//!
//! Run with:
//!   cargo run --release --example audit_fixtures
//!
//! Pass a path or directory:
//!   cargo run --release --example audit_fixtures -- corpus/extracted/characters
//!
//! Reports per-FQN pass / fail / skip counts + the first N mismatches.
//!
//! Mixed sync/async dispatch: pure-CPU handlers (`characters/*`,
//! `dict/get-digit`, etc.) run synchronously inside the tokio task;
//! DB-backed handlers (`dict/find-word`, etc.) `.await` against the
//! shared [`KaniranContext`]. Without `DATABASE_URL` set, DB handlers
//! report a clear "no context" error and the CPU handlers still pass.
//! Concurrency = [`CONCURRENCY`] in-flight tasks, so DB rows fan out
//! across the connection pool instead of serializing on a single
//! Postgres round-trip.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use arrow::array::StringArray;
use fancy_regex::Regex as FancyRegex;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use tokio::task::JoinSet;

use kaniran_core::conn::kani_context::KaniranContext;
use kaniran_core::characters::as_hiragana::as_hiragana;
use kaniran_core::characters::as_katakana::as_katakana;
use kaniran_core::characters::basic_split::{basic_split, SegmentKind};
use kaniran_core::characters::char_class_type::CharClass;
use kaniran_core::characters::geminate::geminate;
use kaniran_core::characters::kanji_prefix::kanji_prefix;
use kaniran_core::characters::mora_length::mora_length;
use kaniran_core::characters::normalize::normalize;
use kaniran_core::characters::rendaku::{rendaku, Voicing};
use kaniran_core::characters::sequential_kanji_positions::sequential_kanji_positions;
use kaniran_core::characters::simplify_ngrams::simplify_ngrams;
use kaniran_core::characters::split_by_regex::split_by_regex;
use kaniran_core::characters::test_word::test_word;
use kaniran_core::characters::to_normal_char::{to_normal_char, NormalizationContext};
use kaniran_core::dict::conj_data_prop::conj_data_prop;
use kaniran_core::dict::conj_data_struct::ConjData;
use kaniran_core::dict::conj_prop_dao::ConjProp;
use kaniran_core::dict::counter_age_class::CounterAge;
use kaniran_core::dict::counter_days_kun_class::CounterDaysKun;
use kaniran_core::dict::counter_days_on_class::CounterDaysOn;
use kaniran_core::dict::counter_halfhour_class::CounterHalfhour;
use kaniran_core::dict::counter_hifumi_class::CounterHifumi;
use kaniran_core::dict::counter_join::counter_join;
use kaniran_core::dict::find_counter::find_counter;
use kaniran_core::dict::counter_months_class::CounterMonths;
use kaniran_core::dict::counter_people_class::CounterPeople;
use kaniran_core::dict::compound_text_class::{CompoundText, ScoreMod};
use kaniran_core::dict::counter_text_class::{
    Common, Counter, CounterSource, CounterText, DigitOp, DigitOptEntry, DigitOptKey,
};
use kaniran_core::dict::counter_tsu_class::CounterTsu;
use kaniran_core::dict::counter_wari_class::CounterWari;
use kaniran_core::dict::common::common;
use kaniran_core::dict::kana_text_dao::KanaText;
use kaniran_core::dict::kani_word::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use kaniran_core::dict::kanji_text_dao::KanjiText;
use kaniran_core::dict::get_text::get_text;
use kaniran_core::dict::number_text_class::NumberText;
use kaniran_core::dict::proxy_text_class::ProxyText;
use kaniran_core::dict::segment_struct::Segment;
use kaniran_core::dict::seq::seq as seq_fn;
use kaniran_core::dict::simple_text_class::SimpleText;
use kaniran_core::dict::source::{source, SourceRef};
use kaniran_core::dict::value_string::value_string;
use kaniran_core::dict::verify::verify;
use kaniran_core::dict::word_info_class::WordInfoSeq;
use kaniran_core::dict::get_digit::get_digit;
use kaniran_core::dict::ordinal_str::ordinal_str;
use kaniran_core::dict::kani_conj_form::{ConjForm, FormToken};
use kaniran_core::dict::make_conj_data::make_conj_data;
use kaniran_core::dict::get_kana_forms_conj_data_filter::get_kana_forms_conj_data_filter;
use kaniran_core::dict::no_conj_data::no_conj_data;
use kaniran_core::dict::skip_by_conj_data::skip_by_conj_data;
use kaniran_core::dict::test_conj_prop::test_conj_prop;
use kaniran_core::kani::sexp::{self, Sexp};
use kaniran_core::numbers::_star_digit_kanji_default_star_::DIGIT_KANJI_DEFAULT;
use kaniran_core::numbers::_star_power_kanji_star_::POWER_KANJI;
use kaniran_core::numbers::group_to_kana::group_to_kana;
use kaniran_core::numbers::kani_num_class::NumClass;
use kaniran_core::numbers::number_to_kana::{number_to_kana, NumberToKanaOutput};
use kaniran_core::numbers::number_to_kanji::number_to_kanji;
use kaniran_core::numbers::parse_number::parse_number;

// Async DB-backed dict imports (merged from former audit_dict_fixtures.rs).
use kaniran_core::dict::find_word::{find_word, FindWordRows};
use kaniran_core::dict::find_word_as_hiragana::find_word_as_hiragana;
use kaniran_core::dict::find_word_conj_of::find_word_conj_of;
use kaniran_core::dict::find_word_seq::{find_word_seq, WordSeqRows};
use kaniran_core::dict::get_conj_data::{get_conj_data, FromOrConjIds};
use kaniran_core::dict::get_kana_form::get_kana_form;
use kaniran_core::dict::get_kana_forms::get_kana_forms;
use kaniran_core::dict::get_kana_forms_star_::get_kana_forms_star_;
use kaniran_core::dict::get_kanji_kana_old::get_kanji_kana_old;
use kaniran_core::dict::get_split::get_split;
use kaniran_core::dict::get_split_star_::get_split_star_;
use kaniran_core::dict::kani_split_part::SplitPart;
use kaniran_core::dict::simple_text_class::WordConjugations;
use kaniran_core::dict::word_conj_data::word_conj_data;

const MISMATCH_PRINT_LIMIT: usize = 5;
const CONCURRENCY: usize = 16;

/// Driver-scoped context populated in `main` when `DATABASE_URL` is
/// set. Sync handlers that need DB-backed caches (e.g.
/// [`audit_no_conj_data`]) read it via [`audit_ctx`]; if unset,
/// those handlers fail with a clear "no context" message rather
/// than silently passing on empty caches.
static AUDIT_CTX: std::sync::OnceLock<Arc<KaniranContext>> = std::sync::OnceLock::new();

fn audit_ctx() -> Option<&'static KaniranContext> {
    AUDIT_CTX.get().map(|a| &**a)
}

/// Single dispatch point. Sync handlers (CPU-only) return `Result`
/// directly; async DB handlers `.await` against the driver-scoped
/// [`audit_ctx`]. Returns `None` if the FQN has no registered
/// handler (audit_file logs a SKIP). Returns `Some(Err(...))` for
/// DB handlers when no `KaniranContext` is available.
async fn audit_one(
    fqn: &str, args: &Sexp, expected: &Sexp,
) -> Option<Result<(), String>> {
    let with_ctx = |missing: &str| audit_ctx().ok_or_else(||
        format!("{}: needs DB context — set DATABASE_URL ({})", fqn, missing));
    Some(match fqn {
        // --- sync (CPU-only) handlers ---
        "ICHIRAN/CHARACTERS:NORMALIZE"                  => audit_normalize(args, expected),
        "ICHIRAN/CHARACTERS:BASIC-SPLIT"                => audit_basic_split(args, expected),
        "ICHIRAN/CHARACTERS:AS-HIRAGANA"                => audit_as_hiragana(args, expected),
        "ICHIRAN/CHARACTERS:AS-KATAKANA"                => audit_as_katakana(args, expected),
        "ICHIRAN/CHARACTERS:MORA-LENGTH"                => audit_mora_length(args, expected),
        "ICHIRAN/CHARACTERS:KANJI-PREFIX"               => audit_kanji_prefix(args, expected),
        "ICHIRAN/CHARACTERS:SEQUENTIAL-KANJI-POSITIONS" => audit_sequential_kanji_positions(args, expected),
        "ICHIRAN/CHARACTERS:TO-NORMAL-CHAR"             => audit_to_normal_char(args, expected),
        "ICHIRAN/CHARACTERS:SIMPLIFY-NGRAMS"            => audit_simplify_ngrams(args, expected),
        "ICHIRAN/CHARACTERS:SPLIT-BY-REGEX"             => audit_split_by_regex(args, expected),
        "ICHIRAN/CHARACTERS:TEST-WORD"                  => audit_test_word(args, expected),
        "ICHIRAN/CHARACTERS:GEMINATE"                   => audit_geminate(args, expected),
        "ICHIRAN/CHARACTERS:RENDAKU"                    => audit_rendaku(args, expected),
        "ICHIRAN/NUMBERS:PARSE-NUMBER"                  => audit_parse_number(args, expected),
        "ICHIRAN/NUMBERS:NUMBER-TO-KANA"                => audit_number_to_kana(args, expected),
        "ICHIRAN/NUMBERS:NUMBER-TO-KANJI"               => audit_number_to_kanji(args, expected),
        "ICHIRAN/NUMBERS:GROUP-TO-KANA"                 => audit_group_to_kana(args, expected),
        "ICHIRAN/DICT::NO-CONJ-DATA"                    => audit_no_conj_data(args, expected),
        "ICHIRAN/DICT::MAKE-CONJ-DATA"                  => audit_make_conj_data(args, expected),
        "ICHIRAN/DICT::CONJ-DATA-PROP"                  => audit_conj_data_prop(args, expected),
        "ICHIRAN/DICT::TEST-CONJ-PROP"                  => audit_test_conj_prop(args, expected),
        "ICHIRAN/DICT::SKIP-BY-CONJ-DATA"               => audit_skip_by_conj_data(args, expected),
        "ICHIRAN/DICT::GET-KANA-FORMS-CONJ-DATA-FILTER" => audit_get_kana_forms_conj_data_filter(args, expected),
        "ICHIRAN/DICT::GET-DIGIT" | "ICHIRAN/DICT:GET-DIGIT"           => audit_get_digit(args, expected),
        "ICHIRAN/DICT::COUNTER-JOIN" | "ICHIRAN/DICT:COUNTER-JOIN"     => audit_counter_join(args, expected),
        "ICHIRAN/DICT::ORDINAL-STR" | "ICHIRAN/DICT:ORDINAL-STR"       => audit_ordinal_str(args, expected),
        "ICHIRAN/DICT::VERIFY" | "ICHIRAN/DICT:VERIFY"                 => audit_verify(args, expected),
        "ICHIRAN/DICT::FIND-COUNTER" | "ICHIRAN/DICT:FIND-COUNTER"     => audit_find_counter(args, expected),
        "ICHIRAN/DICT::SEQ" | "ICHIRAN/DICT:SEQ"                       => audit_seq(args, expected),
        "ICHIRAN/DICT::COMMON" | "ICHIRAN/DICT:COMMON"                 => audit_common(args, expected),
        "ICHIRAN/DICT::SOURCE" | "ICHIRAN/DICT:SOURCE"                 => audit_source(args, expected),
        "ICHIRAN/DICT::VALUE-STRING" | "ICHIRAN/DICT:VALUE-STRING"     => audit_value_string(args, expected),
        "ICHIRAN/DICT::GET-TEXT" | "ICHIRAN/DICT:GET-TEXT"             => audit_get_text(args, expected),

        // --- async DB-backed handlers ---
        "ICHIRAN/DICT:FIND-WORD" => match with_ctx("find-word") {
            Err(e) => Err(e),
            Ok(ctx) => audit_find_word(ctx, args, expected).await,
        },
        "ICHIRAN/DICT:FIND-WORD-AS-HIRAGANA" => match with_ctx("find-word-as-hiragana") {
            Err(e) => Err(e),
            Ok(ctx) => audit_find_word_as_hiragana(ctx, args, expected).await,
        },
        "ICHIRAN/DICT::FIND-WORD-SEQ" => match with_ctx("find-word-seq") {
            Err(e) => Err(e),
            Ok(ctx) => audit_find_word_seq(ctx, args, expected).await,
        },
        "ICHIRAN/DICT::FIND-WORD-CONJ-OF" => match with_ctx("find-word-conj-of") {
            Err(e) => Err(e),
            Ok(ctx) => audit_find_word_conj_of(ctx, args, expected).await,
        },
        "ICHIRAN/DICT::GET-KANA-FORM" => match with_ctx("get-kana-form") {
            Err(e) => Err(e),
            Ok(ctx) => audit_get_kana_form(ctx, args, expected).await,
        },
        "ICHIRAN/DICT::GET-KANA-FORMS" => match with_ctx("get-kana-forms") {
            Err(e) => Err(e),
            Ok(ctx) => audit_get_kana_forms(ctx, args, expected).await,
        },
        "ICHIRAN/DICT::GET-KANA-FORMS*" => match with_ctx("get-kana-forms*") {
            Err(e) => Err(e),
            Ok(ctx) => audit_get_kana_forms_star(ctx, args, expected).await,
        },
        "ICHIRAN/DICT::GET-CONJ-DATA" => match with_ctx("get-conj-data") {
            Err(e) => Err(e),
            Ok(ctx) => audit_get_conj_data(ctx, args, expected).await,
        },
        "ICHIRAN/DICT:WORD-CONJ-DATA" => match with_ctx("word-conj-data") {
            Err(e) => Err(e),
            Ok(ctx) => audit_word_conj_data(ctx, args, expected).await,
        },
        "ICHIRAN/DICT::GET-KANJI-KANA-OLD" | "ICHIRAN/DICT:GET-KANJI-KANA-OLD" => {
            match with_ctx("get-kanji-kana-old") {
                Err(e) => Err(e),
                Ok(ctx) => audit_get_kanji_kana_old(ctx, args, expected).await,
            }
        }
        "ICHIRAN/DICT:GET-SPLIT" => match with_ctx("get-split") {
            Err(e) => Err(e),
            Ok(ctx) => audit_get_split(ctx, args, expected).await,
        },
        "ICHIRAN/DICT:GET-SPLIT*" => match with_ctx("get-split*") {
            Err(e) => Err(e),
            Ok(ctx) => audit_get_split_star(ctx, args, expected).await,
        },

        _ => return None,
    })
}


// --- shared helpers ------------------------------------------------------

fn list_elems(s: &Sexp) -> Result<Vec<&Sexp>, String> {
    s.list_iter()
        .map(|it| it.collect())
        .ok_or_else(|| format!("expected proper list, got {}", s))
}

fn expect_one(expected: &Sexp) -> Result<&Sexp, String> {
    let elems = list_elems(expected)?;
    if elems.len() != 1 {
        return Err(format!("expected 1-element list, got {}", expected));
    }
    Ok(elems[0])
}

fn parse_keyword_arg(args: &[&Sexp], i: usize, key: &str) -> Result<Option<String>, String> {
    if args.len() <= i { return Ok(None); }
    let k = args[i].as_keyword()
        .ok_or_else(|| format!("arg {} not a keyword: {}", i, args[i]))?;
    if k != key {
        return Err(format!("expected :{}, got :{}", key, k));
    }
    if args.len() <= i + 1 {
        return Err(format!("missing value after :{}", key));
    }
    let v = args[i + 1].as_keyword()
        .ok_or_else(|| format!("value after :{} not a keyword: {}", key, args[i + 1]))?;
    Ok(Some(v.to_string()))
}

fn parse_norm_context(name: Option<&str>) -> Result<NormalizationContext, String> {
    Ok(match name {
        None | Some("DEFAULT") => NormalizationContext::Default,
        Some("KANA") => NormalizationContext::Kana,
        Some(other) => return Err(format!("unknown NormalizationContext: :{}", other)),
    })
}

fn parse_char_class(name: &str) -> Result<CharClass, String> {
    Ok(match name {
        "KATAKANA" => CharClass::Katakana,
        "KATAKANA-UNIQ" => CharClass::KatakanaUniq,
        "HIRAGANA" => CharClass::Hiragana,
        "KANJI" => CharClass::Kanji,
        "KANJI-CHAR" => CharClass::KanjiChar,
        "KANA" => CharClass::Kana,
        "TRADITIONAL" => CharClass::Traditional,
        "NONWORD" => CharClass::Nonword,
        "NUMBER" => CharClass::Number,
        other => return Err(format!("unknown CharClass: :{}", other)),
    })
}

/// `(:CHAR n)` → char.
fn parse_char_tag(s: &Sexp) -> Result<char, String> {
    let elems = list_elems(s).map_err(|e| format!("(:CHAR n) shape: {}", e))?;
    if elems.len() != 2 { return Err(format!("(:CHAR n) wants 2 elements, got {}", s)); }
    if elems[0].as_keyword() != Some("CHAR") {
        return Err(format!("(:CHAR n) car: {}", elems[0]));
    }
    let cp = elems[1].as_i64().ok_or_else(|| format!("(:CHAR n) cdr: {}", elems[1]))? as u32;
    char::from_u32(cp).ok_or_else(|| format!("invalid codepoint: {}", cp))
}


// --- per-FQN handlers ----------------------------------------------------

fn audit_normalize(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    let argv = list_elems(args)?;
    let s = argv[0].as_str().ok_or("arg 0 not string")?;
    let ctx = parse_norm_context(parse_keyword_arg(&argv, 1, "CONTEXT")?.as_deref())?;
    let actual = normalize(s, ctx);
    let exp = expect_one(expected)?.as_str().ok_or("expected[0] not string")?;
    if actual == exp { Ok(()) } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", actual, exp))
    }
}

fn audit_as_hiragana(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    let argv = list_elems(args)?;
    let s = argv[0].as_str().ok_or("arg 0 not string")?;
    let actual = as_hiragana(s);
    let exp = expect_one(expected)?.as_str().ok_or("expected[0] not string")?;
    if actual == exp { Ok(()) } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", actual, exp))
    }
}

fn audit_as_katakana(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    let argv = list_elems(args)?;
    let s = argv[0].as_str().ok_or("arg 0 not string")?;
    let actual = as_katakana(s);
    let exp = expect_one(expected)?.as_str().ok_or("expected[0] not string")?;
    if actual == exp { Ok(()) } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", actual, exp))
    }
}

fn audit_mora_length(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    let argv = list_elems(args)?;
    let s = argv[0].as_str().ok_or("arg 0 not string")?;
    let actual = mora_length(s) as i64;
    let exp = expect_one(expected)?.as_i64().ok_or("expected[0] not int")?;
    if actual == exp { Ok(()) } else {
        Err(format!("\n  rust: {}\n  lisp: {}", actual, exp))
    }
}

fn audit_kanji_prefix(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    let argv = list_elems(args)?;
    let s = argv[0].as_str().ok_or("arg 0 not string")?;
    let actual = kanji_prefix(s);
    let exp = expect_one(expected)?.as_str().ok_or("expected[0] not string")?;
    if actual == exp { Ok(()) } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", actual, exp))
    }
}

fn audit_sequential_kanji_positions(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    let argv = list_elems(args)?;
    let s = argv[0].as_str().ok_or("arg 0 not string")?;
    let offset = argv.get(1).and_then(|v| v.as_i64()).unwrap_or(0) as usize;
    let actual: Vec<i64> = sequential_kanji_positions(s, offset).into_iter().map(|n| n as i64).collect();
    // expected: ((p1 p2 ...))  — outer 1-element list, inner list-of-ints (or NIL)
    let inner = expect_one(expected)?;
    let exp: Vec<i64> = if inner.is_nil() {
        Vec::new()
    } else {
        list_elems(inner)?.iter()
            .map(|s| s.as_i64().ok_or_else(|| format!("position not int: {}", s)))
            .collect::<Result<_, _>>()?
    };
    if actual == exp { Ok(()) } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", actual, exp))
    }
}

fn audit_to_normal_char(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    let argv = list_elems(args)?;
    let c = parse_char_tag(argv[0])?;
    let ctx = parse_norm_context(parse_keyword_arg(&argv, 1, "CONTEXT")?.as_deref())?;
    let actual = to_normal_char(c, ctx);
    // expected: (NIL) or ((:CHAR n))
    let inner = expect_one(expected)?;
    let exp = if inner.is_nil() {
        None
    } else {
        Some(parse_char_tag(inner)?)
    };
    if actual == exp { Ok(()) } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", actual, exp))
    }
}

fn audit_simplify_ngrams(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    let argv = list_elems(args)?;
    let s = argv[0].as_str().ok_or("arg 0 not string")?;
    // arg 1 is a flat plist of (k1 v1 k2 v2 ...) — pair them up.
    let plist = list_elems(argv[1])?;
    if plist.len() % 2 != 0 {
        return Err(format!("plist has odd length: {}", plist.len()));
    }
    let mut map: Vec<(String, String)> = Vec::with_capacity(plist.len() / 2);
    for pair in plist.chunks(2) {
        let k = pair[0].as_str().ok_or_else(|| format!("plist key not string: {}", pair[0]))?;
        let v = pair[1].as_str().ok_or_else(|| format!("plist value not string: {}", pair[1]))?;
        map.push((k.to_string(), v.to_string()));
    }
    let actual = simplify_ngrams(s, &map);
    // expected: (cleaned T)  — multiple values; first is the string, second is bool.
    let exp_elems = list_elems(expected)?;
    let exp_str = exp_elems.first().and_then(|v| v.as_str())
        .ok_or("expected[0] not string")?;
    if actual == exp_str { Ok(()) } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", actual, exp_str))
    }
}

fn audit_split_by_regex(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    let argv = list_elems(args)?;
    let pattern = argv[0].as_str().ok_or("arg 0 not regex string")?;
    let s = argv[1].as_str().ok_or("arg 1 not string")?;
    let re = FancyRegex::new(pattern)
        .map_err(|e| format!("regex compile failed: {}", e))?;
    let actual: Vec<String> = split_by_regex(&re, s);
    let inner = expect_one(expected)?;
    let exp: Vec<String> = if inner.is_nil() {
        Vec::new()
    } else {
        list_elems(inner)?.iter()
            .map(|v| v.as_str().map(String::from).ok_or_else(|| format!("token not string: {}", v)))
            .collect::<Result<_, _>>()?
    };
    if actual == exp { Ok(()) } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", actual, exp))
    }
}

fn audit_seq(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    let argv = list_elems(args)?;
    if argv.len() != 1 {
        return Err(format!("seq wants 1 arg, got {}", argv.len()));
    }
    // Skip variants outside the Rust word polymorphism. Lisp dispatches
    // `seq` on conjugation/sense/sense-prop/restricted-readings/entry/
    // conj-source-reading too, but those are typed-known DAOs at every
    // callsite — the Rust port reads `.seq` directly without going
    // through a dispatcher. Auditing those rows would be a category
    // error: the polymorphic surface doesn't include them.
    let class = list_elems(argv[0])
        .ok().and_then(|e| plist_class(&e).ok())
        .unwrap_or_default();
    if matches!(class.as_str(),
        "CONJUGATION" | "SENSE" | "SENSE-PROP"
        | "RESTRICTED-READINGS" | "ENTRY" | "CONJ-SOURCE-READING") {
        return Ok(()); // out of polymorphic scope; treat as pass
    }
    let word = parse_word_plist(argv[0])?;
    let actual = seq_fn(&word);
    let exp = expect_one(expected)?;
    let actual_repr = repr_word_info_seq(&actual);
    let exp_repr = repr_seq_expected(exp)?;
    if actual_repr == exp_repr { Ok(()) } else {
        Err(format!("\n  rust: {}\n  lisp: {}", actual_repr, exp_repr))
    }
}

fn audit_common(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    let argv = list_elems(args)?;
    if argv.len() != 1 {
        return Err(format!("common wants 1 arg, got {}", argv.len()));
    }
    let word = parse_word_plist(argv[0])?;
    let actual = common(&word);
    let exp = expect_one(expected)?;
    let actual_repr = repr_common(&actual);
    let exp_repr = repr_common_expected(exp)?;
    if actual_repr == exp_repr { Ok(()) } else {
        Err(format!("\n  rust: {}\n  lisp: {}", actual_repr, exp_repr))
    }
}

fn audit_source(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    let argv = list_elems(args)?;
    if argv.len() != 1 {
        return Err(format!("source wants 1 arg, got {}", argv.len()));
    }
    let word = parse_word_plist(argv[0])?;
    let actual = source(&word);
    let exp = expect_one(expected)?;
    // Compare on (variant_name, seq, text, kana) — slot identity is enough
    // to verify the dispatcher returned the same source row.
    let actual_id = source_identity(&actual);
    let exp_id = expected_source_identity(exp)?;
    if actual_id == exp_id { Ok(()) } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", actual_id, exp_id))
    }
}

fn audit_value_string(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    // dict-counters.lisp:44-49 — captured args shape is a single counter
    // plist. parse_word_plist + variant filter produce the Counter; then
    // patch in :SUFFIX-DESCRIPTIONS (parse_counter omits it because the
    // counter-join audit doesn't need it, but the value-string default
    // body reads it).
    let argv = list_elems(args)?;
    if argv.len() != 1 {
        return Err(format!("value-string wants 1 arg, got {}", argv.len()));
    }
    let word = parse_word_plist(argv[0])?;
    let counter = match word {
        KaniWordDispatchEnum::Counter(c) => c,
        other => return Err(format!("value-string expects counter plist, got {:?}", other)),
    };
    let plist = list_elems(argv[0])?;
    let descs = plist_get(&plist, "SUFFIX-DESCRIPTIONS")
        .map(parse_string_list)
        .transpose()?
        .unwrap_or_default();
    let counter = patch_suffix_descriptions(counter, descs);
    let actual = value_string(&counter);
    let exp = expect_one(expected)?.as_str().ok_or("expected[0] not string")?;
    if actual == exp { Ok(()) } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", actual, exp))
    }
}

/// Parse a NIL-or-`(string string ...)` plist value into a `Vec<String>`.
fn parse_string_list(s: &Sexp) -> Result<Vec<String>, String> {
    if s.is_nil() { return Ok(Vec::new()); }
    list_elems(s)?
        .into_iter()
        .map(|e| e.as_str().map(|s| s.to_string())
             .ok_or_else(|| format!("string-list element not string: {}", e)))
        .collect()
}

/// Replace `suffix_descriptions` on the [`CounterText`] inside any
/// [`Counter`] variant. value-string's default-method audit needs the
/// slot populated; parse_counter leaves it empty because counter-join
/// (its first audit consumer) doesn't read it.
fn patch_suffix_descriptions(counter: Counter, descs: Vec<String>) -> Counter {
    fn patch(c: &mut CounterText, d: Vec<String>) { c.suffix_descriptions = d; }
    match counter {
        Counter::Base(mut c)        => { patch(&mut c, descs); Counter::Base(c) }
        Counter::NumberText(NumberText(mut c)) => { patch(&mut c, descs); Counter::NumberText(NumberText(c)) }
        Counter::Age(CounterAge(mut c))        => { patch(&mut c, descs); Counter::Age(CounterAge(c)) }
        Counter::DaysKun(CounterDaysKun(mut c))=> { patch(&mut c, descs); Counter::DaysKun(CounterDaysKun(c)) }
        Counter::DaysOn(CounterDaysOn(mut c))  => { patch(&mut c, descs); Counter::DaysOn(CounterDaysOn(c)) }
        Counter::Halfhour(CounterHalfhour(mut c))=> { patch(&mut c, descs); Counter::Halfhour(CounterHalfhour(c)) }
        Counter::Months(CounterMonths(mut c))  => { patch(&mut c, descs); Counter::Months(CounterMonths(c)) }
        Counter::People(CounterPeople(mut c))  => { patch(&mut c, descs); Counter::People(CounterPeople(c)) }
        Counter::Tsu(CounterTsu(mut c))        => { patch(&mut c, descs); Counter::Tsu(CounterTsu(c)) }
        Counter::Wari(CounterWari(mut c))      => { patch(&mut c, descs); Counter::Wari(CounterWari(c)) }
        Counter::Hifumi(mut h) => { patch(&mut h.base, descs); Counter::Hifumi(h) }
    }
}

fn audit_get_text(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    // dict.lisp:18-20 + 677-679 (segment override). Captured fixture
    // covers two surfaces: SEGMENT (lazy memoization via Segment::get_text)
    // and the word polymorphism (T method = text gf), which fires for
    // KANA-TEXT / KANJI-TEXT / COMPOUND-TEXT plists. The entry method
    // (dict.lisp:47-49) needs DB; no entry rows in this corpus.
    let argv = list_elems(args)?;
    if argv.len() != 1 {
        return Err(format!("get-text wants 1 arg, got {}", argv.len()));
    }
    let class = list_elems(argv[0])
        .ok().and_then(|e| plist_class(&e).ok())
        .unwrap_or_default();
    let exp = expect_one(expected)?.as_str().ok_or("expected[0] not string")?;
    let actual: String = if class == "SEGMENT" {
        let plist = list_elems(argv[0])?;
        let word_v = plist_get(&plist, "WORD").ok_or(":WORD missing on segment")?;
        let word = parse_word_plist(word_v)?;
        let start = plist_get_i64(&plist, "START").unwrap_or(0).max(0) as usize;
        let end = plist_get_i64(&plist, "END").unwrap_or(0).max(0) as usize;
        let mut seg = Segment {
            start, end, word,
            score: None, info: None, top: None, text: None,
        };
        seg.get_text().to_string()
    } else if class == "ENTRY" {
        // entry method needs DB; out of scope for the sync harness.
        return Ok(());
    } else {
        let word = parse_word_plist(argv[0])?;
        get_text(&word).into_owned()
    };
    if actual == exp { Ok(()) } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", actual, exp))
    }
}

// --- result repr helpers ------------------------------------------------

fn repr_word_info_seq(v: &Option<WordInfoSeq>) -> String {
    match v {
        None => "NIL".into(),
        Some(WordInfoSeq::Single(n)) => format!("{}", n),
        Some(WordInfoSeq::Multi(v)) => {
            let s: Vec<String> = v.iter().map(|n| n.to_string()).collect();
            format!("({})", s.join(" "))
        }
    }
}

/// Seq's Lisp result is bare: int, list, or NIL — captured as the values-list
/// element. Single int → `42`. List → `(1 2 3)`. NIL → `NIL`.
fn repr_seq_expected(s: &Sexp) -> Result<String, String> {
    if s.is_nil() { return Ok("NIL".into()); }
    if let Some(n) = s.as_i64() { return Ok(format!("{}", n)); }
    let elems = list_elems(s)?;
    let mut nums = Vec::with_capacity(elems.len());
    for e in elems {
        if e.is_nil() {
            // mapcar over a list with nil entries — Rust elides these.
            // Skip to keep the comparison aligned with the Rust port's
            // documented compound-text flatten behavior.
            continue;
        }
        let n = e.as_i64().ok_or_else(|| format!("seq result element not int/NIL: {}", e))?;
        nums.push(n.to_string());
    }
    Ok(format!("({})", nums.join(" ")))
}

fn repr_common(c: &Common) -> String {
    match c {
        Common::Score(n) => format!("{}", n),
        Common::Null => "NULL".into(),
        Common::Inherit => "INHERIT".into(), // unreachable from the dispatcher
    }
}

/// Common's Lisp result: integer / `:NULL` / `NIL` / sometimes a `(or db-null integer)` row value.
fn repr_common_expected(s: &Sexp) -> Result<String, String> {
    if s.is_nil() { return Ok("NULL".into()); }
    if let Some(kw) = s.as_keyword() {
        if kw.eq_ignore_ascii_case("NULL") { return Ok("NULL".into()); }
        return Err(format!("common result keyword: :{}", kw));
    }
    if let Some(n) = s.as_i64() { return Ok(format!("{}", n)); }
    Err(format!("common result shape: {}", s))
}

/// Source identity = (variant-class-tag, seq, text). The Lisp result of
/// `(source obj)` is whatever the slot held — for both counter-text and
/// proxy-text, that's a kanji-text or kana-text row (or `NIL`). The
/// counter-vs-proxy distinction lives at the input, not the output, so
/// equality just compares row identity.
type SourceIdentity = Option<(String, i32, String)>;

fn source_identity(s: &Option<SourceRef<'_>>) -> SourceIdentity {
    match s {
        None => None,
        Some(SourceRef::CounterKanji(k)) => Some(("KANJI-TEXT".into(), k.seq, k.text.clone())),
        Some(SourceRef::CounterKana(k)) => Some(("KANA-TEXT".into(), k.seq, k.text.clone())),
        Some(SourceRef::ProxySimple(s)) => simple_identity(s),
    }
}

fn simple_identity(s: &KaniSimpleTextDispatchEnum) -> SourceIdentity {
    match s {
        KaniSimpleTextDispatchEnum::Kanji(k) => Some(("KANJI-TEXT".into(), k.seq, k.text.clone())),
        KaniSimpleTextDispatchEnum::Kana(k) => Some(("KANA-TEXT".into(), k.seq, k.text.clone())),
        KaniSimpleTextDispatchEnum::Proxy(p) => simple_identity(&p.source),
    }
}

fn expected_source_identity(s: &Sexp) -> Result<SourceIdentity, String> {
    if s.is_nil() { return Ok(None); }
    let elems = list_elems(s)?;
    if elems.len() % 2 != 0 {
        return Err(format!("source result plist odd length: {}", s));
    }
    let class = plist_class(&elems)?;
    if class.eq_ignore_ascii_case("PROXY-TEXT") {
        // Lisp's source can return another proxy if its source slot holds
        // one. Recurse into the inner SOURCE slot to find the leaf row.
        let inner = plist_get(&elems, "SOURCE")
            .ok_or("proxy source plist missing :SOURCE")?;
        return expected_source_identity(inner);
    }
    let seq = plist_get_i64(&elems, "SEQ").unwrap_or(0) as i32;
    let text = plist_get_string(&elems, "TEXT");
    Ok(Some((class, seq, text)))
}

// --- plist → KaniWordDispatchEnum reconstruction ------------------------

/// Recursively reconstruct a `KaniWordDispatchEnum` from a captured
/// plist. Handles every variant the segmenter dispatches on: kanji-text,
/// kana-text, proxy-text, compound-text, and the 11 counter-text
/// subclasses. Slots not relevant to seq/common/source dispatchers are
/// filled with default placeholders.
fn parse_word_plist(s: &Sexp) -> Result<KaniWordDispatchEnum, String> {
    let elems = list_elems(s)?;
    if elems.len() % 2 != 0 {
        return Err(format!("word plist odd length: {}", s));
    }
    let class = plist_class(&elems)?;
    match class.as_str() {
        "KANA-TEXT" => Ok(KaniWordDispatchEnum::Kana(parse_kana_text(&elems)?)),
        "KANJI-TEXT" => Ok(KaniWordDispatchEnum::Kanji(parse_kanji_text(&elems)?)),
        "PROXY-TEXT" => Ok(KaniWordDispatchEnum::Proxy(parse_proxy_text(&elems)?)),
        "COMPOUND-TEXT" => Ok(KaniWordDispatchEnum::Compound(parse_compound_text(&elems)?)),
        c if c.starts_with("COUNTER-") || c == "NUMBER-TEXT" => {
            Ok(KaniWordDispatchEnum::Counter(parse_counter(&elems, c)?))
        }
        other => Err(format!("unknown word :CLASS: :{}", other)),
    }
}

fn parse_simple_text_plist(s: &Sexp) -> Result<KaniSimpleTextDispatchEnum, String> {
    let elems = list_elems(s)?;
    let class = plist_class(&elems)?;
    match class.as_str() {
        "KANA-TEXT" => Ok(KaniSimpleTextDispatchEnum::Kana(parse_kana_text(&elems)?)),
        "KANJI-TEXT" => Ok(KaniSimpleTextDispatchEnum::Kanji(parse_kanji_text(&elems)?)),
        "PROXY-TEXT" => Ok(KaniSimpleTextDispatchEnum::Proxy(parse_proxy_text(&elems)?)),
        other => Err(format!("simple-text :CLASS not Kana/Kanji/Proxy: :{}", other)),
    }
}

fn plist_class(elems: &[&Sexp]) -> Result<String, String> {
    for pair in elems.chunks(2) {
        if pair[0].as_keyword().map(|k| k.eq_ignore_ascii_case("CLASS")).unwrap_or(false) {
            return pair[1].as_keyword()
                .map(|s| s.to_string())
                .ok_or_else(|| format!(":CLASS value not keyword: {}", pair[1]));
        }
    }
    Err("plist missing :CLASS".into())
}

fn plist_get<'a>(elems: &[&'a Sexp], key: &str) -> Option<&'a Sexp> {
    for pair in elems.chunks(2) {
        if pair[0].as_keyword().map(|k| k.eq_ignore_ascii_case(key)).unwrap_or(false) {
            return Some(pair[1]);
        }
    }
    None
}

fn plist_get_i64(elems: &[&Sexp], key: &str) -> Option<i64> {
    plist_get(elems, key).and_then(|v| v.as_i64())
}

fn plist_get_string(elems: &[&Sexp], key: &str) -> String {
    plist_get(elems, key).and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_default()
}

fn plist_common_to_optional_i32(v: &Sexp) -> Option<i32> {
    if v.is_nil() { return None; }
    if let Some(kw) = v.as_keyword() {
        if kw.eq_ignore_ascii_case("NULL") { return None; }
    }
    v.as_i64().map(|n| n as i32)
}

fn plist_common_to_enum(v: &Sexp) -> Common {
    if v.is_nil() { return Common::Inherit; }
    if let Some(kw) = v.as_keyword() {
        if kw.eq_ignore_ascii_case("NULL") { return Common::Null; }
    }
    if let Some(n) = v.as_i64() { return Common::Score(n as i32); }
    Common::Inherit
}

fn parse_kanji_text(elems: &[&Sexp]) -> Result<KanjiText, String> {
    Ok(KanjiText {
        id: plist_get_i64(elems, "ID").unwrap_or(0) as i32,
        seq: plist_get_i64(elems, "SEQ").unwrap_or(0) as i32,
        text: plist_get_string(elems, "TEXT"),
        ord: plist_get_i64(elems, "ORD").unwrap_or(0) as i32,
        common: plist_get(elems, "COMMON").map(plist_common_to_optional_i32).unwrap_or(None),
        common_tags: plist_get_string(elems, "COMMON-TAGS"),
        conjugate_p: !plist_get(elems, "CONJUGATE-P").map(|v| v.is_nil()).unwrap_or(true),
        nokanji: !plist_get(elems, "NOKANJI").map(|v| v.is_nil()).unwrap_or(true),
        best_kana: plist_get(elems, "BEST-KANA").and_then(|v| {
            if v.is_nil() { None }
            else if v.as_keyword().map(|k| k.eq_ignore_ascii_case("NULL")).unwrap_or(false) { None }
            else { v.as_str().map(|s| s.to_string()) }
        }),
        state: SimpleText::default(),
    })
}

fn parse_kana_text(elems: &[&Sexp]) -> Result<KanaText, String> {
    Ok(KanaText {
        id: plist_get_i64(elems, "ID").unwrap_or(0) as i32,
        seq: plist_get_i64(elems, "SEQ").unwrap_or(0) as i32,
        text: plist_get_string(elems, "TEXT"),
        ord: plist_get_i64(elems, "ORD").unwrap_or(0) as i32,
        common: plist_get(elems, "COMMON").map(plist_common_to_optional_i32).unwrap_or(None),
        common_tags: plist_get_string(elems, "COMMON-TAGS"),
        conjugate_p: !plist_get(elems, "CONJUGATE-P").map(|v| v.is_nil()).unwrap_or(true),
        nokanji: !plist_get(elems, "NOKANJI").map(|v| v.is_nil()).unwrap_or(true),
        best_kanji: plist_get(elems, "BEST-KANJI").and_then(|v| {
            if v.is_nil() { None }
            else if v.as_keyword().map(|k| k.eq_ignore_ascii_case("NULL")).unwrap_or(false) { None }
            else { v.as_str().map(|s| s.to_string()) }
        }),
        state: SimpleText::default(),
    })
}

fn parse_proxy_text(elems: &[&Sexp]) -> Result<ProxyText, String> {
    let source_v = plist_get(elems, "SOURCE")
        .ok_or("proxy plist missing :SOURCE")?;
    let inner = parse_simple_text_plist(source_v)?;
    Ok(ProxyText {
        text: plist_get_string(elems, "TEXT"),
        kana: plist_get_string(elems, "KANA"),
        source: Box::new(inner),
        state: SimpleText::default(),
    })
}

fn parse_compound_text(elems: &[&Sexp]) -> Result<CompoundText, String> {
    let primary_v = plist_get(elems, "PRIMARY")
        .ok_or("compound plist missing :PRIMARY")?;
    let primary = Box::new(parse_word_plist(primary_v)?);
    let mut words = Vec::new();
    if let Some(words_v) = plist_get(elems, "WORDS") {
        if !words_v.is_nil() {
            for w in list_elems(words_v)? {
                words.push(parse_word_plist(w)?);
            }
        }
    }
    Ok(CompoundText {
        text: plist_get_string(elems, "TEXT"),
        kana: plist_get_string(elems, "KANA"),
        primary,
        words,
        score_base: None,
        score_mod: ScoreMod::Single(0),
    })
}

fn parse_counter(elems: &[&Sexp], class: &str) -> Result<Counter, String> {
    let source = plist_get(elems, "SOURCE").and_then(|v| {
        if v.is_nil() { return None; }
        // SOURCE is itself a kanji-text or kana-text plist.
        match parse_simple_text_plist(v).ok()? {
            KaniSimpleTextDispatchEnum::Kanji(k) => Some(CounterSource::Kanji(k)),
            KaniSimpleTextDispatchEnum::Kana(k) => Some(CounterSource::Kana(k)),
            KaniSimpleTextDispatchEnum::Proxy(_) => None, // counter-text source isn't a proxy
        }
    });
    let common = plist_get(elems, "COMMON").map(plist_common_to_enum).unwrap_or(Common::Inherit);
    let base = CounterText {
        text: plist_get_string(elems, "TEXT"),
        kana: plist_get_string(elems, "KANA"),
        number_text: plist_get_string(elems, "NUMBER-TEXT"),
        number: plist_get_i64(elems, "NUMBER").unwrap_or(0).max(0) as u64,
        source,
        ordinalp: !plist_get(elems, "ORDINALP").map(|v| v.is_nil()).unwrap_or(true),
        suffix: plist_get(elems, "SUFFIX").and_then(|v| {
            if v.is_nil() { None } else { v.as_str().map(|s| s.to_string()) }
        }),
        accepts_suffixes: Vec::new(),
        suffix_descriptions: Vec::new(),
        digit_opts: Vec::new(),
        common,
        allowed: Vec::new(),
        foreign: !plist_get(elems, "FOREIGN").map(|v| v.is_nil()).unwrap_or(true),
    };
    Ok(match class {
        "COUNTER-TEXT" => Counter::Base(base),
        "NUMBER-TEXT" => Counter::NumberText(NumberText(base)),
        "COUNTER-TSU" => Counter::Tsu(CounterTsu(base)),
        "COUNTER-AGE" => Counter::Age(CounterAge(base)),
        "COUNTER-DAYS-KUN" => Counter::DaysKun(CounterDaysKun(base)),
        "COUNTER-DAYS-ON" => Counter::DaysOn(CounterDaysOn(base)),
        "COUNTER-HALFHOUR" => Counter::Halfhour(CounterHalfhour(base)),
        "COUNTER-MONTHS" => Counter::Months(CounterMonths(base)),
        "COUNTER-PEOPLE" => Counter::People(CounterPeople(base)),
        "COUNTER-WARI" => Counter::Wari(CounterWari(base)),
        "COUNTER-HIFUMI" => Counter::Hifumi(CounterHifumi { base, digit_set: Vec::new() }),
        other => return Err(format!("unknown counter :CLASS: :{}", other)),
    })
}

fn audit_find_counter(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    // dict-counters.lisp:273 — `(find-counter number counter &key (unique t))`.
    // Captured args shape: `(<number-str> <counter-str> :UNIQUE T)`.
    // Expected: a values-wrapped list of counter plists, possibly NIL.
    let argv = list_elems(args)?;
    if argv.len() < 2 {
        return Err(format!("find-counter wants ≥2 args, got {}", argv.len()));
    }
    let number = argv[0].as_str().ok_or("arg 0 (number) not string")?;
    let counter = argv[1].as_str().ok_or("arg 1 (counter) not string")?;
    // `:UNIQUE` value is the bare symbol `T` or `NIL` (not a keyword),
    // so parse_keyword_arg's keyword=keyword shape doesn't fit. Pass
    // through Option<bool> — the absent case lets find_counter apply
    // the upstream `(unique t)` default at its boundary instead of
    // here.
    let unique: Option<bool> = {
        let mut found: Option<bool> = None;
        let mut i = 2;
        while i + 1 < argv.len() {
            if let Some(k) = argv[i].as_keyword() {
                if k.eq_ignore_ascii_case("UNIQUE") {
                    found = Some(!argv[i + 1].is_nil());
                    break;
                }
            }
            i += 1;
        }
        found
    };

    let ctx = audit_ctx().ok_or("find-counter needs DB context — set DATABASE_URL")?;
    let actual = find_counter(ctx, number, counter, unique);
    let actual_keys: Vec<(String, String, String)> = actual
        .iter()
        .map(|c| (
            counter_variant_name(c).to_string(),
            c.base().text.clone(),
            c.base().kana.clone(),
        ))
        .collect();

    let expected_list = expect_one(expected)?;
    let expected_keys = parse_find_counter_expected(expected_list)?;

    // Compare as multisets — sort by (text, kana, class) and equality-check.
    let mut a = actual_keys;
    let mut e = expected_keys;
    a.sort();
    e.sort();
    if a == e { Ok(()) } else {
        Err(format!("\n  rust ({} items): {:?}\n  lisp ({} items): {:?}",
                    a.len(), a, e.len(), e))
    }
}

fn counter_variant_name(c: &Counter) -> &'static str {
    match c {
        Counter::Base(_) => "COUNTER-TEXT",
        Counter::NumberText(_) => "NUMBER-TEXT",
        Counter::Tsu(_) => "COUNTER-TSU",
        Counter::Age(_) => "COUNTER-AGE",
        Counter::DaysKun(_) => "COUNTER-DAYS-KUN",
        Counter::DaysOn(_) => "COUNTER-DAYS-ON",
        Counter::Halfhour(_) => "COUNTER-HALFHOUR",
        Counter::Hifumi(_) => "COUNTER-HIFUMI",
        Counter::Months(_) => "COUNTER-MONTHS",
        Counter::People(_) => "COUNTER-PEOPLE",
        Counter::Wari(_) => "COUNTER-WARI",
    }
}

/// Parse the captured `(<plist1> <plist2> ...)` result form into a
/// list of (variant-class-name, text, kana) tuples. NIL → empty.
fn parse_find_counter_expected(s: &Sexp) -> Result<Vec<(String, String, String)>, String> {
    if s.is_nil() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in list_elems(s)? {
        let elems = list_elems(entry)?;
        if elems.len() % 2 != 0 {
            return Err(format!("counter plist has odd element count: {}", entry));
        }
        let mut class: Option<String> = None;
        let mut text: Option<String> = None;
        let mut kana: Option<String> = None;
        for pair in elems.chunks(2) {
            let k = pair[0].as_keyword()
                .ok_or_else(|| format!("counter plist key not keyword: {}", pair[0]))?;
            let v = pair[1];
            match k {
                "CLASS" => class = Some(v.as_keyword()
                    .ok_or_else(|| format!(":CLASS value not keyword: {}", v))?
                    .to_string()),
                "TEXT" => text = Some(v.as_str()
                    .ok_or_else(|| format!(":TEXT value not string: {}", v))?
                    .to_string()),
                "KANA" => kana = Some(v.as_str()
                    .ok_or_else(|| format!(":KANA value not string: {}", v))?
                    .to_string()),
                _ => {}
            }
        }
        out.push((
            class.ok_or("counter plist missing :CLASS")?,
            text.ok_or("counter plist missing :TEXT")?,
            kana.ok_or("counter plist missing :KANA")?,
        ));
    }
    Ok(out)
}

fn audit_verify(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    // dict-counters.lisp:31 — `(verify counter unique)`. Captured args
    // shape is `(<counter-plist> <unique>)`; verify only reads the
    // counter's CLASS (variant pick), NUMBER (counter-tsu, days-on,
    // and the default's allowed-membership check), and ALLOWED
    // (default method). Other plist slots are reconstructable but
    // load-bearing for nothing under verify.
    let argv = list_elems(args)?;
    if argv.len() != 2 {
        return Err(format!("verify wants 2 args, got {}", argv.len()));
    }
    let (class, number, allowed) = parse_verify_slots(argv[0])?;
    let unique = !argv[1].is_nil(); // T → true, NIL → false
    let counter = stub_counter_for_verify(class, number, allowed);
    let actual = verify(&counter, unique);
    let exp_first = expect_one(expected)?;
    let exp = if exp_first.is_nil() { false } else if exp_first.is_t() { true } else {
        return Err(format!("expected[0] not T/NIL: {}", exp_first));
    };
    if actual == exp { Ok(()) } else {
        Err(format!("\n  rust: {}\n  lisp: {}", actual, exp))
    }
}

/// Walk the captured plist for the three slots `verify` reads. The
/// plist's `:SOURCE` value is itself a plist with its own `:CLASS`
/// keyword — pair-chunked iteration handles the nesting correctly
/// because we never recurse into values.
fn parse_verify_slots(plist: &Sexp) -> Result<(String, u64, Vec<i32>), String> {
    let elems = list_elems(plist)?;
    if elems.len() % 2 != 0 {
        return Err(format!("counter plist has odd element count: {}", plist));
    }
    let mut class: Option<String> = None;
    let mut number: u64 = 0;
    let mut allowed: Vec<i32> = Vec::new();
    for pair in elems.chunks(2) {
        let k = pair[0]
            .as_keyword()
            .ok_or_else(|| format!("counter plist key not keyword: {}", pair[0]))?;
        let v = pair[1];
        match k {
            "CLASS" => {
                class = Some(v.as_keyword()
                    .ok_or_else(|| format!(":CLASS value not keyword: {}", v))?
                    .to_string());
            }
            "NUMBER" => {
                let n = v.as_i64()
                    .ok_or_else(|| format!(":NUMBER value not int: {}", v))?;
                number = u64::try_from(n)
                    .map_err(|_| format!(":NUMBER negative: {}", n))?;
            }
            "ALLOWED" => {
                if !v.is_nil() {
                    for e in list_elems(v)? {
                        let n = e.as_i64()
                            .ok_or_else(|| format!(":ALLOWED entry not int: {}", e))?;
                        allowed.push(i32::try_from(n)
                            .map_err(|_| format!(":ALLOWED entry overflows i32: {}", n))?);
                    }
                }
            }
            _ => {} // ignore other slots
        }
    }
    let class = class.ok_or("counter plist missing :CLASS")?;
    Ok((class, number, allowed))
}

/// Build a [`Counter`] of the right variant with only the slots
/// `verify` reads filled in. Other slots get neutral defaults — the
/// dispatcher and per-variant methods don't read them. CounterHifumi's
/// `digit_set` is initialised to empty here on purpose: its `verify`
/// chains to the default, which never reads digit_set.
fn stub_counter_for_verify(class: String, number: u64, allowed: Vec<i32>) -> Counter {
    let base = CounterText {
        text: String::new(),
        kana: String::new(),
        number_text: number.to_string(),
        number,
        source: None,
        ordinalp: false,
        suffix: None,
        accepts_suffixes: Vec::new(),
        suffix_descriptions: Vec::new(),
        digit_opts: Vec::new(),
        common: Common::Inherit,
        allowed,
        foreign: false,
    };
    match class.as_str() {
        "COUNTER-TEXT" => Counter::Base(base),
        "NUMBER-TEXT" => Counter::NumberText(NumberText(base)),
        "COUNTER-TSU" => Counter::Tsu(CounterTsu(base)),
        "COUNTER-AGE" => Counter::Age(CounterAge(base)),
        "COUNTER-DAYS-KUN" => Counter::DaysKun(CounterDaysKun(base)),
        "COUNTER-DAYS-ON" => Counter::DaysOn(CounterDaysOn(base)),
        "COUNTER-HALFHOUR" => Counter::Halfhour(CounterHalfhour(base)),
        "COUNTER-MONTHS" => Counter::Months(CounterMonths(base)),
        "COUNTER-PEOPLE" => Counter::People(CounterPeople(base)),
        "COUNTER-WARI" => Counter::Wari(CounterWari(base)),
        "COUNTER-HIFUMI" => Counter::Hifumi(CounterHifumi { base, digit_set: Vec::new() }),
        other => panic!("unknown counter :CLASS keyword :{}", other),
    }
}

fn audit_ordinal_str(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    let argv = list_elems(args)?;
    if argv.len() != 1 {
        return Err(format!("ordinal-str wants 1 arg, got {}", argv.len()));
    }
    let n = argv[0].as_i64().ok_or("arg 0 not int")?;
    let actual = ordinal_str(n);
    let exp = expect_one(expected)?.as_str().ok_or("expected[0] not string")?;
    if actual == exp { Ok(()) } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", actual, exp))
    }
}

fn audit_get_digit(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    let argv = list_elems(args)?;
    let n = argv[0].as_i64().ok_or("arg 0 not int")?;
    let actual = get_digit(n);
    let exp_first = expect_one(expected)?;
    let exp = if exp_first.is_nil() {
        None
    } else {
        Some(exp_first.as_i64().ok_or("expected[0] not int/nil")?)
    };
    if actual == exp { Ok(()) } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", actual, exp))
    }
}

fn audit_counter_join(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    // dict-counters.lisp:3-7,101-201 — captured args shape is
    // (<counter-plist> <n> <number-kana-str> <counter-kana-str>); the
    // body only consumes counter.base().digit_opts and .foreign, so we
    // only parse those slots from the plist and stub the rest on a
    // Counter::Base. Variant choice doesn't affect counter-join (no
    // per-subclass override).
    let argv = list_elems(args)?;
    if argv.len() != 4 {
        return Err(format!("counter-join wants 4 args, got {}", argv.len()));
    }
    let (digit_opts, foreign) = parse_counter_join_slots(argv[0])?;
    let n = argv[1].as_i64().ok_or("arg 1 (n) not int")?;
    let number_kana = argv[2].as_str().ok_or("arg 2 (number-kana) not string")?.to_string();
    let counter_kana = argv[3].as_str().ok_or("arg 3 (counter-kana) not string")?.to_string();
    let counter = stub_counter(digit_opts, foreign);
    let actual = counter_join(&counter, n, number_kana, counter_kana);
    let exp = expect_one(expected)?.as_str().ok_or("expected[0] not string")?;
    if actual == exp {
        Ok(())
    } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", actual, exp))
    }
}

fn stub_counter(digit_opts: Vec<DigitOptEntry>, foreign: bool) -> Counter {
    Counter::Base(CounterText {
        text: String::new(),
        kana: String::new(),
        number_text: "0".into(),
        number: 0,
        source: None,
        ordinalp: false,
        suffix: None,
        accepts_suffixes: Vec::new(),
        suffix_descriptions: Vec::new(),
        digit_opts,
        common: Common::Inherit,
        allowed: Vec::new(),
        foreign,
    })
}

/// Walk the captured `(:KEY value :KEY value ...)` counter plist and
/// pluck the two slots `counter-join` reads. Other keys (`:TEXT`,
/// `:KANA`, `:SOURCE`, `:NUMBER-TEXT`, …) are ignored on purpose —
/// reconstructing them would require porting the JMdict DAO row
/// parsers and isn't load-bearing for this audit.
fn parse_counter_join_slots(plist: &Sexp) -> Result<(Vec<DigitOptEntry>, bool), String> {
    let elems = list_elems(plist)?;
    if elems.len() % 2 != 0 {
        return Err(format!("counter plist has odd element count: {}", plist));
    }
    let mut digit_opts: Vec<DigitOptEntry> = Vec::new();
    let mut foreign = false;
    for pair in elems.chunks(2) {
        let k = pair[0]
            .as_keyword()
            .ok_or_else(|| format!("counter plist key not keyword: {}", pair[0]))?;
        let v = pair[1];
        match k {
            "DIGIT-OPTS" => {
                if !v.is_nil() {
                    for entry in list_elems(v)? {
                        digit_opts.push(parse_digit_opt_entry(entry)?);
                    }
                }
            }
            "FOREIGN" => foreign = !v.is_nil(),
            _ => {} // ignore other slots
        }
    }
    Ok((digit_opts, foreign))
}

fn parse_digit_opt_entry(entry: &Sexp) -> Result<DigitOptEntry, String> {
    let elems = list_elems(entry)
        .map_err(|_| format!("digit-opts entry not a list: {}", entry))?;
    if elems.is_empty() {
        return Err(format!("digit-opts entry empty: {}", entry));
    }
    let key = if let Some(kw) = elems[0].as_keyword() {
        if kw.eq_ignore_ascii_case("OFF") {
            DigitOptKey::Off
        } else {
            return Err(format!("unknown digit-opts key keyword: :{}", kw));
        }
    } else if let Some(d) = elems[0].as_i64() {
        DigitOptKey::Digit(d as i32)
    } else {
        return Err(format!("digit-opts entry car not int / :off: {}", elems[0]));
    };
    let mut ops: Vec<DigitOp> = Vec::with_capacity(elems.len() - 1);
    for o in &elems[1..] {
        ops.push(parse_digit_op(o)?);
    }
    Ok(DigitOptEntry { key, ops })
}

fn parse_digit_op(s: &Sexp) -> Result<DigitOp, String> {
    if let Some(kw) = s.as_keyword() {
        return Ok(match kw {
            "G" => DigitOp::Geminate,
            "R" => DigitOp::Rendaku,
            "H" => DigitOp::Handakuten,
            "C" => DigitOp::Counter,
            other => return Err(format!("unknown digit-op keyword: :{}", other)),
        });
    }
    if let Some(s) = s.as_str() {
        return Ok(DigitOp::Replace(s.to_string()));
    }
    Err(format!("digit-op not keyword/string: {}", s))
}

fn audit_geminate(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    // characters.lisp:336 (defun geminate (string &key fresh)) — fresh
    // defaults nil → in-place mutation, returns the mutated string.
    // The capture's args are just `(string)`; the Rust port mirrors
    // the in-place semantics via &mut String.
    let argv = list_elems(args)?;
    let s = argv[0].as_str().ok_or("arg 0 not string")?;
    let mut owned = s.to_string();
    geminate(&mut owned);
    let exp = expect_one(expected)?.as_str().ok_or("expected[0] not string")?;
    if owned == exp { Ok(()) } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", owned, exp))
    }
}

fn audit_rendaku(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    // characters.lisp:320 (defun rendaku (string &key handakuten fresh))
    // fresh defaults nil → in-place. handakuten=t → Voicing::Handakuten,
    // else Voicing::Dakuten. The capture omits :FRESH (defaulted) and
    // includes :HANDAKUTEN only when set; per CONVENTIONS §4.4 the
    // boolean became a 2-variant enum.
    let argv = list_elems(args)?;
    let s = argv[0].as_str().ok_or("arg 0 not string")?;
    let voicing = if find_bool_keyword(&argv, "HANDAKUTEN") {
        Voicing::Handakuten
    } else {
        Voicing::Dakuten
    };
    let mut owned = s.to_string();
    rendaku(&mut owned, voicing);
    let exp = expect_one(expected)?.as_str().ok_or("expected[0] not string")?;
    if owned == exp { Ok(()) } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", owned, exp))
    }
}

fn audit_parse_number(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    // numbers.lisp:77 (defun parse-number (s)) — returns u64 on
    // success, raises NOT-A-NUMBER on invalid input. The Lisp capture
    // wrapper converts the raise to a single-NIL result via
    // handler-case at the trace edge; in our trace the success path
    // captures `(<int>)`, the failure path captures `(NIL)`.
    let argv = list_elems(args)?;
    let s = argv[0].as_str().ok_or("arg 0 not string")?;
    let actual = parse_number(s).ok();
    let exp_first = expect_one(expected)?;
    let exp = if exp_first.is_nil() {
        None
    } else {
        Some(exp_first.as_i64().ok_or("expected[0] not int/nil")? as u64)
    };
    if actual == exp { Ok(()) } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", actual, exp))
    }
}

fn audit_number_to_kana(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    // numbers.lisp:122 (defun number-to-kana (n &key separator method))
    // separator defaults to #\zero-width-space (8203); method defaults
    // to 'number-to-kanji-default-style which is number-to-kanji with
    // DIGIT_KANJI_DEFAULT + POWER_KANJI + one-sen=nil. Tracer captures
    // characters as (:CHAR <codepoint>).
    let argv = list_elems(args)?;
    let n = argv[0].as_i64().ok_or("arg 0 not int")? as u64;
    let separator = parse_char_arg(&argv, 1, "SEPARATOR")?
        .or(Some('\u{200B}'));  // zero-width space default
    let actual = number_to_kana(n, separator, |k| {
        number_to_kanji(k, DIGIT_KANJI_DEFAULT, POWER_KANJI, false)
    });
    let exp_first = expect_one(expected)?;
    // Capture shape is "(<string>)" for joined or "(<list-of-strings>)"
    // for groups. In practice the Lisp callers always pass a separator,
    // so the joined form is what we see.
    match (&actual, exp_first.as_str()) {
        (NumberToKanaOutput::Joined(s), Some(exp)) => {
            if s == exp { Ok(()) } else {
                Err(format!("\n  rust: {:?}\n  lisp: {:?}", s, exp))
            }
        }
        _ => Err(format!("shape mismatch:\n  rust: {:?}\n  lisp: {}", actual, exp_first)),
    }
}

fn audit_number_to_kanji(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    // numbers.lisp:35 (defun number-to-kanji (n &optional digits powers
    // &key one-sen)) — digits / powers / one-sen all default. Capture
    // typically shows `(<n>)` only when the caller relied on defaults;
    // explicit args fill the trailing slots. Mirror the upstream
    // defaulting in Rust by filling in DIGIT_KANJI_DEFAULT / POWER_KANJI
    // / false when the positional slots are absent (i.e. the next arg
    // is a keyword or end-of-list).
    let argv = list_elems(args)?;
    let n = argv[0].as_i64().ok_or("arg 0 not int")? as u64;
    // Walk positional optionals from index 1; stop at the first keyword
    // (or end of list). The slot AFTER a keyword is that keyword's
    // value, not a positional arg.
    let mut idx = 1usize;
    let digits = match argv.get(idx) {
        Some(s) if s.as_keyword().is_none() => {
            idx += 1;
            s.as_str().ok_or("arg 1 (digits) not string")?
        }
        _ => DIGIT_KANJI_DEFAULT,
    };
    let powers = match argv.get(idx) {
        Some(s) if s.as_keyword().is_none() => {
            s.as_str().ok_or("arg 2 (powers) not string")?
        }
        _ => POWER_KANJI,
    };
    // Upstream keyword is `:1sen` (digit-1 + "sen"), not `:one-sen`.
    let one_sen = find_bool_keyword(&argv, "1SEN");
    let actual = number_to_kanji(n, digits, powers, one_sen);
    let exp = expect_one(expected)?.as_str().ok_or("expected[0] not string")?;
    if actual == exp { Ok(()) } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", actual, exp))
    }
}

fn audit_group_to_kana(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    // numbers.lisp:117 (defun group-to-kana (group)) — group is a list
    // of (CLASS . VAL) cons pairs in upstream; the projector emits each
    // as a 2-element list `(:JD 0)` so the captured shape is
    // `(((:JD 0) (:P 1) ...))`. Parse into Vec<(NumClass, u8)>.
    let argv = list_elems(args)?;
    let group_sexp = argv.first().ok_or("missing group arg")?;
    let group_elems = list_elems(group_sexp)?;
    let mut group: Vec<(NumClass, u8)> = Vec::with_capacity(group_elems.len());
    for entry in group_elems {
        let pair = list_elems(entry)?;
        if pair.len() != 2 {
            return Err(format!("expected 2-elem (class val) pair, got {}", entry));
        }
        let class_kw = pair[0].as_keyword().ok_or("class not keyword")?;
        let class = match class_kw {
            "JD" => NumClass::Jd,
            "AD" => NumClass::Ad,
            "P"  => NumClass::P,
            other => return Err(format!("unknown class :{}", other)),
        };
        let val = pair[1].as_i64().ok_or("val not int")? as u8;
        group.push((class, val));
    }
    // The Lisp default tables match number-to-kanji-default-style;
    // group-to-kana doesn't take tables in upstream — they're stored
    // in *digit-to-kana* / *power-to-kana* and consulted internally.
    // Rust takes them as args; pass the same defaults.
    use kaniran_core::numbers::_star_digit_to_kana_star_::DIGIT_TO_KANA;
    use kaniran_core::numbers::_star_power_to_kana_star_::POWER_TO_KANA;
    let actual = group_to_kana(&group, DIGIT_TO_KANA, POWER_TO_KANA);
    let exp = expect_one(expected)?.as_str().ok_or("expected[0] not string")?;
    if actual == exp { Ok(()) } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", actual, exp))
    }
}

/// Scan `args` for a `:KEY <truthy>` pair. Returns `true` only when the
/// keyword is present and the following value parses as Lisp truthy
/// (`T` symbol, anything non-NIL). Used for boolean-flag keyword args
/// like `:HANDAKUTEN T` / `:ONE-SEN T` where the value is a symbol, not
/// a keyword (the existing `parse_keyword_arg` requires keyword=keyword
/// pairs).
fn find_bool_keyword(args: &[&Sexp], key: &str) -> bool {
    let mut i = 0;
    while i + 1 < args.len() {
        if let Some(k) = args[i].as_keyword() {
            if k.eq_ignore_ascii_case(key) {
                return !args[i + 1].is_nil();
            }
        }
        i += 1;
    }
    false
}

/// Helper: parse `(... :KEY (:CHAR codepoint) ...)` keyword arg into Option<char>.
fn parse_char_arg(args: &[&Sexp], start: usize, key: &str) -> Result<Option<char>, String> {
    let mut i = start;
    while i + 1 < args.len() {
        if let Some(k) = args[i].as_keyword() {
            if k.eq_ignore_ascii_case(key) {
                if args[i + 1].is_nil() {
                    return Ok(None);
                }
                if let Some(c) = args[i + 1].as_char() {
                    return Ok(Some(c));
                }
                // Tagged form: (:CHAR <codepoint>)
                let pair = list_elems(args[i + 1])?;
                if pair.len() == 2
                    && pair[0].as_keyword().map(|k| k.eq_ignore_ascii_case("CHAR")).unwrap_or(false)
                {
                    let cp = pair[1].as_i64().ok_or("CHAR codepoint not int")? as u32;
                    return Ok(char::from_u32(cp));
                }
                return Err(format!("unrecognized char form for :{}: {}", key, args[i + 1]));
            }
        }
        i += 1;
    }
    Ok(None)
}

fn audit_test_word(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    let argv = list_elems(args)?;
    let s = argv[0].as_str().ok_or("arg 0 not string")?;
    let kw = argv[1].as_keyword().ok_or("arg 1 not keyword")?;
    let class = parse_char_class(kw)?;
    let actual = test_word(s, class);
    // Lisp `test-word` calls cl-ppcre:scan which returns 4 values:
    // (start end groups names) on match, or NIL on no match. The
    // multiple-value-list capture is therefore either `(NIL)` (no match)
    // or `(start end #() #())` (match). Compare just the truthiness of
    // the first element. The Rust port collapsed to `bool` ahead of
    // CONVENTIONS §4.1 — see TODO upstream.
    let exp_elems = list_elems(expected)?;
    let exp_truthy = match exp_elems.first() {
        Some(first) => !first.is_nil(),
        None => return Err("expected list is empty".into()),
    };
    if actual == exp_truthy { Ok(()) } else {
        Err(format!("\n  rust bool: {}\n  lisp:      {}", actual, expected))
    }
}

// --- conj-data cluster helpers (waves 100, 113-121) ----------------------

fn parse_int_or_nil(s: &Sexp) -> Result<Option<i32>, String> {
    if s.is_nil() { return Ok(None); }
    Ok(Some(s.as_i64().ok_or_else(|| format!("not int/nil: {}", s))? as i32))
}

fn parse_bool_or_dbnull(s: &Sexp) -> Result<Option<bool>, String> {
    if s.is_nil() { return Ok(Some(false)); }
    if s.is_t() { return Ok(Some(true)); }
    if s.as_keyword() == Some("NULL") { return Ok(None); }
    Err(format!("not T / NIL / :NULL: {}", s))
}

fn parse_conj_prop_plist(s: &Sexp) -> Result<ConjProp, String> {
    let elems = list_elems(s)?;
    if elems.len() % 2 != 0 {
        return Err(format!("conj-prop plist odd length: {}", s));
    }
    let mut id = None;
    let mut conj_id = None;
    let mut conj_type = None;
    let mut pos = None;
    let mut neg = None;
    let mut fml = None;
    let mut class_ok = false;
    for pair in elems.chunks(2) {
        let k = pair[0].as_keyword()
            .ok_or_else(|| format!("conj-prop plist key not keyword: {}", pair[0]))?;
        let v = pair[1];
        match k {
            "CLASS" => {
                if v.as_keyword() != Some("CONJ-PROP") {
                    return Err(format!(":CLASS not :CONJ-PROP: {}", v));
                }
                class_ok = true;
            }
            "ID" => id = Some(v.as_i64().ok_or("conj-prop :ID not int")? as i32),
            "CONJ-ID" => conj_id = Some(v.as_i64().ok_or("conj-prop :CONJ-ID not int")? as i32),
            "CONJ-TYPE" => conj_type = Some(v.as_i64().ok_or("conj-prop :CONJ-TYPE not int")? as i32),
            "POS" => pos = Some(v.as_str().ok_or("conj-prop :POS not string")?.to_string()),
            "NEG" => neg = Some(parse_bool_or_dbnull(v)?),
            "FML" => fml = Some(parse_bool_or_dbnull(v)?),
            other => return Err(format!("unknown conj-prop key: :{}", other)),
        }
    }
    if !class_ok { return Err(":CLASS missing on conj-prop".into()); }
    Ok(ConjProp {
        id: id.ok_or(":ID missing")?,
        conj_id: conj_id.ok_or(":CONJ-ID missing")?,
        conj_type: conj_type.ok_or(":CONJ-TYPE missing")?,
        pos: pos.ok_or(":POS missing")?,
        neg: neg.ok_or(":NEG missing")?,
        fml: fml.ok_or(":FML missing")?,
    })
}

fn parse_string_pair(s: &Sexp) -> Result<(String, String), String> {
    let v = list_elems(s)?;
    if v.len() != 2 {
        return Err(format!("src-map pair want 2 elems, got {}: {}", v.len(), s));
    }
    Ok((
        v[0].as_str().ok_or_else(|| format!("src-map[0] not string: {}", v[0]))?.to_string(),
        v[1].as_str().ok_or_else(|| format!("src-map[1] not string: {}", v[1]))?.to_string(),
    ))
}

fn parse_src_map(s: &Sexp) -> Result<Vec<(String, String)>, String> {
    if s.is_nil() { return Ok(Vec::new()); }
    list_elems(s)?.into_iter().map(parse_string_pair).collect()
}

fn parse_conj_data_plist(s: &Sexp) -> Result<ConjData, String> {
    let elems = list_elems(s)?;
    if elems.len() % 2 != 0 {
        return Err(format!("conj-data plist odd length: {}", s));
    }
    let mut seq = None;
    let mut from = None;
    let mut via = None;
    let mut prop = None;
    let mut src_map = None;
    let mut class_ok = false;
    for pair in elems.chunks(2) {
        let k = pair[0].as_keyword()
            .ok_or_else(|| format!("conj-data plist key not keyword: {}", pair[0]))?;
        let v = pair[1];
        match k {
            "CLASS" => {
                if v.as_keyword() != Some("CONJ-DATA") {
                    return Err(format!(":CLASS not :CONJ-DATA: {}", v));
                }
                class_ok = true;
            }
            "SEQ"  => seq = Some(parse_int_or_nil(v)?),
            "FROM" => from = Some(parse_int_or_nil(v)?),
            "VIA"  => via = Some(parse_int_or_nil(v)?),
            "PROP" => prop = Some(if v.is_nil() { None } else { Some(parse_conj_prop_plist(v)?) }),
            "SRC-MAP" => src_map = Some(parse_src_map(v)?),
            other => return Err(format!("unknown conj-data key: :{}", other)),
        }
    }
    if !class_ok { return Err(":CLASS missing on conj-data".into()); }
    Ok(ConjData {
        seq: seq.ok_or(":SEQ missing")?,
        from: from.ok_or(":FROM missing")?,
        via: via.ok_or(":VIA missing")?,
        prop: prop.ok_or(":PROP missing")?,
        src_map: src_map.ok_or(":SRC-MAP missing")?,
    })
}

fn parse_form_token(s: &Sexp) -> Result<FormToken, String> {
    if s.is_nil() { return Ok(FormToken::Bool(false)); }
    if s.is_t() { return Ok(FormToken::Bool(true)); }
    if let Some(k) = s.as_keyword() {
        return match k {
            "ANY" => Ok(FormToken::Any),
            "NULL" => Ok(FormToken::DbNull),
            other => Err(format!("unknown form-token keyword: :{}", other)),
        };
    }
    if let Some(n) = s.as_i64() { return Ok(FormToken::Int(n as i32)); }
    if let Some(s) = s.as_str() {
        // Leak the string; pos values in form data are short and the
        // audit run is one-shot. Avoids inventing a Cow variant.
        return Ok(FormToken::Str(Box::leak(s.to_string().into_boxed_str())));
    }
    Err(format!("can't parse as form-token: {}", s))
}

fn parse_conj_form(s: &Sexp) -> Result<ConjForm, String> {
    let elems = list_elems(s)?;
    let toks: Vec<FormToken> = elems.iter().map(|e| parse_form_token(e))
        .collect::<Result<_, _>>()?;
    match toks.as_slice() {
        [a, b, c]    => Ok(ConjForm::Triple(*a, *b, *c)),
        [a, b, c, d] => Ok(ConjForm::Quadruple(*a, *b, *c, *d)),
        _ => Err(format!("conj-form length not 3 or 4: {}", s)),
    }
}

/// Project a Rust [`ConjProp`] back to its captured plist for diffing.
fn render_conj_prop(p: &ConjProp) -> String {
    fn b(b: Option<bool>) -> &'static str {
        match b { Some(true) => "T", Some(false) => "NIL", None => ":NULL" }
    }
    format!(
        "(:CLASS :CONJ-PROP :ID {} :CONJ-ID {} :CONJ-TYPE {} :POS {:?} :NEG {} :FML {})",
        p.id, p.conj_id, p.conj_type, p.pos, b(p.neg), b(p.fml),
    )
}

fn render_conj_data(cd: &ConjData) -> String {
    fn opt_int(n: Option<i32>) -> String {
        match n { Some(n) => n.to_string(), None => "NIL".into() }
    }
    let prop_str = match &cd.prop {
        Some(p) => render_conj_prop(p),
        None => "NIL".into(),
    };
    let src_map_str = if cd.src_map.is_empty() {
        "NIL".into()
    } else {
        let pairs: Vec<String> = cd.src_map.iter()
            .map(|(t, s)| format!("({:?} {:?})", t, s))
            .collect();
        format!("({})", pairs.join(" "))
    };
    format!(
        "(:CLASS :CONJ-DATA :SEQ {} :FROM {} :VIA {} :PROP {} :SRC-MAP {})",
        opt_int(cd.seq), opt_int(cd.from), opt_int(cd.via), prop_str, src_map_str,
    )
}


// --- conj-data cluster handlers ------------------------------------------

fn audit_no_conj_data(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    let argv = list_elems(args)?;
    let seq = argv.first().and_then(|s| s.as_i64())
        .ok_or("no-conj-data arg 0 not int")? as i32;
    let ctx = audit_ctx()
        .ok_or("no-conj-data: KaniranContext not initialised (DATABASE_URL unset?)")?;
    let actual = no_conj_data(ctx, seq);
    let inner = expect_one(expected)?;
    let exp = if inner.is_t() { true } else if inner.is_nil() { false } else {
        return Err(format!("no-conj-data expected[0] not T/NIL: {}", inner));
    };
    if actual == exp { Ok(()) } else {
        Err(format!("\n  rust: {}\n  lisp: {}", actual, exp))
    }
}

fn audit_make_conj_data(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    let argv = list_elems(args)?;
    if argv.len() % 2 != 0 {
        return Err(format!("make-conj-data args plist odd length: {}", args));
    }
    let mut seq = None;
    let mut from = None;
    let mut via = None;
    let mut prop = None;
    let mut src_map = None;
    for pair in argv.chunks(2) {
        let k = pair[0].as_keyword()
            .ok_or_else(|| format!("make-conj-data arg key not keyword: {}", pair[0]))?;
        let v = pair[1];
        match k {
            "SEQ"  => seq = Some(parse_int_or_nil(v)?),
            "FROM" => from = Some(parse_int_or_nil(v)?),
            "VIA"  => via = Some(parse_int_or_nil(v)?),
            "PROP" => prop = Some(if v.is_nil() { None } else { Some(parse_conj_prop_plist(v)?) }),
            "SRC-MAP" => src_map = Some(parse_src_map(v)?),
            other => return Err(format!("unknown make-conj-data key: :{}", other)),
        }
    }
    let actual = make_conj_data(
        seq.unwrap_or(None),
        from.unwrap_or(None),
        via.unwrap_or(None),
        prop.unwrap_or(None),
        src_map.unwrap_or_default(),
    );
    let exp = parse_conj_data_plist(expect_one(expected)?)?;
    compare_conj_data(&actual, &exp)
}

fn audit_conj_data_prop(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    let argv = list_elems(args)?;
    let cd = parse_conj_data_plist(argv.first()
        .ok_or("conj-data-prop args empty")?)?;
    let actual = conj_data_prop(&cd);
    let inner = expect_one(expected)?;
    match (actual, inner.is_nil()) {
        (None, true) => Ok(()),
        (Some(a), false) => {
            let e = parse_conj_prop_plist(inner)?;
            if conj_prop_eq(&a, &e) { Ok(()) } else {
                Err(format!("\n  rust: {}\n  lisp: {}",
                    render_conj_prop(&a), render_conj_prop(&e)))
            }
        }
        (Some(a), true)  => Err(format!("rust returned prop, lisp NIL\n  rust: {}", render_conj_prop(&a))),
        (None, false)    => Err(format!("rust returned None, lisp returned a plist: {}", inner)),
    }
}

fn audit_test_conj_prop(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    let argv = list_elems(args)?;
    if argv.len() != 2 {
        return Err(format!("test-conj-prop wants 2 args, got {}", argv.len()));
    }
    let prop = parse_conj_prop_plist(argv[0])?;
    let forms_list = list_elems(argv[1])?;
    let forms: Vec<ConjForm> = forms_list.iter()
        .map(|f| parse_conj_form(f))
        .collect::<Result<_, _>>()?;
    let actual = test_conj_prop(&prop, &forms);
    let inner = expect_one(expected)?;
    let exp = if inner.is_t() { true } else if inner.is_nil() { false } else {
        return Err(format!("test-conj-prop expected[0] not T/NIL: {}", inner));
    };
    if actual == exp { Ok(()) } else {
        Err(format!("\n  rust: {}\n  lisp: {}", actual, exp))
    }
}

fn audit_skip_by_conj_data(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    let argv = list_elems(args)?;
    if argv.len() != 1 {
        return Err(format!("skip-by-conj-data wants 1 arg, got {}", argv.len()));
    }
    // Empty list / NIL means an empty conj-data input, not an absent argument.
    let cd_list: Vec<ConjData> = if argv[0].is_nil() {
        Vec::new()
    } else {
        list_elems(argv[0])?
            .into_iter()
            .map(parse_conj_data_plist)
            .collect::<Result<_, _>>()?
    };
    let actual = skip_by_conj_data(&cd_list);
    let inner = expect_one(expected)?;
    let exp = if inner.is_t() { true } else if inner.is_nil() { false } else {
        return Err(format!("skip-by-conj-data expected[0] not T/NIL: {}", inner));
    };
    if actual == exp { Ok(()) } else {
        Err(format!("\n  rust: {}\n  lisp: {}", actual, exp))
    }
}

fn audit_get_kana_forms_conj_data_filter(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    let argv = list_elems(args)?;
    if argv.len() != 1 {
        return Err(format!("get-kana-forms-conj-data-filter wants 1 arg, got {}", argv.len()));
    }
    let cd_list: Vec<ConjData> = if argv[0].is_nil() {
        Vec::new()
    } else {
        list_elems(argv[0])?
            .into_iter()
            .map(parse_conj_data_plist)
            .collect::<Result<_, _>>()?
    };
    let actual = get_kana_forms_conj_data_filter(&cd_list);
    let inner = expect_one(expected)?;
    let exp: Vec<i32> = if inner.is_nil() {
        Vec::new()
    } else {
        list_elems(inner)?
            .into_iter()
            .map(|s| s.as_i64()
                .map(|i| i as i32)
                .ok_or_else(|| format!("conj-id not int: {}", s)))
            .collect::<Result<_, _>>()?
    };
    if actual == exp { Ok(()) } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", actual, exp))
    }
}

fn conj_prop_eq(a: &ConjProp, b: &ConjProp) -> bool {
    a.id == b.id && a.conj_id == b.conj_id && a.conj_type == b.conj_type
        && a.pos == b.pos && a.neg == b.neg && a.fml == b.fml
}

fn compare_conj_data(a: &ConjData, e: &ConjData) -> Result<(), String> {
    let prop_eq = match (&a.prop, &e.prop) {
        (None, None) => true,
        (Some(ap), Some(ep)) => conj_prop_eq(ap, ep),
        _ => false,
    };
    if a.seq == e.seq && a.from == e.from && a.via == e.via
        && prop_eq && a.src_map == e.src_map
    {
        Ok(())
    } else {
        Err(format!("\n  rust: {}\n  lisp: {}", render_conj_data(a), render_conj_data(e)))
    }
}


fn audit_basic_split(args: &Sexp, expected: &Sexp) -> Result<(), String> {
    let argv = list_elems(args)?;
    let s = argv[0].as_str().ok_or("arg 0 not string")?;
    let actual = basic_split(s);
    // expected: (((:WORD . "x") (:MISC . ".") ...))  — outer 1-list, inner list-of-cons
    let inner = expect_one(expected)?;
    let pairs = if inner.is_nil() { Vec::new() } else { list_elems(inner)? };
    if actual.len() != pairs.len() {
        return Err(format!("len mismatch: rust {} vs lisp {}", actual.len(), pairs.len()));
    }
    for (i, (rust_pair, lisp_pair)) in actual.iter().zip(pairs.iter()).enumerate() {
        let (lcar, lcdr) = lisp_pair.as_cons()
            .ok_or_else(|| format!("pair {} not cons: {}", i, lisp_pair))?;
        let lkind = lcar.as_keyword()
            .ok_or_else(|| format!("pair {} car not keyword: {}", i, lcar))?;
        let lstr = lcdr.as_str()
            .ok_or_else(|| format!("pair {} cdr not string: {}", i, lcdr))?;
        let rust_kind = match rust_pair.0 { SegmentKind::Word => "WORD", SegmentKind::Misc => "MISC" };
        if rust_kind != lkind || rust_pair.1 != lstr {
            return Err(format!(
                "\n  pair {}:\n    rust: ({} . {:?})\n    lisp: (:{} . {:?})",
                i, rust_kind, rust_pair.1, lkind, lstr
            ));
        }
    }
    Ok(())
}


// --- driver --------------------------------------------------------------

/// Audit one parquet file. Async because some handlers issue DB
/// queries; up to [`CONCURRENCY`] rows are in flight per file via
/// [`JoinSet`] so DB-bound batches fan out across the connection pool
/// instead of serializing on a single Postgres round-trip. CPU-only
/// rows still run inline in the spawned task — `audit_one` is sync
/// for those arms, no extra runtime cost.
async fn audit_file(idx: usize, of: usize, path: &Path, totals: &mut Totals) {
    println!("--- file {}/{} : {}", idx, of, path.display());
    let file = std::fs::File::open(path).unwrap_or_else(|e| panic!("open {:?}: {}", path, e));
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)
        .unwrap_or_else(|e| panic!("parquet builder {:?}: {}", path, e));
    let metadata = builder.metadata().clone();
    let kv = metadata
        .file_metadata()
        .key_value_metadata()
        .cloned()
        .unwrap_or_default();
    let fqn_owned: Arc<str> = Arc::from(
        kv.iter()
            .find(|kv| kv.key == "ichiran_extractor_fqn")
            .and_then(|kv| kv.value.clone())
            .unwrap_or_else(|| panic!("no ichiran_extractor_fqn in {:?}", path))
            .as_str(),
    );

    let total_rows = metadata.file_metadata().num_rows() as usize;
    let reader = builder.build().expect("build reader");
    let mut n_pass = 0usize;
    let mut n_fail = 0usize;
    let mut n_done = 0usize;
    let mut last_progress_at = 0usize;
    let progress_every: usize = (total_rows / 10).max(50_000).min(200_000);
    let mut first_failures: Vec<String> = Vec::new();
    let t0 = Instant::now();
    println!("  > {:48}  total={}", fqn_owned, format_count(total_rows));

    let mut set: JoinSet<(String, Option<Result<(), String>>)> = JoinSet::new();
    let mut skip_logged = false;

    let drain_one = |
        joined: (String, Option<Result<(), String>>),
        n_pass: &mut usize, n_fail: &mut usize,
        first_failures: &mut Vec<String>,
        skip_logged: &mut bool,
        fqn: &str,
        totals: &mut Totals,
    | -> bool {
        let (args_src, res) = joined;
        match res {
            None => {
                if !*skip_logged {
                    println!("  SKIP {}  (no handler registered)", fqn);
                    totals.skipped += 1;
                    *skip_logged = true;
                }
                false
            }
            Some(Ok(())) => { *n_pass += 1; true }
            Some(Err(e)) => {
                *n_fail += 1;
                if first_failures.len() < MISMATCH_PRINT_LIMIT {
                    first_failures.push(format!("{}\n  args: {}\n  err:  {}",
                        args_src, args_src, e));
                }
                true
            }
        }
    };

    for batch in reader {
        let batch = batch.expect("batch");
        let args_col = batch.column_by_name("args").expect("args column")
            .as_any().downcast_ref::<StringArray>().expect("args is StringArray");
        let result_col = batch.column_by_name("result").expect("result column")
            .as_any().downcast_ref::<StringArray>().expect("result is StringArray");
        for i in 0..batch.num_rows() {
            // Backpressure.
            while set.len() >= CONCURRENCY {
                if let Some(joined) = set.join_next().await {
                    let pair = joined.expect("join");
                    let counted = drain_one(pair, &mut n_pass, &mut n_fail,
                        &mut first_failures, &mut skip_logged, &fqn_owned, totals);
                    if counted { n_done += 1; }
                }
            }
            let args_src = args_col.value(i).to_string();
            let result_src = result_col.value(i).to_string();
            let fqn_clone: Arc<str> = Arc::clone(&fqn_owned);
            set.spawn(async move {
                let args = match sexp::parse(&args_src) {
                    Ok(v) => v,
                    Err(e) => {
                        return (args_src.clone(),
                            Some(Err(format!("[parse-args] {}\n  args: {}", e, args_src))));
                    }
                };
                let expected = match sexp::parse(&result_src) {
                    Ok(v) => v,
                    Err(e) => {
                        return (args_src.clone(),
                            Some(Err(format!("[parse-result] {}\n  result: {}", e, result_src))));
                    }
                };
                let res = audit_one(&fqn_clone, &args, &expected).await;
                (args_src, res)
            });
            if n_done > 0 && n_done - last_progress_at >= progress_every {
                last_progress_at = n_done;
                let elapsed = t0.elapsed().as_secs_f64();
                let rate = n_done as f64 / elapsed.max(1e-9);
                let pct = if total_rows > 0 { 100.0 * n_done as f64 / total_rows as f64 } else { 0.0 };
                let eta = if rate > 0.0 && total_rows > n_done {
                    format_duration((total_rows - n_done) as f64 / rate)
                } else { "?".into() };
                println!(
                    "    [progress] total={}  current={} ({:.1}%)  rate={}/s  elapsed={}  ETA={}  fail={}",
                    format_count(total_rows),
                    format_count(n_done), pct,
                    format_count(rate as usize),
                    format_duration(elapsed),
                    eta,
                    n_fail,
                );
            }
        }
    }
    while let Some(joined) = set.join_next().await {
        let pair = joined.expect("join");
        let _ = drain_one(pair, &mut n_pass, &mut n_fail,
            &mut first_failures, &mut skip_logged, &fqn_owned, totals);
    }
    let _ = n_done; // last pre-loop progress print already emitted

    if skip_logged { return; }

    let elapsed = t0.elapsed().as_secs_f64();
    let total = n_pass + n_fail;
    let rate = total as f64 / elapsed.max(1e-9);
    let pct = if total > 0 { 100.0 * n_pass as f64 / total as f64 } else { 0.0 };
    let tag = if n_fail == 0 { "PASS" } else { "FAIL" };
    println!(
        "  {} {:48} pass={:>7}  fail={:>7}  ({:>6.2}%, {:>6.0} rows/s, {:>5.2}s)",
        tag, fqn_owned, n_pass, n_fail, pct, rate, elapsed,
    );
    if !first_failures.is_empty() {
        for (i, f) in first_failures.iter().enumerate() {
            println!("    [{}] {}", i + 1, f);
        }
    }

    totals.pass += n_pass;
    totals.fail += n_fail;
    if n_fail == 0 { totals.fns_clean += 1; } else { totals.fns_with_failures += 1; }
}

#[derive(Default)]
struct Totals {
    pass: usize,
    fail: usize,
    skipped: usize,
    fns_clean: usize,
    fns_with_failures: usize,
}

fn format_count(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 { out.push(','); }
        out.push(c);
    }
    out.chars().rev().collect()
}

fn format_duration(seconds: f64) -> String {
    let total = seconds as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{}h{:02}m{:02}s", h, m, s)
    } else if m > 0 {
        format!("{}m{:02}s", m, s)
    } else {
        format!("{:.1}s", seconds)
    }
}

fn discover_parquets(arg: &str) -> Vec<PathBuf> {
    let p = Path::new(arg);
    if p.is_file() {
        return vec![p.to_path_buf()];
    }
    let mut out = Vec::new();
    if p.is_dir() {
        for entry in std::fs::read_dir(p).expect("read_dir") {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            if path.is_dir() {
                out.extend(discover_parquets(path.to_str().expect("utf-8 path")));
            } else if path.extension().and_then(|s| s.to_str()) == Some("parquet") {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

#[tokio::main]
async fn main() {
    let arg = std::env::args().nth(1)
        .unwrap_or_else(|| "corpus/extracted/characters".to_string());
    let parquets = discover_parquets(&arg);
    if parquets.is_empty() {
        eprintln!("no .parquet files at {}", arg);
        std::process::exit(2);
    }

    // Build a KaniranContext using the layered config (kaniran.toml +
    // env). `from_env` runs every cache populator before returning,
    // so the handlers see fully populated caches without further
    // setup. A `MissingConnection` error means no source supplied a
    // URL — skip the build silently and let DB-dependent handlers
    // report "no context". CPU-only fixture sets (characters/*,
    // numbers/*, etc.) audit fine without it.
    match KaniranContext::from_env().await {
        Ok(ctx) => { let _ = AUDIT_CTX.set(ctx); }
        Err(kaniran_core::conn::kani_context::Error::MissingConnection(_)) => {
            eprintln!("note: DATABASE_URL unset — DB-backed handlers will report \"no context\"");
        }
        Err(e) => eprintln!("warning: KaniranContext::from_env failed: {e}"),
    }

    let mut totals = Totals::default();
    let t0 = Instant::now();

    println!("=== auditing {} parquet file(s) ===\n", parquets.len());
    for (i, path) in parquets.iter().enumerate() {
        audit_file(i + 1, parquets.len(), path, &mut totals).await;
        let so_far = totals.pass + totals.fail;
        let elapsed = t0.elapsed().as_secs_f64();
        let rate = so_far as f64 / elapsed.max(1e-9);
        println!(
            "    [overall] files={}/{}  rows={}  pass={}  fail={}  rate={}/s  elapsed={}\n",
            i + 1, parquets.len(),
            format_count(so_far),
            format_count(totals.pass),
            format_count(totals.fail),
            format_count(rate as usize),
            format_duration(elapsed),
        );
    }

    let elapsed = t0.elapsed().as_secs_f64();
    let total_rows = totals.pass + totals.fail;
    println!();
    println!("=== summary ===");
    println!("  parquet files:    {}", parquets.len());
    println!("  fns clean:        {}", totals.fns_clean);
    println!("  fns w/ failures:  {}", totals.fns_with_failures);
    println!("  fns skipped:      {}", totals.skipped);
    println!("  rows audited:     {}", total_rows);
    println!("  rows passed:      {} ({:.3}%)",
             totals.pass,
             100.0 * totals.pass as f64 / total_rows.max(1) as f64);
    println!("  rows failed:      {}", totals.fail);
    println!("  wall clock:       {:.1}s ({:.0} rows/s)",
             elapsed, total_rows as f64 / elapsed.max(1e-9));

    if totals.fail > 0 {
        std::process::exit(1);
    }
}


// ===========================================================================
// Async DB-backed handlers (merged from former audit_dict_fixtures.rs).
//
// Helpers below collide on `parse_kanji_text` / `parse_kana_text` /
// `parse_proxy_text` / `parse_compound_text` with the lenient sync
// versions earlier in the file. The lenient versions ignore
// `:CONJUGATIONS` (their handlers don't read `state.conjugations`);
// the strict versions below — used by `word-conj-data` and the
// `get-kana-form*` audits — require it as a freshness check that
// the fixture was extracted under the post-2026-05-08 projector
// patch. Renamed `*_strict` to coexist.
// ===========================================================================

fn expect_first_of_one_or_two(expected: &Sexp) -> Result<&Sexp, String> {
    let elems = list_elems(expected)?;
    if elems.is_empty() || elems.len() > 2 {
        return Err(format!(
            "expected 1- or 2-element list (multi-val wrap), got {}", expected,
        ));
    }
    Ok(elems[0])
}

fn parse_int(s: &Sexp) -> Result<i32, String> {
    s.as_i64()
        .ok_or_else(|| format!("not an int: {}", s))
        .map(|n| n as i32)
}


// --- projected DAO row + compare ---------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
struct DaoRow {
    class: String,
    /// `None` when the captured plist had `:ID NIL`. Upstream cache-path
    /// rows carry NIL because `kana-text` / `kanji-text` declare `id`
    /// without `:initarg :id` (`dict.lisp:87,129`); `find-word`'s
    /// `(apply 'make-instance recipe)` therefore leaves the slot unbound
    /// when serving from `*substring-hash*`. The Rust port always hits
    /// the DB and so always has an integer id; comparison treats Lisp's
    /// NIL as a wildcard against any Rust id (see `dao_row_eq`).
    id: Option<i32>,
    seq: i32,
    text: String,
    ord: i32,
}

fn parse_dao_plist(s: &Sexp) -> Result<DaoRow, String> {
    let elems = list_elems(s)?;
    if elems.len() % 2 != 0 {
        return Err(format!("plist has odd element count: {}", s));
    }
    let mut class = None;
    let mut id: Option<Option<i32>> = None;
    let mut seq = None;
    let mut text = None;
    let mut ord = None;
    for pair in elems.chunks(2) {
        let k = pair[0].as_keyword()
            .ok_or_else(|| format!("plist key not keyword: {}", pair[0]))?;
        let v = pair[1];
        match k {
            "CLASS" => class = Some(
                v.as_keyword()
                    .ok_or_else(|| format!(":CLASS value not keyword: {}", v))?
                    .to_string(),
            ),
            "ID"   => id   = Some(if v.is_nil() { None } else { Some(parse_int(v)?) }),
            "SEQ"  => seq  = Some(parse_int(v)?),
            "TEXT" => text = Some(
                v.as_str()
                    .ok_or_else(|| format!(":TEXT value not string: {}", v))?
                    .to_string(),
            ),
            "ORD"  => ord  = Some(parse_int(v)?),
            _ => {}
        }
    }
    Ok(DaoRow {
        class: class.ok_or(":CLASS missing")?,
        id:    id.ok_or(":ID missing")?,
        seq:   seq.ok_or(":SEQ missing")?,
        text:  text.ok_or(":TEXT missing")?,
        ord:   ord.ok_or(":ORD missing")?,
    })
}

fn dao_from_kana(r: &KanaText) -> DaoRow {
    DaoRow {
        class: "KANA-TEXT".into(),
        id: Some(r.id), seq: r.seq, text: r.text.clone(), ord: r.ord,
    }
}

fn dao_from_kanji(r: &KanjiText) -> DaoRow {
    DaoRow {
        class: "KANJI-TEXT".into(),
        id: Some(r.id), seq: r.seq, text: r.text.clone(), ord: r.ord,
    }
}

fn project_word_seq(rows: &WordSeqRows) -> Vec<DaoRow> {
    match rows {
        WordSeqRows::Kana(v)  => v.iter().map(dao_from_kana).collect(),
        WordSeqRows::Kanji(v) => v.iter().map(dao_from_kanji).collect(),
    }
}

fn dao_row_eq(a: &DaoRow, e: &DaoRow) -> bool {
    a.class == e.class && a.seq == e.seq && a.text == e.text && a.ord == e.ord
        && match (a.id, e.id) {
            (Some(x), Some(y)) => x == y,
            _ => true,
        }
}

fn compare_dao_lists(mut actual: Vec<DaoRow>, expected: &Sexp) -> Result<(), String> {
    let exp_elems = list_elems(expected)?;
    let mut expected_rows: Vec<DaoRow> = exp_elems.iter()
        .map(|e| parse_dao_plist(e))
        .collect::<Result<_, _>>()?;
    if actual.len() != expected_rows.len() {
        return Err(format!(
            "row count: rust={} lisp={}\n  rust: {:?}\n  lisp: {:?}",
            actual.len(), expected_rows.len(), actual, expected_rows,
        ));
    }
    let key = |r: &DaoRow| (r.seq, r.text.clone(), r.ord, r.class.clone());
    actual.sort_by_key(key);
    expected_rows.sort_by_key(key);
    for (i, (a, e)) in actual.iter().zip(&expected_rows).enumerate() {
        if !dao_row_eq(a, e) {
            return Err(format!("row {}: rust={:?} lisp={:?}", i, a, e));
        }
    }
    Ok(())
}


// --- per-FQN async handlers --------------------------------------------

async fn audit_find_word_seq(
    ctx: &KaniranContext, args: &Sexp, expected: &Sexp,
) -> Result<(), String> {
    let argv = list_elems(args)?;
    if argv.is_empty() { return Err("args empty (need at least word)".into()); }
    let word = argv[0].as_str().ok_or("arg 0 not string")?;
    let seqs: Vec<i32> = argv[1..].iter()
        .map(|s| parse_int(s))
        .collect::<Result<_, _>>()?;
    let actual = find_word_seq(ctx, word, &seqs).await
        .map_err(|e| format!("find_word_seq query: {}", e))?;
    compare_dao_lists(project_word_seq(&actual), expect_one(expected)?)
}

async fn audit_find_word_conj_of(
    ctx: &KaniranContext, args: &Sexp, expected: &Sexp,
) -> Result<(), String> {
    let argv = list_elems(args)?;
    if argv.is_empty() { return Err("args empty (need at least word)".into()); }
    let word = argv[0].as_str().ok_or("arg 0 not string")?;
    let seqs: Vec<i32> = argv[1..].iter()
        .map(|s| parse_int(s))
        .collect::<Result<_, _>>()?;
    let actual = find_word_conj_of(ctx, word, &seqs).await
        .map_err(|e| format!("find_word_conj_of query: {}", e))?;
    compare_dao_lists(project_word_seq(&actual), expect_one(expected)?)
}


// --- get-split / get-split* --------------------------------------------

fn parse_get_split_args(args: &Sexp)
    -> Result<(KaniSimpleTextDispatchEnum, Vec<i32>), String>
{
    let argv = list_elems(args)?;
    if argv.len() != 2 {
        return Err(format!("expected 2 args (reading conj-of), got {}", argv.len()));
    }
    let reading = parse_simple_text_plist(argv[0])?;
    let conj_of: Vec<i32> = if argv[1].is_nil() {
        Vec::new()
    } else {
        list_elems(argv[1])?.iter()
            .map(|s| parse_int(s))
            .collect::<Result<_, _>>()?
    };
    Ok((reading, conj_of))
}

/// Compare a single Rust `SplitPart` against its captured Lisp form.
/// Words are projected to [`DaoRow`] (re-using the find-word-seq
/// comparison rule: NIL `:ID` is wildcard); `:SCORE` / `:PSCORE`
/// keywords pass through their Rust enum variants.
fn compare_split_part_word(actual: &SplitPart, expected: &Sexp) -> Result<(), String> {
    if let Some(kw) = expected.as_keyword() {
        return match (kw, actual) {
            ("SCORE",  SplitPart::Score)  => Ok(()),
            ("PSCORE", SplitPart::PScore) => Ok(()),
            (k, _) => Err(format!("expected :{}, got {:?}", k, actual)),
        };
    }
    let exp_dao = parse_dao_plist(expected)?;
    let act_dao = match actual {
        SplitPart::Word(KaniWordDispatchEnum::Kana(k))  => dao_from_kana(k),
        SplitPart::Word(KaniWordDispatchEnum::Kanji(k)) => dao_from_kanji(k),
        SplitPart::Word(other) =>
            return Err(format!("non-simple word in parts: {:?}", other)),
        SplitPart::Score  => return Err("expected DAO, got SplitPart::Score".into()),
        SplitPart::PScore => return Err("expected DAO, got SplitPart::PScore".into()),
    };
    if !dao_row_eq(&act_dao, &exp_dao) {
        return Err(format!("rust={:?} lisp={:?}", act_dao, exp_dao));
    }
    Ok(())
}

/// Same as [`compare_split_part_word`] but accepts NIL as a captured
/// failed-lookup placeholder. get-split* preserves NILs in the parts
/// list; get-split filters them out before returning.
fn compare_split_part_optional(actual: &Option<SplitPart>, expected: &Sexp) -> Result<(), String> {
    match (actual, expected.is_nil()) {
        (None, true)  => Ok(()),
        (None, false) => Err(format!("rust=None lisp={}", expected)),
        (Some(_), true)  => Err(format!("rust=Some({:?}) lisp=NIL", actual)),
        (Some(p), false) => compare_split_part_word(p, expected),
    }
}

async fn audit_get_split(
    ctx: &KaniranContext, args: &Sexp, expected: &Sexp,
) -> Result<(), String> {
    let (reading, conj_of) = parse_get_split_args(args)?;
    let actual = get_split(ctx, &reading, &conj_of).await
        .map_err(|e| format!("get_split query: {}", e))?;
    let exp_elems = list_elems(expected)?;
    match (&actual, exp_elems.len()) {
        (None, 1) => {
            if !exp_elems[0].is_nil() {
                return Err(format!("expected NIL, got {}", exp_elems[0]));
            }
            Ok(())
        }
        (Some((parts, score)), 2) => {
            let exp_parts = list_elems(exp_elems[0])?;
            let exp_score = parse_int(exp_elems[1])?;
            if *score != exp_score {
                return Err(format!("score: rust={} lisp={}", score, exp_score));
            }
            if parts.len() != exp_parts.len() {
                return Err(format!(
                    "parts count: rust={} lisp={}\n  rust: {:?}\n  lisp: {:?}",
                    parts.len(), exp_parts.len(), parts, exp_parts,
                ));
            }
            for (i, (act, exp)) in parts.iter().zip(exp_parts.iter()).enumerate() {
                compare_split_part_word(act, exp).map_err(|e| format!("part {}: {}", i, e))?;
            }
            Ok(())
        }
        (None, n) => Err(format!(
            "rust=None but lisp returned {}-element result: {}", n, expected,
        )),
        (Some(_), n) => Err(format!(
            "rust=Some but lisp returned {}-element result: {}", n, expected,
        )),
    }
}

async fn audit_get_split_star(
    ctx: &KaniranContext, args: &Sexp, expected: &Sexp,
) -> Result<(), String> {
    let (reading, conj_of) = parse_get_split_args(args)?;
    let actual = get_split_star_(ctx, &reading, &conj_of).await
        .map_err(|e| format!("get_split_star query: {}", e))?;
    let exp_elems = list_elems(expected)?;
    match (&actual, exp_elems.len()) {
        (None, 1) => {
            if !exp_elems[0].is_nil() {
                return Err(format!("expected NIL, got {}", exp_elems[0]));
            }
            Ok(())
        }
        (Some((parts, score)), 2) => {
            let exp_parts = list_elems(exp_elems[0])?;
            let exp_score = parse_int(exp_elems[1])?;
            if *score != exp_score {
                return Err(format!("score: rust={} lisp={}", score, exp_score));
            }
            if parts.len() != exp_parts.len() {
                return Err(format!(
                    "parts count: rust={} lisp={}\n  rust: {:?}\n  lisp: {:?}",
                    parts.len(), exp_parts.len(), parts, exp_parts,
                ));
            }
            for (i, (act, exp)) in parts.iter().zip(exp_parts.iter()).enumerate() {
                compare_split_part_optional(act, exp).map_err(|e| format!("part {}: {}", i, e))?;
            }
            Ok(())
        }
        (None, n) => Err(format!(
            "rust=None but lisp returned {}-element result: {}", n, expected,
        )),
        (Some(_), n) => Err(format!(
            "rust=Some but lisp returned {}-element result: {}", n, expected,
        )),
    }
}

fn project_find_word_rows(rows: &FindWordRows) -> Vec<DaoRow> {
    match rows {
        FindWordRows::Kana(v)  => v.iter().map(dao_from_kana).collect(),
        FindWordRows::Kanji(v) => v.iter().map(dao_from_kanji).collect(),
    }
}

async fn audit_find_word(
    ctx: &KaniranContext, args: &Sexp, expected: &Sexp,
) -> Result<(), String> {
    let argv = list_elems(args)?;
    if argv.is_empty() { return Err("find-word args empty".into()); }
    let word = argv[0].as_str().ok_or("arg 0 not string")?;
    let mut root_only = false;
    let mut i = 1;
    while i < argv.len() {
        let key = argv[i].as_keyword()
            .ok_or_else(|| format!("expected keyword at idx {}: {}", i, argv[i]))?;
        if i + 1 >= argv.len() {
            return Err(format!("keyword :{} missing value", key));
        }
        let v = argv[i + 1];
        match key {
            "ROOT-ONLY" => root_only = v.is_t(),
            other => return Err(format!("find-word: unknown keyword :{}", other)),
        }
        i += 2;
    }
    let actual = find_word(ctx, word, root_only, None).await
        .map_err(|e| format!("find_word query: {}", e))?;
    compare_dao_lists(project_find_word_rows(&actual), expect_first_of_one_or_two(expected)?)
}


// --- find-word-as-hiragana: proxy-text rows comparison ------------------

#[derive(Debug, PartialEq, Eq)]
struct ProxyDao {
    text: String,
    kana: String,
    source: DaoRow,
}

fn parse_proxy_plist(s: &Sexp) -> Result<ProxyDao, String> {
    let elems = list_elems(s)?;
    if elems.len() % 2 != 0 {
        return Err(format!("proxy plist odd length: {}", s));
    }
    let mut text = None;
    let mut kana = None;
    let mut source = None;
    let mut class_ok = false;
    for pair in elems.chunks(2) {
        let k = pair[0].as_keyword()
            .ok_or_else(|| format!("proxy key not keyword: {}", pair[0]))?;
        let v = pair[1];
        match k {
            "CLASS" => {
                if v.as_keyword() != Some("PROXY-TEXT") {
                    return Err(format!(":CLASS not :PROXY-TEXT: {}", v));
                }
                class_ok = true;
            }
            "TEXT" => text = Some(v.as_str().ok_or(":TEXT not string")?.to_string()),
            "KANA" => kana = Some(v.as_str().ok_or(":KANA not string")?.to_string()),
            "SOURCE" => source = Some(parse_dao_plist(v)?),
            "CONJUGATIONS" | "HINTEDP" => {}
            other => return Err(format!("unknown proxy key: :{}", other)),
        }
    }
    if !class_ok { return Err(":CLASS missing on proxy-text".into()); }
    Ok(ProxyDao {
        text: text.ok_or(":TEXT missing")?,
        kana: kana.ok_or(":KANA missing")?,
        source: source.ok_or(":SOURCE missing")?,
    })
}

fn dao_from_simple(s: &KaniSimpleTextDispatchEnum) -> Result<DaoRow, String> {
    match s {
        KaniSimpleTextDispatchEnum::Kana(k)  => Ok(dao_from_kana(k)),
        KaniSimpleTextDispatchEnum::Kanji(k) => Ok(dao_from_kanji(k)),
        KaniSimpleTextDispatchEnum::Proxy(_) =>
            Err("unexpected nested proxy under find-word-as-hiragana".into()),
    }
}

fn proxy_dao_from(p: &ProxyText) -> Result<ProxyDao, String> {
    Ok(ProxyDao {
        text: p.text.clone(),
        kana: p.kana.clone(),
        source: dao_from_simple(&p.source)?,
    })
}

fn compare_proxy_lists(
    mut actual: Vec<ProxyDao>, expected: &Sexp,
) -> Result<(), String> {
    let exp_elems = if expected.is_nil() { Vec::new() } else { list_elems(expected)? };
    let mut expected_rows: Vec<ProxyDao> = exp_elems.iter()
        .map(|e| parse_proxy_plist(e))
        .collect::<Result<_, _>>()?;
    if actual.len() != expected_rows.len() {
        return Err(format!(
            "row count: rust={} lisp={}\n  rust: {:?}\n  lisp: {:?}",
            actual.len(), expected_rows.len(), actual, expected_rows,
        ));
    }
    actual.sort_by_key(|r| (r.source.id, r.source.class.clone()));
    expected_rows.sort_by_key(|r| (r.source.id, r.source.class.clone()));
    for (i, (a, e)) in actual.iter().zip(&expected_rows).enumerate() {
        if a != e {
            return Err(format!("row {}: rust={:?} lisp={:?}", i, a, e));
        }
    }
    Ok(())
}

async fn audit_find_word_as_hiragana(
    ctx: &KaniranContext, args: &Sexp, expected: &Sexp,
) -> Result<(), String> {
    let argv = list_elems(args)?;
    if argv.is_empty() { return Err("find-word-as-hiragana args empty".into()); }
    let str_ = argv[0].as_str().ok_or("arg 0 not string")?;
    let mut exclude: Vec<i32> = Vec::new();
    let mut finder_present_non_nil = false;
    let mut i = 1;
    while i < argv.len() {
        let key = argv[i].as_keyword()
            .ok_or_else(|| format!("expected keyword at idx {}: {}", i, argv[i]))?;
        if i + 1 >= argv.len() {
            return Err(format!("keyword :{} missing value", key));
        }
        let v = argv[i + 1];
        match key {
            "EXCLUDE" => {
                if !v.is_nil() {
                    for e in list_elems(v)? {
                        exclude.push(parse_int(e)?);
                    }
                }
            }
            "FINDER" => finder_present_non_nil = !v.is_nil(),
            other => return Err(format!("find-word-as-hiragana: unknown keyword :{}", other)),
        }
        i += 2;
    }
    if finder_present_non_nil {
        return Err(":FINDER non-nil — audit harness has no replay strategy for closures".into());
    }
    let actual = find_word_as_hiragana(ctx, str_, &exclude, None).await
        .map_err(|e| format!("find_word_as_hiragana query: {}", e))?;
    let actual_proxies: Vec<ProxyDao> = actual.iter()
        .map(proxy_dao_from)
        .collect::<Result<_, _>>()?;
    compare_proxy_lists(actual_proxies, expect_one(expected)?)
}


// --- get-conj-data: DB-backed conj-data list audit ---------------------
//
// `parse_bool_or_dbnull`, `parse_int_or_nil`, `parse_conj_prop_plist`,
// and `parse_conj_data_plist` are reused from the sync helpers earlier
// in the file (used by the `conj-data-prop` / `make-conj-data` audits).

fn conj_data_eq(a: &ConjData, b: &ConjData) -> bool {
    let prop_eq = match (&a.prop, &b.prop) {
        (None, None) => true,
        (Some(p), Some(q)) => conj_prop_eq(p, q),
        _ => false,
    };
    let mut a_src = a.src_map.clone();
    let mut b_src = b.src_map.clone();
    a_src.sort();
    b_src.sort();
    a.seq == b.seq && a.from == b.from && a.via == b.via
        && prop_eq && a_src == b_src
}

fn parse_from_or_conj_ids(s: &Sexp) -> Result<FromOrConjIds, String> {
    if s.is_nil() { return Ok(FromOrConjIds::All); }
    if s.as_keyword() == Some("ROOT") { return Ok(FromOrConjIds::Root); }
    if let Some(n) = s.as_i64() { return Ok(FromOrConjIds::From(n as i32)); }
    if s.is_list() {
        let elems = list_elems(s)?;
        let ids: Vec<i32> = elems.iter().map(|e| parse_int(e)).collect::<Result<_, _>>()?;
        return Ok(FromOrConjIds::ConjIds(ids));
    }
    Err(format!("can't classify from/conj-ids: {}", s))
}

async fn audit_get_conj_data(
    ctx: &KaniranContext, args: &Sexp, expected: &Sexp,
) -> Result<(), String> {
    let argv = list_elems(args)?;
    if argv.is_empty() { return Err("get-conj-data needs at least seq".into()); }
    let seq = parse_int(argv[0])?;
    let from_or = if argv.len() >= 2 {
        parse_from_or_conj_ids(argv[1])?
    } else {
        FromOrConjIds::All
    };
    let mut texts_owned: Vec<String> = Vec::new();
    if argv.len() >= 3 {
        let t = argv[2];
        if t.is_nil() {
        } else if let Some(s) = t.as_str() {
            texts_owned.push(s.to_string());
        } else if t.is_list() {
            for e in list_elems(t)? {
                texts_owned.push(e.as_str().ok_or("texts list elem not string")?.to_string());
            }
        } else {
            return Err(format!("texts not nil/string/list: {}", t));
        }
    }
    let texts: Vec<&str> = texts_owned.iter().map(String::as_str).collect();
    let actual = get_conj_data(ctx, seq, from_or, &texts).await
        .map_err(|e| format!("get_conj_data query: {}", e))?;
    let inner = expect_one(expected)?;
    let exp_elems = if inner.is_nil() { Vec::new() } else { list_elems(inner)? };
    let mut expected_rows: Vec<ConjData> = exp_elems.iter()
        .map(|e| parse_conj_data_plist(e))
        .collect::<Result<_, _>>()?;
    if actual.len() != expected_rows.len() {
        return Err(format!("conj-data count: rust={} lisp={}", actual.len(), expected_rows.len()));
    }
    let mut actual_sorted = actual;
    let key = |c: &ConjData| (
        c.seq.unwrap_or(0), c.from.unwrap_or(0), c.via.unwrap_or(0),
        c.prop.as_ref().map(|p| p.id).unwrap_or(0),
    );
    actual_sorted.sort_by_key(key);
    expected_rows.sort_by_key(key);
    for (i, (a, e)) in actual_sorted.iter().zip(&expected_rows).enumerate() {
        if !conj_data_eq(a, e) {
            return Err(format!("row {}: rust seq={:?} from={:?} via={:?} prop_id={:?}\n         lisp seq={:?} from={:?} via={:?} prop_id={:?}",
                i, a.seq, a.from, a.via, a.prop.as_ref().map(|p| p.id),
                e.seq, e.from, e.via, e.prop.as_ref().map(|p| p.id)));
        }
    }
    Ok(())
}


// --- word-conj-data: build a KaniWordDispatchEnum from the args plist --

/// Decode an `:ID NIL` integer-or-NIL slot to `i32`. Captured plists
/// emit `NIL` for synthesized rows (no DB id); audit-side word_conj_data
/// never reads the id, so we substitute 0.
fn parse_int_or_nil_default(s: &Sexp) -> Result<i32, String> {
    if s.is_nil() { return Ok(0); }
    s.as_i64().map(|n| n as i32).ok_or_else(|| format!("not int/nil: {}", s))
}

fn parse_string_or_null(s: &Sexp) -> Result<Option<String>, String> {
    if s.as_keyword() == Some("NULL") { return Ok(None); }
    if s.is_nil() { return Ok(None); }
    Ok(Some(s.as_str().ok_or_else(|| format!("not string/:NULL: {}", s))?.to_string()))
}

fn parse_word_to_dispatch(s: &Sexp) -> Result<KaniWordDispatchEnum, String> {
    let elems = list_elems(s)?;
    if elems.is_empty() {
        return Err("empty word plist".into());
    }
    let mut class: Option<&str> = None;
    for pair in elems.chunks(2) {
        if let Some("CLASS") = pair[0].as_keyword() {
            class = pair[1].as_keyword();
            break;
        }
    }
    let class = class.ok_or_else(|| format!(":CLASS missing on word plist: {}", s))?;
    match class {
        "KANJI-TEXT" => Ok(KaniWordDispatchEnum::Kanji(parse_kanji_text_strict(&elems)?)),
        "KANA-TEXT"  => Ok(KaniWordDispatchEnum::Kana(parse_kana_text_strict(&elems)?)),
        "PROXY-TEXT" => Ok(KaniWordDispatchEnum::Proxy(parse_proxy_text_strict(&elems)?)),
        "COMPOUND-TEXT" => Ok(KaniWordDispatchEnum::Compound(parse_compound_text_strict(&elems)?)),
        "COUNTER-TEXT" | "NUMBER-TEXT" | "COUNTER-TSU" | "COUNTER-HIFUMI"
        | "COUNTER-DAYS-KUN" | "COUNTER-DAYS-ON" | "COUNTER-MONTHS"
        | "COUNTER-PEOPLE" | "COUNTER-AGE" | "COUNTER-WARI" | "COUNTER-HALFHOUR"
            => Ok(KaniWordDispatchEnum::Counter(stub_counter_simple())),
        other => Err(format!("unknown :CLASS for word: {}", other)),
    }
}

struct SimpleTextSlots {
    seq: i32,
    text: String,
    conjugations: Option<WordConjugations>,
}

fn parse_simple_text_slots(elems: &[&Sexp]) -> Result<SimpleTextSlots, String> {
    let mut seq = None;
    let mut text = None;
    let mut conjugations = None;
    let mut conjugations_seen = false;
    for pair in elems.chunks(2) {
        let k = pair[0].as_keyword()
            .ok_or_else(|| format!("simple-text key not keyword: {}", pair[0]))?;
        let v = pair[1];
        match k {
            "CLASS" | "ID" | "ORD" | "COMMON" | "COMMON-TAGS"
            | "CONJUGATE-P" | "NOKANJI" | "BEST-KANJI" | "BEST-KANA"
            | "HINTEDP" => {}
            "SEQ"  => seq = Some(parse_int(v)?),
            "TEXT" => text = Some(v.as_str().ok_or(":TEXT not string")?.to_string()),
            "CONJUGATIONS" => {
                conjugations = parse_conjugations_audit(v)?;
                conjugations_seen = true;
            }
            other => return Err(format!("unknown simple-text key: :{}", other)),
        }
    }
    if !conjugations_seen {
        return Err("simple-text plist missing :CONJUGATIONS — re-extract under projector patch".into());
    }
    Ok(SimpleTextSlots {
        seq: seq.ok_or(":SEQ missing")?,
        text: text.ok_or(":TEXT missing")?,
        conjugations,
    })
}

fn parse_conjugations_audit(s: &Sexp) -> Result<Option<WordConjugations>, String> {
    if s.is_nil() { return Ok(None); }
    if s.as_keyword() == Some("ROOT") { return Ok(Some(WordConjugations::Root)); }
    if s.is_list() {
        let elems = list_elems(s)?;
        let ids: Vec<i32> = elems.iter().map(|e| parse_int(e)).collect::<Result<_, _>>()?;
        return Ok(Some(WordConjugations::Ids(ids)));
    }
    Err(format!(":CONJUGATIONS: expected NIL/:ROOT/(int...), got {}", s))
}

fn parse_kanji_text_strict(elems: &[&Sexp]) -> Result<KanjiText, String> {
    let s = parse_simple_text_slots(elems)?;
    Ok(KanjiText {
        id: 0, seq: s.seq, text: s.text, ord: 0,
        common: None, common_tags: String::new(), conjugate_p: false,
        nokanji: false, best_kana: None,
        state: SimpleText { conjugations: s.conjugations, hintedp: false },
    })
}

fn parse_kana_text_strict(elems: &[&Sexp]) -> Result<KanaText, String> {
    let s = parse_simple_text_slots(elems)?;
    Ok(KanaText {
        id: 0, seq: s.seq, text: s.text, ord: 0,
        common: None, common_tags: String::new(), conjugate_p: false,
        nokanji: false, best_kanji: None,
        state: SimpleText { conjugations: s.conjugations, hintedp: false },
    })
}

fn parse_proxy_text_strict(elems: &[&Sexp]) -> Result<ProxyText, String> {
    let mut text = None;
    let mut kana = None;
    let mut source = None;
    let mut conjugations = None;
    for pair in elems.chunks(2) {
        let k = pair[0].as_keyword()
            .ok_or_else(|| format!("proxy-text key not keyword: {}", pair[0]))?;
        let v = pair[1];
        match k {
            "CLASS" | "HINTEDP" => {}
            "TEXT" => text = Some(v.as_str().ok_or(":TEXT not string")?.to_string()),
            "KANA" => kana = Some(v.as_str().ok_or(":KANA not string")?.to_string()),
            "CONJUGATIONS" => conjugations = parse_conjugations_audit(v)?,
            "SOURCE" => source = Some(parse_simple_text_dispatch_strict(v)?),
            other => return Err(format!("unknown proxy-text key: :{}", other)),
        }
    }
    Ok(ProxyText {
        text: text.ok_or(":TEXT missing on proxy-text")?,
        kana: kana.ok_or(":KANA missing on proxy-text")?,
        source: Box::new(source.ok_or(":SOURCE missing on proxy-text")?),
        state: SimpleText { conjugations, hintedp: false },
    })
}

fn parse_simple_text_dispatch_strict(s: &Sexp) -> Result<KaniSimpleTextDispatchEnum, String> {
    let elems = list_elems(s)?;
    let mut class: Option<&str> = None;
    for pair in elems.chunks(2) {
        if let Some("CLASS") = pair[0].as_keyword() {
            class = pair[1].as_keyword();
            break;
        }
    }
    match class {
        Some("KANJI-TEXT") => Ok(KaniSimpleTextDispatchEnum::Kanji(parse_kanji_text_strict(&elems)?)),
        Some("KANA-TEXT")  => Ok(KaniSimpleTextDispatchEnum::Kana(parse_kana_text_strict(&elems)?)),
        Some("PROXY-TEXT") => Ok(KaniSimpleTextDispatchEnum::Proxy(parse_proxy_text_strict(&elems)?)),
        Some(other) => Err(format!("proxy :SOURCE not simple-text: :{}", other)),
        None => Err(format!("proxy :SOURCE missing :CLASS: {}", s)),
    }
}

fn parse_compound_text_strict(elems: &[&Sexp]) -> Result<CompoundText, String> {
    let mut text = None;
    let mut kana = None;
    let mut primary: Option<KaniWordDispatchEnum> = None;
    let mut words: Option<Vec<KaniWordDispatchEnum>> = None;
    for pair in elems.chunks(2) {
        let k = pair[0].as_keyword()
            .ok_or_else(|| format!("compound-text key not keyword: {}", pair[0]))?;
        let v = pair[1];
        match k {
            "CLASS" | "SCORE-BASE" | "SCORE-MOD" => {}
            "TEXT" => text = Some(v.as_str().ok_or(":TEXT not string")?.to_string()),
            "KANA" => kana = Some(v.as_str().ok_or(":KANA not string")?.to_string()),
            "PRIMARY" => primary = Some(parse_word_to_dispatch(v)?),
            "WORDS" => {
                let ws: Vec<KaniWordDispatchEnum> = list_elems(v)?
                    .iter()
                    .map(|e| parse_word_to_dispatch(e))
                    .collect::<Result<_, _>>()?;
                words = Some(ws);
            }
            other => return Err(format!("unknown compound-text key: :{}", other)),
        }
    }
    let words = words.ok_or(":WORDS missing on compound-text")?;
    if words.is_empty() {
        return Err(":WORDS empty on compound-text".into());
    }
    let primary = primary.unwrap_or_else(|| words[0].clone());
    Ok(CompoundText {
        text: text.ok_or(":TEXT missing on compound-text")?,
        kana: kana.ok_or(":KANA missing on compound-text")?,
        primary: Box::new(primary),
        words,
        score_base: None,
        score_mod: ScoreMod::Single(0),
    })
}

/// Word-dispatch counter stub for handlers that don't read counter
/// slots (`word-conj-data` returns nil for the whole counter family).
/// Distinct from the earlier `stub_counter(digit_opts, foreign)`
/// which the `counter-join` audit uses to populate the slots that
/// handler actually reads.
fn stub_counter_simple() -> Counter {
    Counter::Base(CounterText {
        text: String::new(),
        kana: String::new(),
        number_text: String::new(),
        number: 0,
        source: None,
        ordinalp: false,
        suffix: None,
        accepts_suffixes: Vec::new(),
        suffix_descriptions: Vec::new(),
        digit_opts: Vec::new(),
        common: Common::Inherit,
        allowed: Vec::new(),
        foreign: false,
    })
}

#[allow(dead_code)]
fn _audit_word_conj_data_helpers_keep(s: &Sexp) -> Result<(i32, Option<String>), String> {
    Ok((parse_int_or_nil_default(s)?, parse_string_or_null(s)?))
}

async fn audit_word_conj_data(
    ctx: &KaniranContext, args: &Sexp, expected: &Sexp,
) -> Result<(), String> {
    let argv = list_elems(args)?;
    if argv.len() != 1 {
        return Err(format!("word-conj-data wants exactly 1 arg, got {}", argv.len()));
    }
    let word = parse_word_to_dispatch(argv[0])?;
    let actual = word_conj_data(ctx, &word).await
        .map_err(|e| format!("word_conj_data query: {}", e))?;
    let inner = expect_one(expected)?;
    let exp_elems = if inner.is_nil() { Vec::new() } else { list_elems(inner)? };
    let mut expected_rows: Vec<ConjData> = exp_elems.iter()
        .map(|e| parse_conj_data_plist(e))
        .collect::<Result<_, _>>()?;
    if actual.len() != expected_rows.len() {
        return Err(format!("conj-data count: rust={} lisp={}", actual.len(), expected_rows.len()));
    }
    let mut actual_sorted = actual;
    let key = |c: &ConjData| (
        c.seq.unwrap_or(0), c.from.unwrap_or(0), c.via.unwrap_or(0),
        c.prop.as_ref().map(|p| p.id).unwrap_or(0),
    );
    actual_sorted.sort_by_key(key);
    expected_rows.sort_by_key(key);
    for (i, (a, e)) in actual_sorted.iter().zip(&expected_rows).enumerate() {
        if !conj_data_eq(a, e) {
            return Err(format!("row {}: rust seq={:?} from={:?} via={:?} prop_id={:?}\n         lisp seq={:?} from={:?} via={:?} prop_id={:?}",
                i, a.seq, a.from, a.via, a.prop.as_ref().map(|p| p.id),
                e.seq, e.from, e.via, e.prop.as_ref().map(|p| p.id)));
        }
    }
    Ok(())
}

async fn audit_get_kanji_kana_old(
    ctx: &KaniranContext, args: &Sexp, expected: &Sexp,
) -> Result<(), String> {
    let argv = list_elems(args)?;
    if argv.len() != 1 {
        return Err(format!("get-kanji-kana-old wants 1 arg, got {}", argv.len()));
    }
    let elems = list_elems(argv[0])?;
    let mut class: Option<&str> = None;
    for pair in elems.chunks(2) {
        if let Some("CLASS") = pair[0].as_keyword() {
            class = pair[1].as_keyword();
            break;
        }
    }
    if class != Some("KANJI-TEXT") {
        return Err(format!("get-kanji-kana-old expects KANJI-TEXT, got :{:?}", class));
    }
    let kt = parse_kanji_text_strict(&elems)?;
    let actual = get_kanji_kana_old(ctx, &kt).await
        .map_err(|e| format!("get_kanji_kana_old query: {}", e))?;
    let exp_first = expect_one(expected)?;
    let exp = if exp_first.is_nil() {
        None
    } else {
        Some(exp_first.as_str().ok_or("expected[0] not string/nil")?.to_string())
    };
    if actual == exp { Ok(()) } else {
        Err(format!("\n  rust: {:?}\n  lisp: {:?}", actual, exp))
    }
}


// --- get-kana-forms / get-kana-forms* DAO + conjugations comparison ----

#[derive(Debug, PartialEq, Eq)]
struct KanaTextWithConj {
    id: i32,
    seq: i32,
    text: String,
    ord: i32,
    conjugations: Option<WordConjCmp>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum WordConjCmp {
    Root,
    Ids(Vec<i32>),
}

fn parse_conjugations(s: &Sexp) -> Result<Option<WordConjCmp>, String> {
    if s.is_nil() { return Ok(None); }
    if s.as_keyword() == Some("ROOT") { return Ok(Some(WordConjCmp::Root)); }
    if s.is_list() {
        let elems = list_elems(s)?;
        let ids: Vec<i32> = elems.iter().map(|e| parse_int(e)).collect::<Result<_, _>>()?;
        return Ok(Some(WordConjCmp::Ids(ids)));
    }
    Err(format!("conjugations: expected NIL/:ROOT/(int...), got {}", s))
}

fn parse_kana_text_plist(s: &Sexp) -> Result<KanaTextWithConj, String> {
    let elems = list_elems(s)?;
    if elems.len() % 2 != 0 {
        return Err(format!("kana-text plist odd length: {}", s));
    }
    let mut id = None;
    let mut seq = None;
    let mut text = None;
    let mut ord = None;
    let mut conjugations = None;
    let mut conjugations_seen = false;
    let mut class_ok = false;
    for pair in elems.chunks(2) {
        let k = pair[0].as_keyword()
            .ok_or_else(|| format!("kana-text key not keyword: {}", pair[0]))?;
        let v = pair[1];
        match k {
            "CLASS" => {
                if v.as_keyword() != Some("KANA-TEXT") {
                    return Err(format!(":CLASS not :KANA-TEXT: {}", v));
                }
                class_ok = true;
            }
            "ID"   => id   = Some(parse_int(v)?),
            "SEQ"  => seq  = Some(parse_int(v)?),
            "TEXT" => text = Some(v.as_str().ok_or(":TEXT not string")?.to_string()),
            "ORD"  => ord  = Some(parse_int(v)?),
            "CONJUGATIONS" => {
                conjugations = parse_conjugations(v)?;
                conjugations_seen = true;
            }
            "COMMON" | "COMMON-TAGS" | "CONJUGATE-P" | "NOKANJI"
                | "BEST-KANJI" | "HINTEDP" => {}
            other => return Err(format!("unknown kana-text key: :{}", other)),
        }
    }
    if !class_ok { return Err(":CLASS missing on kana-text".into()); }
    if !conjugations_seen {
        return Err("captured kana-text missing :CONJUGATIONS — re-extract under projector patch".into());
    }
    Ok(KanaTextWithConj {
        id: id.ok_or(":ID missing")?,
        seq: seq.ok_or(":SEQ missing")?,
        text: text.ok_or(":TEXT missing")?,
        ord: ord.ok_or(":ORD missing")?,
        conjugations,
    })
}

fn project_kana_text_with_conj(rows: &[KanaText]) -> Vec<KanaTextWithConj> {
    rows.iter()
        .map(|r| KanaTextWithConj {
            id: r.id,
            seq: r.seq,
            text: r.text.clone(),
            ord: r.ord,
            conjugations: r.state.conjugations.as_ref().map(|c| match c {
                WordConjugations::Root => WordConjCmp::Root,
                WordConjugations::Ids(v) => WordConjCmp::Ids(v.clone()),
            }),
        })
        .collect()
}

fn compare_kana_text_with_conj(
    mut actual: Vec<KanaTextWithConj>, expected: &Sexp,
) -> Result<(), String> {
    let exp_elems = if expected.is_nil() { Vec::new() } else { list_elems(expected)? };
    let mut expected_rows: Vec<KanaTextWithConj> = exp_elems.iter()
        .map(|e| parse_kana_text_plist(e))
        .collect::<Result<_, _>>()?;
    if actual.len() != expected_rows.len() {
        return Err(format!(
            "row count: rust={} lisp={}\n  rust ids: {:?}\n  lisp ids: {:?}",
            actual.len(), expected_rows.len(),
            actual.iter().map(|r| r.id).collect::<Vec<_>>(),
            expected_rows.iter().map(|r| r.id).collect::<Vec<_>>(),
        ));
    }
    actual.sort_by_key(|r| r.id);
    expected_rows.sort_by_key(|r| r.id);
    for (i, (a, e)) in actual.iter().zip(&expected_rows).enumerate() {
        if a.id != e.id || a.seq != e.seq || a.text != e.text || a.ord != e.ord {
            return Err(format!("row {}: rust={:?} lisp={:?}", i, a, e));
        }
        let conj_eq = match (&a.conjugations, &e.conjugations) {
            (None, None) => true,
            (Some(WordConjCmp::Root), Some(WordConjCmp::Root)) => true,
            (Some(WordConjCmp::Ids(av)), Some(WordConjCmp::Ids(ev))) => {
                let mut a_sorted = av.clone();
                let mut e_sorted = ev.clone();
                a_sorted.sort();
                e_sorted.sort();
                a_sorted == e_sorted
            }
            _ => false,
        };
        if !conj_eq {
            return Err(format!(
                "row {} (id={}) conjugations: rust={:?} lisp={:?}",
                i, a.id, a.conjugations, e.conjugations,
            ));
        }
    }
    Ok(())
}

async fn audit_get_kana_forms_star(
    ctx: &KaniranContext, args: &Sexp, expected: &Sexp,
) -> Result<(), String> {
    let argv = list_elems(args)?;
    if argv.len() != 1 { return Err(format!("get-kana-forms* wants 1 arg, got {}", argv.len())); }
    let seq = parse_int(argv[0])?;
    let actual = get_kana_forms_star_(ctx, seq).await
        .map_err(|e| format!("get_kana_forms_star query: {}", e))?;
    compare_kana_text_with_conj(project_kana_text_with_conj(&actual), expect_one(expected)?)
}

async fn audit_get_kana_forms(
    ctx: &KaniranContext, args: &Sexp, expected: &Sexp,
) -> Result<(), String> {
    let argv = list_elems(args)?;
    if argv.len() != 1 { return Err(format!("get-kana-forms wants 1 arg, got {}", argv.len())); }
    let seq = parse_int(argv[0])?;
    let actual = get_kana_forms(ctx, seq).await
        .map_err(|e| format!("get_kana_forms query: {}", e))?;
    compare_kana_text_with_conj(project_kana_text_with_conj(&actual), expect_one(expected)?)
}

async fn audit_get_kana_form(
    ctx: &KaniranContext, args: &Sexp, expected: &Sexp,
) -> Result<(), String> {
    let argv = list_elems(args)?;
    if argv.len() < 2 { return Err("args want at least (seq text)".into()); }
    let seq = parse_int(argv[0])?;
    let text = argv[1].as_str().ok_or("arg 1 not string")?;
    let actual = get_kana_form(ctx, seq, text, None).await
        .map_err(|e| format!("get_kana_form query: {}", e))?;
    let inner = expect_one(expected)?;
    let actual_row = actual.as_ref().map(dao_from_kana);
    match (actual_row, inner.is_nil()) {
        (None, true) => Ok(()),
        (Some(a), false) => {
            let e = parse_dao_plist(inner)?;
            if a == e { Ok(()) } else {
                Err(format!("row: rust={:?} lisp={:?}", a, e))
            }
        }
        (Some(a), true)  => Err(format!("rust returned row, lisp returned nil\n  rust: {:?}", a)),
        (None,    false) => Err(format!("rust returned nil, lisp returned a row\n  lisp: {}", inner)),
    }
}
