use crate::dict::split::segsplit::KANA_HINT_SPACE;
use crate::dict::counters::kani_counter_args::{CounterArgs, CounterClass};
use crate::dict::kana_text_dao::KanaText;
use crate::dict::grammar::suffix::kani_suffix_kind::SuffixKind;
use crate::dict::kanji_text_dao::KanjiText;
use crate::dict::number_text_class::NumberText;
use crate::numbers::constants::{DIGIT_KANJI_DEFAULT, POWER_KANJI};
use crate::numbers::kana_form::{number_to_kana, NumberToKanaOutput};
use crate::numbers::kanji_form::{number_to_kanji, parse_number, NotANumber};

/// Port of `ichiran/dict:counter-text` (`dict-counters.lisp:9`).
///
/// Base type for every counter entry the `*counter-cache*` populator
/// stores and that `find-counter` instantiates per query, carrying the
/// surface forms plus the metadata that drives `counter-join`'s
/// euphonic transformations and `find-counter`'s filtering. The
/// [`Counter`] enum below dispatches across the special-counter
/// subclasses.
#[derive(Debug, Clone)]
pub struct CounterText {
    pub text: String,
    pub kana: String,
    pub number_text: String,
    pub number: u64,
    pub source: Option<CounterSource>,
    pub ordinalp: bool,
    pub suffix: Option<String>,
    pub accepts_suffixes: Vec<SuffixKind>,
    pub suffix_descriptions: Vec<String>,
    pub digit_opts: Vec<DigitOptEntry>,
    pub common: Common,
    pub allowed: Vec<i32>,
    pub foreign: bool,
}

impl CounterText {
    /// Default `verify` method body — `dict-counters.lisp:31-36`,
    /// the `(:method (counter unique) ...)` clause on the gf:
    ///
    /// ```lisp
    /// (and (or (not (counter-allowed counter))
    ///          (find (number-value counter) (counter-allowed counter)))
    ///      unique)
    /// ```
    ///
    /// Empty `allowed` means "any number is fine"; otherwise the
    /// number must be a member. Subclass overrides
    /// ([`CounterTsu::verify`], [`CounterDaysOn::verify`]) chain back
    /// here through the dispatcher rather than calling this method
    /// directly — except [`CounterDaysOn::verify`], which mirrors
    /// the upstream `(call-next-method)` on the days-on method.
    ///
    /// [`CounterTsu::verify`]: crate::dict::counters::classes::CounterTsu::verify
    /// [`CounterDaysOn::verify`]: crate::dict::counters::classes::CounterDaysOn::verify
    pub fn verify(&self, unique: bool) -> bool {
        let n = self.number as i64;
        let allowed_match = self.allowed.is_empty() || self.allowed.iter().any(|&a| a as i64 == n);
        allowed_match && unique
    }

    /// Default `value-string` body — `dict-counters.lisp:46-49`,
    /// the `(:method ((counter counter-text)) ...)` clause on the gf:
    ///
    /// ```lisp
    /// (format nil "Value: ~a~{ ~a~}"
    ///         (if (ordinalp counter) (ordinal-str value) value)
    ///         (reverse (counter-suffix-descriptions counter)))
    /// ```
    ///
    /// `~{ ~a~}` iterates the reversed descriptions list, prefixing
    /// each entry with a single space — output is
    /// `"Value: <n-or-ordinal>"` followed by zero or more
    /// `" <desc>"`. Subclass overrides
    /// ([`CounterHalfhour::value_string`], [`CounterMonths::value_string`],
    /// [`CounterWari::value_string`]) replace this body entirely;
    /// they're routed by [`crate::dict::counters::methods::value_string`].
    ///
    /// [`CounterHalfhour::value_string`]: crate::dict::counters::classes::CounterHalfhour::value_string
    /// [`CounterMonths::value_string`]: crate::dict::counters::classes::CounterMonths::value_string
    /// [`CounterWari::value_string`]: crate::dict::counters::classes::CounterWari::value_string
    pub fn value_string(&self) -> String {
        let head = if self.ordinalp {
            crate::dict::counters::methods::ordinal_str(self.number as i64)
        } else {
            self.number.to_string()
        };
        let mut out = format!("Value: {}", head);
        for desc in self.suffix_descriptions.iter().rev() {
            out.push(' ');
            out.push_str(desc);
        }
        out
    }

    /// `get-kana` base primary — `dict-counters.lisp:64-67`:
    ///
    /// ```lisp
    /// (defmethod get-kana ((obj counter-text))
    ///   (counter-join obj n (number-to-kana n :separator *kana-hint-space*)
    ///                 (copy-seq (counter-kana obj))))
    /// ```
    ///
    /// Called by [`Counter::get_kana`] as the `call-next-method`
    /// target for subclasses whose specialized method returns
    /// `None`. Takes the wrapping [`Counter`] because the inner
    /// [`crate::dict::counters::dispatchers::counter_join`] dispatches on
    /// subclass.
    pub fn primary_get_kana_for(counter: &Counter) -> String {
        let base = counter.base();
        let n = base.number;
        let number_kana = match number_to_kana(n, Some(KANA_HINT_SPACE), |x| {
            number_to_kanji(x, DIGIT_KANJI_DEFAULT, POWER_KANJI, false)
        }) {
            NumberToKanaOutput::Joined(s) => s,
            NumberToKanaOutput::Groups(_) => {
                unreachable!("number-to-kana with Some(separator) always returns Joined")
            }
        };
        crate::dict::counters::dispatchers::counter_join(
            counter,
            n as i64,
            number_kana,
            base.kana.clone(),
        )
    }

    /// Build the shared base from a recipe + the user-typed
    /// `number_text`, running the `initialize-instance :after`
    /// (`dict-counters.lisp:51`) that fills the `number` slot from
    /// `(parse-number number-text)`. Private so all construction
    /// flows through [`Counter::new`] — bypassing the dispatcher
    /// would skip subclass-specific slots (e.g. `digit_set`) and
    /// pick the wrong wrapper variant.
    fn from_args(args: &CounterArgs, number_text: String) -> Result<Self, NotANumber> {
        // dict-counters.lisp:51 (initialize-instance :after counter-text)
        let number = parse_number(&number_text)?;
        Ok(CounterText {
            text: args.text.clone(),
            kana: args.kana.clone(),
            number_text,
            number,
            source: args.source.clone(),
            ordinalp: args.ordinalp,
            suffix: args.suffix.clone(),
            accepts_suffixes: args.accepts.clone(),
            suffix_descriptions: args.suffix_descriptions.clone(),
            digit_opts: args.digit_opts.clone(),
            common: args.common,
            allowed: args.allowed.clone(),
            foreign: args.foreign,
        })
    }
}

#[derive(Debug, Clone)]
pub enum CounterSource {
    Kanji(KanjiText),
    Kana(KanaText),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Common {
    Inherit,
    Null,
    Score(i32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigitOptEntry {
    pub key: DigitOptKey,
    pub ops: Vec<DigitOp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigitOptKey {
    Off,
    Digit(i32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DigitOp {
    Geminate,
    Rendaku,
    Handakuten,
    Counter,
    Replace(String),
}

/// Family dispatcher for the counter-text class hierarchy. One
/// variant per Lisp class. Per-generic dispatch methods (`get_kana`,
/// `verify`, `value_string`, `counter_join`, etc.) land here when the
/// `find-counter` wave is ported; they match-and-delegate to the
/// variant's own method on the wrapped struct, then apply
/// counter-text's `:around` wrappers (e.g. appending
/// [`CounterText::suffix`] to `get_kana` output) at the dispatcher
/// level. See CONVENTIONS §4.9.
#[derive(Debug, Clone)]
pub enum Counter {
    Base(CounterText),
    NumberText(NumberText),
    Age(CounterAge),
    DaysKun(CounterDaysKun),
    DaysOn(CounterDaysOn),
    Halfhour(CounterHalfhour),
    Hifumi(CounterHifumi),
    Months(CounterMonths),
    People(CounterPeople),
    Tsu(CounterTsu),
    Wari(CounterWari),
}

impl Counter {
    /// Mirrors `(apply 'make-instance '(,class ,@args :number-text ,number))`
    /// in `find-counter` (`dict-counters.lisp:278`). Single entry
    /// point: branches on [`CounterArgs::class`] to pick the wrapper
    /// variant, populates subclass-specific slots
    /// ([`CounterHifumi::digit_set`] from [`CounterArgs::digit_set`]),
    /// and runs the `counter-text` `initialize-instance :after` once
    /// via [`CounterText::from_args`] for every variant — same
    /// property as CLOS's inherited `:after`. The upstream catches
    /// `not-a-number` at the call site; here that surfaces as
    /// `Err(NotANumber)` for the caller to drop.
    ///
    /// [`CounterArgs::class`]: crate::dict::counters::kani_counter_args::CounterArgs#structfield.class
    /// [`CounterArgs::digit_set`]: crate::dict::counters::kani_counter_args::CounterArgs#structfield.digit_set
    /// [`CounterHifumi::digit_set`]: crate::dict::counters::classes::CounterHifumi#structfield.digit_set
    pub fn new(args: &CounterArgs, number_text: impl Into<String>) -> Result<Self, NotANumber> {
        // dict-counters.lisp:278 (find-counter — apply make-instance over recipe)
        let mut base = CounterText::from_args(args, number_text.into())?;
        Ok(match args.class {
            CounterClass::Text => Counter::Base(base),
            CounterClass::NumberText => Counter::NumberText(NumberText(base)),
            CounterClass::Tsu => Counter::Tsu(CounterTsu(base)),
            CounterClass::Age => Counter::Age(CounterAge(base)),
            CounterClass::DaysKun => {
                // dict-counters.lisp:687 (defclass counter-days-kun — :initform allowed)
                if base.allowed.is_empty() {
                    base.allowed = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 14, 20, 24, 30];
                }
                Counter::DaysKun(CounterDaysKun(base))
            }
            CounterClass::DaysOn => Counter::DaysOn(CounterDaysOn(base)),
            CounterClass::Halfhour => Counter::Halfhour(CounterHalfhour(base)),
            CounterClass::Months => {
                // dict-counters.lisp:722-723 (defclass counter-months — :initform allowed/digit-opts)
                if base.allowed.is_empty() {
                    base.allowed = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
                }
                if base.digit_opts.is_empty() {
                    base.digit_opts = vec![
                        DigitOptEntry {
                            key: DigitOptKey::Digit(4),
                            ops: vec![DigitOp::Replace("し".to_string())],
                        },
                        DigitOptEntry {
                            key: DigitOptKey::Digit(7),
                            ops: vec![DigitOp::Replace("しち".to_string())],
                        },
                        DigitOptEntry {
                            key: DigitOptKey::Digit(9),
                            ops: vec![DigitOp::Replace("く".to_string())],
                        },
                    ];
                }
                Counter::Months(CounterMonths(base))
            }
            CounterClass::People => Counter::People(CounterPeople(base)),
            CounterClass::Wari => Counter::Wari(CounterWari(base)),
            CounterClass::Hifumi => {
                // dict-counters.lisp:518-519 (defclass counter-hifumi) — :digit-set has no
                // :initform; upstream make-instance without :digit-set leaves the slot
                // unbound and any read raises UNBOUND-SLOT. Eager fail-fast here matches
                // that contract; a recipe missing :digit-set is a populator bug, not a
                // runtime input.
                assert!(
                    !args.digit_set.is_empty(),
                    "counter-hifumi requires non-empty :digit-set (dict-counters.lisp:518)"
                );
                Counter::Hifumi(CounterHifumi {
                    base,
                    digit_set: args.digit_set.clone(),
                })
            }
        })
    }

    /// Borrow the shared counter-text slot data underlying this
    /// variant. Used by dispatchers and by callers that need to read
    /// inherited slots (`text`, `kana`, `number`, `source`, etc.)
    /// without caring which subclass produced them.
    pub fn base(&self) -> &CounterText {
        match self {
            Counter::Base(c) => c,
            Counter::NumberText(c) => &c.0,
            Counter::Age(c) => &c.0,
            Counter::DaysKun(c) => &c.0,
            Counter::DaysOn(c) => &c.0,
            Counter::Halfhour(c) => &c.0,
            Counter::Hifumi(c) => &c.base,
            Counter::Months(c) => &c.0,
            Counter::People(c) => &c.0,
            Counter::Tsu(c) => &c.0,
            Counter::Wari(c) => &c.0,
        }
    }

    /// Family-level `get-kana` for the counter-text family.
    /// Implements the `:around` suffix-append wrapper
    /// (`dict-counters.lisp:69-71`):
    ///
    /// ```lisp
    /// (defmethod get-kana :around ((obj counter-text))
    ///   (let ((kana (call-next-method)))
    ///     (if (counter-suffix obj) (concatenate 'string kana (counter-suffix obj)) kana)))
    /// ```
    ///
    /// `call-next-method` dispatches to the subclass's specialized
    /// `get_kana` (if any returns `Some`); on `None` it falls back
    /// to [`CounterText::primary_get_kana_for`]. Per CONVENTIONS §4.7,
    /// each family handles its own `:around` internally; the
    /// top-level [`crate::dict::get_kana::get_kana`] dispatcher
    /// just delegates here for the counter arm.
    pub fn get_kana(&self) -> String {
        let primary = match self {
            Counter::Tsu(c) => c.get_kana(),
            Counter::Hifumi(c) => c.get_kana(),
            Counter::DaysKun(c) => c.get_kana(),
            Counter::People(c) => c.get_kana(),
            Counter::Age(c) => c.get_kana(),
            Counter::NumberText(c) => Some(c.get_kana()),
            // Counter::Base / DaysOn / Halfhour / Months / Wari —
            // no specialized override; primary IS the base body.
            _ => None,
        };
        let body = primary.unwrap_or_else(|| CounterText::primary_get_kana_for(self));
        match &self.base().suffix {
            Some(suf) => format!("{}{}", body, suf),
            None => body,
        }
    }
}

/// Port of `ichiran/dict:counter-halfhour` (`dict-counters.lisp:391`).
///
/// Counter cache entry for 時半 (half past N o'clock), whose
/// `value-string` override formats the display string as `"N:30"`.
#[derive(Debug, Clone)]
pub struct CounterHalfhour(pub CounterText);

impl CounterHalfhour {
    /// `value-string` override — `dict-counters.lisp:393-394`:
    ///
    /// ```lisp
    /// (defmethod value-string ((counter counter-halfhour))
    ///   (format nil "~a:30" (number-value counter)))
    /// ```
    pub fn value_string(&self) -> String {
        format!("{}:30", self.0.number)
    }
}

/// Port of `ichiran/dict:counter-tsu` (`dict-counters.lisp:497`).
///
/// Counter cache entry for the bare つ counter, whose `verify`
/// override restricts validity to `1 <= n <= 9` and whose `get-kana`
/// override is a closed table over those values (ひとつ, ふたつ, …,
/// ここのつ).
#[derive(Debug, Clone)]
pub struct CounterTsu(pub CounterText);

impl CounterTsu {
    /// `dict-counters.lisp:499` — `(<= 1 (number-value counter) 9)`
    /// AND `unique`. Ignores the `allowed` slot entirely; the bare つ
    /// counter is valid only for the kun-yomi 1..9 range and the
    /// `get-kana` table covers exactly those values.
    pub fn verify(&self, unique: bool) -> bool {
        let n = self.0.number;
        (1..=9).contains(&n) && unique
    }

    /// `get-kana` override — `dict-counters.lisp:502-513`.
    /// Closed kun-yomi table for 1..9; everything else falls
    /// through to `call-next-method` (the counter-text base
    /// primary). Returns `None` to signal fall-through.
    pub fn get_kana(&self) -> Option<String> {
        let n = self.0.number as i64;
        Some(match n {
            1 => "ひとつ".to_string(),
            2 => "ふたつ".to_string(),
            3 => "みっつ".to_string(),
            4 => "よっつ".to_string(),
            5 => "いつつ".to_string(),
            6 => "むっつ".to_string(),
            7 => "ななつ".to_string(),
            8 => "やっつ".to_string(),
            9 => "ここのつ".to_string(),
            _ => return None,
        })
    }
}

/// Port of `ichiran/dict:counter-hifumi` (`dict-counters.lisp:518`).
///
/// Counter cache entry for the ~30 counters taking native kun-yomi
/// numeric prefixes (ひと, ふた, み, …) for small counts instead of
/// Sino-Japanese readings. `digit_set` holds the digit values for
/// which the kun-yomi prefix applies; values outside it fall through
/// to the parent's default reading.
#[derive(Debug, Clone)]
pub struct CounterHifumi {
    pub base: CounterText,
    pub digit_set: Vec<i32>,
}

impl CounterHifumi {
    /// `get-kana` override — `dict-counters.lisp:521-538`.
    ///
    /// ```lisp
    /// (defmethod get-kana ((obj counter-hifumi))
    ///   (cond ((find (number-value obj) (digit-set obj))
    ///          (concatenate 'string
    ///                       (case (number-value obj)
    ///                         (1 "ひと") (2 "ふた") (3 "み") (4 "よ")
    ///                         (5 "いつ") (6 "む") (7 "なな") (8 "や")
    ///                         (9 "ここの") (10 "と"))
    ///                       (counter-kana obj)))
    ///         (t (call-next-method))))
    /// ```
    ///
    /// Returns:
    /// - `Some(prefix + counter_kana)` when `value` is in `digit_set`.
    ///   The inner `case` without a `t` clause returns `nil` for
    ///   values outside 1..=10 — `(concatenate 'string nil ...)`
    ///   treats nil as empty, so the result is just `counter_kana`.
    ///   This is NOT `call-next-method` and must not fall through.
    /// - `None` when `value` is NOT in `digit_set` — upstream's
    ///   `(t (call-next-method))` arm.
    pub fn get_kana(&self) -> Option<String> {
        let value = self.base.number as i64;
        if !self.digit_set.iter().any(|&d| i64::from(d) == value) {
            // outside digit-set → call-next-method
            return None;
        }
        let prefix = match value {
            1 => "ひと",
            2 => "ふた",
            3 => "み",
            4 => "よ",
            5 => "いつ",
            6 => "む",
            7 => "なな",
            8 => "や",
            9 => "ここの",
            10 => "と",
            _ => "",
        };
        Some(format!("{}{}", prefix, self.base.kana))
    }
}

/// Port of `ichiran/dict:counter-days-kun` (`dict-counters.lisp:686`).
///
/// Counter cache entry for 日 read with the kun-yomi day counts
/// (ついたち, ふつか, みっか, …, みそか), whose `get-kana` override is
/// a closed table over the allowed values.
#[derive(Debug, Clone)]
pub struct CounterDaysKun(pub CounterText);

impl CounterDaysKun {
    /// `get-kana` override — `dict-counters.lisp:689-704`.
    /// Closed table over the allowed kun-yomi day-count values.
    /// Returns `Some` for every table entry; `Some(String::new())`
    /// for off-table values, mirroring upstream `case` without a
    /// `t` clause returning nil — which the `:around` then
    /// concatenates with the suffix as empty. Never falls through
    /// to `call-next-method` because the `verify` restriction
    /// limits inputs to the table entries.
    pub fn get_kana(&self) -> Option<String> {
        Some(match self.0.number as i64 {
            1 => "ついたち".to_string(),
            2 => "ふつか".to_string(),
            3 => "みっか".to_string(),
            4 => "よっか".to_string(),
            5 => "いつか".to_string(),
            6 => "むいか".to_string(),
            7 => "なのか".to_string(),
            8 => "ようか".to_string(),
            9 => "ここのか".to_string(),
            10 => "とうか".to_string(),
            14 => "じゅうよっか".to_string(),
            20 => "はつか".to_string(),
            24 => "にじゅうよっか".to_string(),
            30 => "みそか".to_string(),
            // `case` without `t` returns nil upstream;
            // `(concatenate 'string nil suffix)` treats nil as
            // empty. Mirror by emitting the empty string here.
            _ => String::new(),
        })
    }
}

/// Port of `ichiran/dict:counter-days-on` (`dict-counters.lisp:709`).
///
/// Counter cache entry for 日 read with the on-yomi day count にち,
/// whose `verify` override restricts validity to `n == 1` or `n > 10`
/// (and never 20, which belongs to the kun-yomi day counter).
#[derive(Debug, Clone)]
pub struct CounterDaysOn(pub CounterText);

impl CounterDaysOn {
    /// `dict-counters.lisp:711-716`:
    ///
    /// ```lisp
    /// (and (or (> n 10) (= n 1))
    ///      (/= n 20)
    ///      (call-next-method))
    /// ```
    ///
    /// `n` is the counter's number-value. The on-yomi day-count にち
    /// is valid for the literal day "1日" or any value above ten,
    /// excluding 20 (which is owned by [`CounterDaysKun`]).
    /// `(call-next-method)` falls through to [`CounterText::verify`].
    ///
    /// [`CounterDaysKun`]: crate::dict::counters::classes::CounterDaysKun
    pub fn verify(&self, unique: bool) -> bool {
        let n = self.0.number;
        (n > 10 || n == 1) && n != 20 && self.0.verify(unique)
    }
}

/// Port of `ichiran/dict:counter-months` (`dict-counters.lisp:721`).
///
/// Counter cache entry for 月 read as がつ (month-of-year, January
/// through December), whose `value-string` override emits the English
/// month name (`"January"`..`"December"`) instead of the numeric
/// default.
#[derive(Debug, Clone)]
pub struct CounterMonths(pub CounterText);

const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

impl CounterMonths {
    /// `value-string` override — `dict-counters.lisp:725-730`:
    ///
    /// ```lisp
    /// (defmethod value-string ((counter counter-months))
    ///   (aref #("January" ... "December") (1- (number-value counter))))
    /// ```
    ///
    /// Upstream `aref` raises a bounds error when `number-value` is
    /// outside `1..=12`; here that surfaces as a panic from the array
    /// index. Counter `verify` keeps `number` inside `allowed`
    /// (`[1..=12]` for this class) so the call site never reaches this
    /// method with an out-of-range value.
    pub fn value_string(&self) -> String {
        MONTH_NAMES[(self.0.number - 1) as usize].to_string()
    }
}

/// Port of `ichiran/dict:counter-people` (`dict-counters.lisp:735`).
///
/// Counter cache entry for 人 (person count), whose `get-kana`
/// override returns ひとり for 1 and ふたり for 2, falling through to
/// the default for all other counts.
#[derive(Debug, Clone)]
pub struct CounterPeople(pub CounterText);

impl CounterPeople {
    /// `get-kana` override — `dict-counters.lisp:737-741`. Returns
    /// ひとり for 1, ふたり for 2, otherwise `None` to fall through
    /// to the counter-text base primary.
    pub fn get_kana(&self) -> Option<String> {
        match self.0.number as i64 {
            1 => Some("ひとり".to_string()),
            2 => Some("ふたり".to_string()),
            _ => None,
        }
    }
}

/// Port of `ichiran/dict:counter-wari` (`dict-counters.lisp:746`).
///
/// Counter cache entry for 割 / 割引 (tenths / percentage), whose
/// `value-string` override emits `"N0%"` (since 1 割 == 10%).
#[derive(Debug, Clone)]
pub struct CounterWari(pub CounterText);

impl CounterWari {
    /// `value-string` override — `dict-counters.lisp:748-749`:
    ///
    /// ```lisp
    /// (defmethod value-string ((counter counter-wari))
    ///   (format nil "~a%" (* 10 (number-value counter))))
    /// ```
    pub fn value_string(&self) -> String {
        format!("{}%", self.0.number * 10)
    }
}

/// Port of `ichiran/dict:counter-age` (`dict-counters.lisp:757`).
///
/// Counter cache entry for the 歳 / 才 (age) counter, whose `get-kana`
/// override turns 20 into はたち (every other value falls through to
/// the default).
#[derive(Debug, Clone)]
pub struct CounterAge(pub CounterText);

impl CounterAge {
    /// `get-kana` override — `dict-counters.lisp:759-762`. Returns
    /// はたち for 20, otherwise `None` to fall through to the
    /// counter-text base primary.
    pub fn get_kana(&self) -> Option<String> {
        match self.0.number as i64 {
            20 => Some("はたち".to_string()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;
