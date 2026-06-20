use crate::characters::kana::as_hiragana;
use crate::conn::kani_context::KaniranContext;
use crate::dict::counters::classes::CounterText;
use crate::dict::dao::SimpleText;
use crate::dict::kani_word::{KaniSimpleTextDispatchEnum, KaniWordDispatchEnum};
use crate::dict::readings::{find_word, FindWordRows};
use crate::dict::split::segsplit::KANA_HINT_SPACE;
use crate::numbers::constants::{DIGIT_KANJI_DEFAULT, POWER_KANJI};
use crate::numbers::kana_form::{number_to_kana, NumberToKanaOutput};
use crate::numbers::kanji_form::number_to_kanji;

/// Port of `ichiran/dict:proxy-text` (`dict.lisp:550`).
///
/// In-memory `simple-text` subclass that wraps a real reading row
/// (`kanji-text`, `kana-text`, or recursively another `proxy-text`)
/// while presenting altered surface forms, delegating identity-bearing
/// accessors through to the wrapped source.
#[derive(Debug, Clone)]
pub struct ProxyText {
    pub text: String,
    pub kana: String,
    pub source: Box<KaniSimpleTextDispatchEnum>,
    pub state: SimpleText,
}

/// Port of `ichiran/dict:find-word-as-hiragana` (`dict.lisp:592`).
///
/// When `str` is not strictly hiragana, looks up its hiragana
/// equivalent through [`find_word`] root-only (or through the
/// caller-provided `finder` override) and wraps each resulting reading
/// in a [`ProxyText`] that carries the original surface form as both
/// `text` and `kana` while delegating identity through to the
/// underlying simple-text row.
/// Boxed async finder closure for [`find_word_as_hiragana`]. Mirrors
/// `or-as-hiragana`'s `(lambda (w) (apply fn w args))` callback shape
/// (`dict-grammar.lisp:97-100`): a one-shot unary call that takes
/// the hiragana surface form and returns either a [`FindWordRows`]
/// list or a `crate::conn::KaniDbError`. `Send` so the result composes with
/// `tokio::task::spawn` paths (audit harness, segmenter pipeline).
pub type HiraganaFinder<'a> = Box<
    dyn FnOnce(String) -> Result<FindWordRows, crate::conn::KaniDbError> + Send + 'a,
>;

pub fn find_word_as_hiragana(
    ctx: &KaniranContext,
    str_: &str,
    exclude: &[i32],
    finder: Option<HiraganaFinder<'_>>,
) -> Result<Vec<ProxyText>, crate::conn::KaniDbError> {
    let as_hira = as_hiragana(str_);
    if str_ == as_hira {
        return Ok(Vec::new());
    }
    let words = match finder {
        Some(f) => f(as_hira)?,
        // root_only=true, so the substring-hash short-circuit doesn't
        // apply (find_word skips the cache check for root_only); the
        // ctx.substring_hash slot is read inside find_word.
        None => find_word(ctx, &as_hira, true)?.into_owned(),
    };
    let proxies = match words {
        FindWordRows::Kana(rows) => rows
            .into_iter()
            .filter(|w| !exclude.contains(&w.seq))
            .map(|w| ProxyText {
                text: str_.to_string(),
                kana: str_.to_string(),
                source: Box::new(KaniSimpleTextDispatchEnum::Kana(w)),
                state: SimpleText::default(),
            })
            .collect(),
        FindWordRows::Kanji(rows) => rows
            .into_iter()
            .filter(|w| !exclude.contains(&w.seq))
            .map(|w| ProxyText {
                text: str_.to_string(),
                kana: str_.to_string(),
                source: Box::new(KaniSimpleTextDispatchEnum::Kanji(w)),
                state: SimpleText::default(),
            })
            .collect(),
    };
    Ok(proxies)
}

/// Port of `ichiran/dict:compound-text` (`dict.lisp:608`).
///
/// In-memory record for a compound token — the runtime aggregate the
/// segmenter builds via `adjoin-word` when it chains adjacent readings
/// into one word.
#[derive(Debug, Clone)]
pub struct CompoundText {
    pub text: String,
    pub kana: String,
    pub primary: Box<KaniWordDispatchEnum>,
    pub words: Vec<KaniWordDispatchEnum>,
    pub score_base: Option<Box<KaniWordDispatchEnum>>,
    pub score_mod: ScoreMod,
}

/// Three variants matching the three reachable methods of
/// `(defgeneric apply-score-mod …)` at `dict.lisp:735-742`:
///
/// - [`ScoreMod::Single`] — the upstream `((score-mod integer) …)`
///   method's input shape. `apply-score-mod` computes
///   `score * sm * len`.
/// - [`ScoreMod::Constant`] — the upstream `((score-mod function) …)`
///   method's only reachable input shape: `(constantly N)` from
///   `dict-grammar.lisp:404, 448, 516, 532`. `apply-score-mod`
///   computes `(funcall (constantly N) score)` → `N`. The Rust
///   variant carries `N` directly because no upstream callsite ever
///   constructs a non-`constantly` closure as `:score-mod`.
/// - [`ScoreMod::Stack`] — the upstream `((score-mod list) …)`
///   method's input shape, holding a flat list whose elements are
///   either `Single` or `Constant` (built by `adjoin-word`'s
///   cons/list growth at `dict.lisp:651`).
///
/// Payloads are `i64` to match SBCL's 63-bit fixnum width — `score`,
/// `len`, every stored `score-mod` literal, and the multiplication
/// chain in `apply-score-mod` all participate in the same numeric
/// type, with no narrowing at the function boundary.
#[derive(Debug, Clone)]
pub enum ScoreMod {
    Single(i64),
    Constant(i64),
    Stack(Vec<ScoreMod>),
}

/// Port of `ichiran/dict:number-text` (`dict-counters.lisp:203`).
///
/// Counter cache entry for the bare-number reading; adds no slots over
/// [`crate::dict::counters::classes::CounterText`].
#[derive(Debug, Clone)]
pub struct NumberText(pub CounterText);

impl NumberText {
    /// `get-kana` override — `dict-counters.lisp:208-209`:
    /// `(number-to-kana (number-value obj) :separator *kana-hint-space*)`.
    /// Always specializes; no call-next-method path.
    pub fn get_kana(&self) -> String {
        match number_to_kana(self.0.number, Some(KANA_HINT_SPACE), |x| {
            number_to_kanji(x, DIGIT_KANJI_DEFAULT, POWER_KANJI, false)
        }) {
            NumberToKanaOutput::Joined(s) => s,
            NumberToKanaOutput::Groups(_) => {
                unreachable!("number-to-kana with Some(separator) always returns Joined")
            }
        }
    }
}
