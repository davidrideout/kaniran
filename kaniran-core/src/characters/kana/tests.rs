use super::*;

// --- normalize ---
/// Default mode normalizes full-width digit, dakuten, and Japanese comma.
#[test]
fn default_mode_normalizes_punctuation_and_dakuten() {
    assert_eq!(normalize("０", NormalizationContext::Default), "0");
    assert_eq!(normalize("か゛", NormalizationContext::Default), "が");
    assert_eq!(normalize("、", NormalizationContext::Default), ", ");
}

/// Kana mode folds half-width kana and dakuten but leaves ASCII punctuation alone.
#[test]
fn kana_mode_only_kana_and_dakuten() {
    assert_eq!(normalize("ｱ", NormalizationContext::Kana), "ア");
    assert_eq!(normalize("か゛", NormalizationContext::Kana), "が");
    assert_eq!(normalize("、", NormalizationContext::Kana), "、");
}

// --- as_hiragana ---
#[test]
fn katakana_becomes_hiragana_kanji_passes_through() {
    assert_eq!(as_hiragana("ア"), "あ");
    assert_eq!(as_hiragana("カタカナ"), "かたかな");
    assert_eq!(as_hiragana("日本ア"), "日本あ");
}
