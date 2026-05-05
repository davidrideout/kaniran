//! Common Lisp `prin1` reader for fixture replay.
//!
//! Replaces the lexpr-based parser. lexpr targets Scheme/Elisp syntax
//! and rejects three things SBCL emits: `#\U<hex>` chars, `#\<NAME>`
//! chars, and `#N=`/`#N#` datum labels. Rather than fork lexpr we read
//! the subset of CL source the tracer actually produces:
//!
//! - lists `(...)`, dotted pairs `(a . b)`, vectors `#(...)`
//! - strings with `\"`, `\\`, `\n`, `\t`, `\r` escapes
//! - integers (i64), floats (`d` `s` `f` `l` `e` exponent markers)
//! - symbols, keywords (`:NAME`)
//! - `NIL` / `()` collapse to [`Sexp::Nil`]; `T` stays as a symbol
//! - chars in three forms:
//!   - `#\<single>` — any single Unicode codepoint
//!   - `#\<NAME>` — the CLtL2 named set (Space, Newline, Tab, …)
//!   - `#\U<hex>` — SBCL's hex extension
//! - `(:CHAR <codepoint>)` — the tagged form `trace_capture.lisp` emits
//!   for chars that SBCL would otherwise print with a Unicode name like
//!   `#\HIRAGANA_LETTER_HA` (which would require a Unicode-name table)
//! - datum labels `#N=value` / `#N#`
//!
//! Anything outside this surface is a parse error with a byte offset.

use std::collections::HashMap;
use std::fmt;

/// A parsed Common Lisp value.
///
/// `Nil` represents both the empty list `()` and the symbol `NIL` —
/// Common Lisp identifies them, so we do too. The symbol `T` stays as
/// `Symbol("T")` because callers may care about its identity (e.g.
/// `(eq x 't)`-style dispatch in ported code).
#[derive(Debug, Clone, PartialEq)]
pub enum Sexp {
    Nil,
    Int(i64),
    Float(f64),
    Symbol(String),
    Keyword(String),
    Char(char),
    String(String),
    Cons(Box<Sexp>, Box<Sexp>),
    Vector(Vec<Sexp>),
}

impl Sexp {
    pub fn is_nil(&self) -> bool {
        matches!(self, Sexp::Nil)
    }

    /// True for the symbol `T`.
    pub fn is_t(&self) -> bool {
        matches!(self, Sexp::Symbol(s) if s == "T")
    }

    pub fn is_int(&self) -> bool { matches!(self, Sexp::Int(_)) }
    pub fn is_float(&self) -> bool { matches!(self, Sexp::Float(_)) }
    pub fn is_string(&self) -> bool { matches!(self, Sexp::String(_)) }
    pub fn is_symbol(&self) -> bool { matches!(self, Sexp::Symbol(_)) }
    pub fn is_keyword(&self) -> bool { matches!(self, Sexp::Keyword(_)) }
    pub fn is_char(&self) -> bool { matches!(self, Sexp::Char(_)) }
    pub fn is_vector(&self) -> bool { matches!(self, Sexp::Vector(_)) }
    pub fn is_cons(&self) -> bool { matches!(self, Sexp::Cons(_, _)) }

    /// True for `Nil` and for any cons chain whose final cdr is `Nil`.
    pub fn is_list(&self) -> bool {
        let mut cur = self;
        loop {
            match cur {
                Sexp::Nil => return true,
                Sexp::Cons(_, cdr) => cur = cdr,
                _ => return false,
            }
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        if let Sexp::Int(i) = self { Some(*i) } else { None }
    }

    pub fn as_f64(&self) -> Option<f64> {
        if let Sexp::Float(x) = self { Some(*x) } else { None }
    }

    pub fn as_str(&self) -> Option<&str> {
        if let Sexp::String(s) = self { Some(s) } else { None }
    }

    pub fn as_symbol(&self) -> Option<&str> {
        if let Sexp::Symbol(s) = self { Some(s) } else { None }
    }

    pub fn as_keyword(&self) -> Option<&str> {
        if let Sexp::Keyword(s) = self { Some(s) } else { None }
    }

    pub fn as_char(&self) -> Option<char> {
        if let Sexp::Char(c) = self { Some(*c) } else { None }
    }

    pub fn as_cons(&self) -> Option<(&Sexp, &Sexp)> {
        if let Sexp::Cons(car, cdr) = self {
            Some((car, cdr))
        } else {
            None
        }
    }

    /// Iterate the cars of a proper list. Returns `None` if `self` is
    /// neither `Nil` nor a proper cons chain (i.e. a dotted pair would
    /// not satisfy this — use [`Sexp::as_cons`] for those).
    pub fn list_iter(&self) -> Option<ListIter<'_>> {
        if self.is_list() {
            Some(ListIter { current: self })
        } else {
            None
        }
    }
}

pub struct ListIter<'a> {
    current: &'a Sexp,
}

impl<'a> Iterator for ListIter<'a> {
    type Item = &'a Sexp;

    fn next(&mut self) -> Option<&'a Sexp> {
        if let Sexp::Cons(car, cdr) = self.current {
            self.current = cdr;
            Some(car)
        } else {
            None
        }
    }
}

impl fmt::Display for Sexp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Sexp::Nil => f.write_str("NIL"),
            Sexp::Int(n) => write!(f, "{}", n),
            Sexp::Float(x) => write!(f, "{}", x),
            Sexp::Symbol(s) => f.write_str(s),
            Sexp::Keyword(s) => write!(f, ":{}", s),
            Sexp::Char(c) => {
                if c.is_ascii_graphic() && !matches!(*c, '"' | '\\' | '(' | ')' | ';') {
                    write!(f, "#\\{}", c)
                } else {
                    write!(f, "#\\U{:X}", *c as u32)
                }
            }
            Sexp::String(s) => write!(f, "{:?}", s),
            Sexp::Cons(_, _) => {
                f.write_str("(")?;
                let mut cur = self;
                let mut first = true;
                loop {
                    match cur {
                        Sexp::Cons(car, cdr) => {
                            if !first { f.write_str(" ")?; }
                            first = false;
                            write!(f, "{}", car)?;
                            cur = cdr;
                        }
                        Sexp::Nil => break,
                        other => {
                            write!(f, " . {}", other)?;
                            break;
                        }
                    }
                }
                f.write_str(")")
            }
            Sexp::Vector(items) => {
                f.write_str("#(")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 { f.write_str(" ")?; }
                    write!(f, "{}", item)?;
                }
                f.write_str(")")
            }
        }
    }
}


// --- ParseError ---------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub byte_offset: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "parse error at byte {}: {}", self.byte_offset, self.message)
    }
}

impl std::error::Error for ParseError {}


// --- parser -------------------------------------------------------------

/// Parse a Common Lisp `prin1` value from a string.
///
/// Accepts exactly one top-level value plus optional surrounding
/// whitespace. Trailing non-whitespace bytes are an error; this matches
/// how fixture rows are emitted (one value per `args` / `result` cell).
pub fn parse(input: &str) -> Result<Sexp, ParseError> {
    let mut p = Parser::new(input);
    let v = p.parse_value()?;
    p.skip_whitespace();
    if p.pos < p.bytes.len() {
        return Err(p.err(format!(
            "trailing characters: {:?}",
            std::str::from_utf8(&p.bytes[p.pos..]).unwrap_or("<non-utf8>")
        )));
    }
    Ok(v)
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
    /// `#N=value` definitions, looked up by `#N#`. Stored as cloned
    /// values; ichiran's prin1 output never contains genuinely circular
    /// structure, just shared substructure that `*print-circle*` writes
    /// once.
    labels: HashMap<u32, Sexp>,
}

impl<'a> Parser<'a> {
    fn new(s: &'a str) -> Self {
        Parser { bytes: s.as_bytes(), pos: 0, labels: HashMap::new() }
    }

    fn err(&self, msg: impl Into<String>) -> ParseError {
        ParseError { message: msg.into(), byte_offset: self.pos }
    }

    fn peek(&self) -> Option<u8> { self.bytes.get(self.pos).copied() }

    fn advance_byte(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.pos += 1;
        Some(b)
    }

    fn skip_whitespace(&mut self) {
        loop {
            match self.peek() {
                Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'\r') => {
                    self.pos += 1;
                }
                Some(b';') => {
                    while let Some(b) = self.peek() {
                        self.pos += 1;
                        if b == b'\n' { break; }
                    }
                }
                _ => break,
            }
        }
    }

    fn parse_value(&mut self) -> Result<Sexp, ParseError> {
        self.skip_whitespace();
        match self.peek() {
            None => Err(self.err("unexpected EOF")),
            Some(b'(') => self.parse_list(),
            Some(b'"') => self.parse_string(),
            Some(b'#') => self.parse_sharpsign(),
            Some(b':') => self.parse_keyword(),
            _ => self.parse_atom(),
        }
    }

    fn parse_list(&mut self) -> Result<Sexp, ParseError> {
        self.advance_byte(); // consume '('
        let mut items = Vec::new();
        let mut tail = Sexp::Nil;
        loop {
            self.skip_whitespace();
            match self.peek() {
                Some(b')') => { self.advance_byte(); break; }
                None => return Err(self.err("unexpected EOF inside list")),
                Some(b'.') if is_isolated_dot(self.bytes, self.pos) => {
                    self.advance_byte();
                    self.skip_whitespace();
                    tail = self.parse_value()?;
                    self.skip_whitespace();
                    match self.peek() {
                        Some(b')') => { self.advance_byte(); break; }
                        _ => return Err(self.err("expected `)` after dotted-pair tail")),
                    }
                }
                _ => items.push(self.parse_value()?),
            }
        }
        let mut result = tail;
        for item in items.into_iter().rev() {
            result = Sexp::Cons(Box::new(item), Box::new(result));
        }
        Ok(result)
    }

    fn parse_vector(&mut self) -> Result<Sexp, ParseError> {
        // Caller has consumed `#(`.
        let mut items = Vec::new();
        loop {
            self.skip_whitespace();
            match self.peek() {
                Some(b')') => { self.advance_byte(); break; }
                None => return Err(self.err("unexpected EOF inside vector")),
                _ => items.push(self.parse_value()?),
            }
        }
        Ok(Sexp::Vector(items))
    }

    fn parse_string(&mut self) -> Result<Sexp, ParseError> {
        self.advance_byte(); // consume opening '"'
        let mut out = String::new();
        loop {
            let b = self.peek().ok_or_else(|| self.err("unterminated string"))?;
            if b == b'"' {
                self.advance_byte();
                return Ok(Sexp::String(out));
            }
            if b == b'\\' {
                self.advance_byte();
                let esc = self.peek().ok_or_else(|| self.err("EOF in string escape"))?;
                self.advance_byte();
                match esc {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'n' => out.push('\n'),
                    b't' => out.push('\t'),
                    b'r' => out.push('\r'),
                    other => {
                        // CL prints bytes \\<n> sparingly; pass through any
                        // other ASCII escape literally to avoid silent loss.
                        out.push(other as char);
                    }
                }
                continue;
            }
            // Normal byte (could start a multibyte UTF-8 sequence).
            out.push(self.read_char()?);
        }
    }

    /// Read one Unicode codepoint from the input, advancing `pos` past
    /// its full UTF-8 width. Errors on invalid UTF-8.
    fn read_char(&mut self) -> Result<char, ParseError> {
        let b = self.peek().ok_or_else(|| self.err("EOF reading char"))?;
        let len = if b & 0x80 == 0 { 1 }
            else if b & 0xE0 == 0xC0 { 2 }
            else if b & 0xF0 == 0xE0 { 3 }
            else if b & 0xF8 == 0xF0 { 4 }
            else { return Err(self.err("invalid UTF-8 lead byte")); };
        if self.pos + len > self.bytes.len() {
            return Err(self.err("incomplete UTF-8 sequence"));
        }
        let s = std::str::from_utf8(&self.bytes[self.pos..self.pos + len])
            .map_err(|e| self.err(format!("invalid UTF-8: {}", e)))?;
        let c = s.chars().next().expect("non-empty utf-8 slice");
        self.pos += len;
        Ok(c)
    }

    fn parse_sharpsign(&mut self) -> Result<Sexp, ParseError> {
        self.advance_byte(); // consume '#'
        match self.peek() {
            Some(b'(') => { self.advance_byte(); self.parse_vector() }
            Some(b'\\') => { self.advance_byte(); self.parse_char_literal() }
            Some(c) if c.is_ascii_digit() => self.parse_datum_label(),
            Some(c) => Err(self.err(format!("unexpected #{}", c as char))),
            None => Err(self.err("EOF after `#`")),
        }
    }

    fn parse_char_literal(&mut self) -> Result<Sexp, ParseError> {
        // `#\` already consumed. CL char literals come in three shapes:
        //   #\X         — single literal char, terminated by delimiter
        //   #\Newline   — name, terminated by delimiter
        //   #\U4ECA     — SBCL hex extension (also a name in our table)
        let first = self.read_char()?;
        // If next byte is a delimiter, this is a single-char literal.
        if is_delim(self.peek()) {
            return Ok(Sexp::Char(first));
        }
        // Multi-char name. Re-read starting from `first`.
        let mut name = String::new();
        name.push(first);
        while !is_delim(self.peek()) {
            name.push(self.read_char()?);
        }
        match resolve_char_name(&name) {
            Some(c) => Ok(Sexp::Char(c)),
            None => Err(self.err(format!(
                "unknown character name {:?} (only CLtL2 names + `U<hex>` \
                 are recognised; tag wider Unicode chars as `(:CHAR <code>)` \
                 on the capture side)",
                name
            ))),
        }
    }

    fn parse_datum_label(&mut self) -> Result<Sexp, ParseError> {
        // `#` already consumed; cursor at the first digit.
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.pos += 1;
        }
        let label_str = std::str::from_utf8(&self.bytes[start..self.pos])
            .expect("ASCII digits are valid UTF-8");
        let label: u32 = label_str.parse()
            .map_err(|_| self.err(format!("invalid label number {:?}", label_str)))?;
        match self.peek() {
            Some(b'=') => {
                self.advance_byte();
                let v = self.parse_value()?;
                self.labels.insert(label, v.clone());
                Ok(v)
            }
            Some(b'#') => {
                self.advance_byte();
                self.labels.get(&label).cloned().ok_or_else(|| {
                    self.err(format!("undefined datum reference #{}#", label))
                })
            }
            _ => Err(self.err(format!("expected `=` or `#` after #{}", label))),
        }
    }

    fn parse_keyword(&mut self) -> Result<Sexp, ParseError> {
        self.advance_byte(); // consume ':'
        let name = self.read_atom_token()?;
        if name.is_empty() {
            return Err(self.err("empty keyword after `:`"));
        }
        Ok(Sexp::Keyword(name))
    }

    fn parse_atom(&mut self) -> Result<Sexp, ParseError> {
        let token = self.read_atom_token()?;
        if token.is_empty() {
            return Err(self.err("empty atom"));
        }
        // NIL / nil → Nil. T stays as Symbol("T").
        if token.eq_ignore_ascii_case("NIL") {
            return Ok(Sexp::Nil);
        }
        // Try integer.
        if let Ok(i) = token.parse::<i64>() {
            return Ok(Sexp::Int(i));
        }
        // Try float (CL exponent markers d/s/f/l → e).
        if looks_floaty(&token) {
            if let Ok(x) = parse_cl_float(&token) {
                return Ok(Sexp::Float(x));
            }
        }
        Ok(Sexp::Symbol(token))
    }

    fn read_atom_token(&mut self) -> Result<String, ParseError> {
        let start = self.pos;
        while !is_delim(self.peek()) {
            self.read_char()?;
        }
        let slice = &self.bytes[start..self.pos];
        std::str::from_utf8(slice)
            .map(|s| s.to_string())
            .map_err(|e| self.err(format!("non-UTF-8 atom: {}", e)))
    }
}

fn is_delim(b: Option<u8>) -> bool {
    matches!(
        b,
        None | Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'\r')
            | Some(b'(') | Some(b')') | Some(b'"') | Some(b';')
    )
}

/// `.` is a dotted-pair marker only when surrounded by whitespace /
/// list delimiter on both sides — otherwise it's a token character
/// (e.g. `3.14`, `foo.bar`).
fn is_isolated_dot(bytes: &[u8], pos: usize) -> bool {
    if bytes.get(pos).copied() != Some(b'.') { return false; }
    let next = bytes.get(pos + 1).copied();
    let prev = if pos == 0 { None } else { bytes.get(pos - 1).copied() };
    is_whitespace(next) && is_whitespace(prev)
}

fn is_whitespace(b: Option<u8>) -> bool {
    matches!(b, None | Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'\r'))
}

fn looks_floaty(s: &str) -> bool {
    // Quick rejection — only treat as float if the token has at least
    // one of `.` or an exponent marker.
    s.bytes().any(|b| matches!(b, b'.' | b'e' | b'E' | b'd' | b'D' | b's' | b'S' | b'f' | b'F' | b'l' | b'L'))
}

fn parse_cl_float(s: &str) -> Result<f64, std::num::ParseFloatError> {
    // Replace CL exponent markers (d/D/s/S/f/F/l/L) with `e` so the
    // standard parser accepts them. Common Lisp uses these to encode
    // float subtype (double, single, etc.); we collapse all to f64.
    let needs = s.bytes().any(|b| matches!(b, b'd' | b'D' | b's' | b'S' | b'f' | b'F' | b'l' | b'L'));
    if !needs { return s.parse(); }
    let normalized: String = s.chars().map(|c| match c {
        'd' | 'D' | 's' | 'S' | 'f' | 'F' | 'l' | 'L' => 'e',
        other => other,
    }).collect();
    normalized.parse()
}

/// CLtL2 §13.1 named characters, plus SBCL's `Uxxxx` hex form.
/// Returns `None` for everything else; broader Unicode-name resolution
/// (`HIRAGANA_LETTER_HA`, etc.) lives on the capture side which tags
/// chars as `(:CHAR <codepoint>)` lists.
fn resolve_char_name(name: &str) -> Option<char> {
    // Hex form: `U<hexdigits>`. Case-insensitive on the leading `U`.
    if name.len() >= 2 {
        let first = name.as_bytes()[0];
        if (first == b'U' || first == b'u')
            && name[1..].bytes().all(|b| b.is_ascii_hexdigit())
        {
            if let Ok(n) = u32::from_str_radix(&name[1..], 16) {
                return char::from_u32(n);
            }
        }
    }
    // Named characters. Canonicalise to uppercase for matching.
    let upper = name.to_ascii_uppercase();
    Some(match upper.as_str() {
        "SPACE" => ' ',
        "NEWLINE" | "LINEFEED" => '\n',
        "TAB" => '\t',
        "RETURN" => '\r',
        "BACKSPACE" => '\u{08}',
        "PAGE" => '\u{0C}',
        "RUBOUT" | "DELETE" => '\u{7F}',
        "NULL" | "NUL" => '\u{00}',
        "ESC" | "ESCAPE" => '\u{1B}',
        "BELL" => '\u{07}',
        "VT" => '\u{0B}',
        _ => return None,
    })
}


// --- tests --------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(s: &str) -> Sexp {
        parse(s).unwrap_or_else(|e| panic!("parse {:?} failed: {}", s, e))
    }

    // --- atoms ----------------------------------------------------------

    #[test]
    fn parses_nil_and_t() {
        assert!(parse_ok("NIL").is_nil());
        assert!(parse_ok("nil").is_nil());
        assert!(parse_ok("()").is_nil());
        assert!(parse_ok("T").is_t());
    }

    #[test]
    fn parses_integers() {
        assert_eq!(parse_ok("42").as_i64(), Some(42));
        assert_eq!(parse_ok("-7").as_i64(), Some(-7));
        assert_eq!(parse_ok("0").as_i64(), Some(0));
    }

    #[test]
    fn parses_floats_with_cl_markers() {
        assert_eq!(parse_ok("3.14").as_f64(), Some(3.14));
        assert_eq!(parse_ok("1.5d0").as_f64(), Some(1.5));
        assert_eq!(parse_ok("2.0e3").as_f64(), Some(2000.0));
        assert_eq!(parse_ok("1.0L0").as_f64(), Some(1.0));
    }

    #[test]
    fn parses_symbols_and_keywords() {
        assert_eq!(parse_ok("WORD").as_symbol(), Some("WORD"));
        assert_eq!(parse_ok("ICHIRAN/CHARACTERS:NORMALIZE").as_symbol(),
                   Some("ICHIRAN/CHARACTERS:NORMALIZE"));
        assert_eq!(parse_ok(":HIRAGANA").as_keyword(), Some("HIRAGANA"));
        assert_eq!(parse_ok(":context").as_keyword(), Some("context"));
    }

    #[test]
    fn parses_strings_with_escapes() {
        assert_eq!(parse_ok(r#""hello""#).as_str(), Some("hello"));
        assert_eq!(parse_ok(r#""今日は""#).as_str(), Some("今日は"));
        assert_eq!(parse_ok(r#""a\"b""#).as_str(), Some("a\"b"));
        assert_eq!(parse_ok(r#""a\\b""#).as_str(), Some("a\\b"));
        assert_eq!(parse_ok(r#""line\nbreak""#).as_str(), Some("line\nbreak"));
    }

    // --- structural -----------------------------------------------------

    #[test]
    fn parses_proper_list() {
        let v = parse_ok("(1 2 3)");
        let items: Vec<i64> = v.list_iter().unwrap().map(|e| e.as_i64().unwrap()).collect();
        assert_eq!(items, vec![1, 2, 3]);
    }

    #[test]
    fn parses_dotted_pair() {
        let v = parse_ok("(:WORD . \"今\")");
        let (car, cdr) = v.as_cons().unwrap();
        assert_eq!(car.as_keyword(), Some("WORD"));
        assert_eq!(cdr.as_str(), Some("今"));
        assert!(!v.is_list(), "dotted pair is not a proper list");
    }

    #[test]
    fn parses_nested_dotted_in_list() {
        // BASIC-SPLIT result shape.
        let v = parse_ok(r#"(((:WORD . "今日") (:MISC . "。")))"#);
        // outer → list of (list of two cons)
        let outer: Vec<&Sexp> = v.list_iter().unwrap().collect();
        assert_eq!(outer.len(), 1);
        let inner: Vec<&Sexp> = outer[0].list_iter().unwrap().collect();
        assert_eq!(inner.len(), 2);
        let (k0, v0) = inner[0].as_cons().unwrap();
        assert_eq!(k0.as_keyword(), Some("WORD"));
        assert_eq!(v0.as_str(), Some("今日"));
        let (k1, v1) = inner[1].as_cons().unwrap();
        assert_eq!(k1.as_keyword(), Some("MISC"));
        assert_eq!(v1.as_str(), Some("。"));
    }

    #[test]
    fn parses_vector() {
        let v = parse_ok("#(1 2 3)");
        assert!(v.is_vector());
        let items: Vec<i64> = if let Sexp::Vector(items) = v {
            items.iter().map(|e| e.as_i64().unwrap()).collect()
        } else { panic!() };
        assert_eq!(items, vec![1, 2, 3]);
    }

    #[test]
    fn parses_empty_vector() {
        let v = parse_ok("#()");
        assert!(v.is_vector());
        if let Sexp::Vector(items) = v { assert!(items.is_empty()); }
    }

    // --- chars ----------------------------------------------------------

    #[test]
    fn parses_char_single_ascii() {
        assert_eq!(parse_ok(r"#\a").as_char(), Some('a'));
        assert_eq!(parse_ok(r"#\)").as_char(), Some(')'));
    }

    #[test]
    fn parses_char_single_unicode() {
        assert_eq!(parse_ok(r"#\今").as_char(), Some('今'));
    }

    #[test]
    fn parses_char_named() {
        assert_eq!(parse_ok(r"#\Space").as_char(), Some(' '));
        assert_eq!(parse_ok(r"#\Newline").as_char(), Some('\n'));
        assert_eq!(parse_ok(r"#\Tab").as_char(), Some('\t'));
    }

    #[test]
    fn parses_char_sbcl_hex() {
        assert_eq!(parse_ok(r"#\U4ECA").as_char(), Some('今'));
        assert_eq!(parse_ok(r"#\U0061").as_char(), Some('a'));
    }

    #[test]
    fn unknown_char_name_errors_clearly() {
        let err = parse(r"#\HIRAGANA_LETTER_HA").unwrap_err();
        assert!(err.message.contains("HIRAGANA_LETTER_HA"),
                "error mentions the offending name: {}", err.message);
        assert!(err.message.contains(":CHAR"),
                "error suggests the tagged-form workaround: {}", err.message);
    }

    // --- datum labels ---------------------------------------------------

    #[test]
    fn parses_datum_label_simple() {
        // #1=expression defines, #1# references.
        let v = parse_ok(r#"(#1="ab" #1# #1#)"#);
        let strings: Vec<&str> = v.list_iter().unwrap().map(|e| e.as_str().unwrap()).collect();
        assert_eq!(strings, vec!["ab", "ab", "ab"]);
    }

    #[test]
    fn parses_datum_label_circular_ref() {
        // Real shape from SIMPLIFY-NGRAMS: bigram alist with shared `, `.
        let v = parse_ok(r#"("text" ("、" #1=", " "，" #1#))"#);
        let outer: Vec<&Sexp> = v.list_iter().unwrap().collect();
        assert_eq!(outer.len(), 2);
        assert_eq!(outer[0].as_str(), Some("text"));
        let inner: Vec<&Sexp> = outer[1].list_iter().unwrap().collect();
        assert_eq!(inner.len(), 4);
        assert_eq!(inner[0].as_str(), Some("、"));
        assert_eq!(inner[1].as_str(), Some(", "));
        assert_eq!(inner[2].as_str(), Some("，"));
        assert_eq!(inner[3].as_str(), Some(", "));
    }

    #[test]
    fn undefined_label_errors() {
        let err = parse("#5#").unwrap_err();
        assert!(err.message.contains("undefined"), "{}", err.message);
    }

    // --- whole captures from the .103 probe -----------------------------

    #[test]
    fn parses_normalize_capture() {
        let v = parse_ok(r#"("今日はいい天気ですね。" :CONTEXT :DEFAULT)"#);
        let items: Vec<&Sexp> = v.list_iter().unwrap().collect();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].as_str(), Some("今日はいい天気ですね。"));
        assert_eq!(items[1].as_keyword(), Some("CONTEXT"));
        assert_eq!(items[2].as_keyword(), Some("DEFAULT"));
    }

    #[test]
    fn parses_to_normal_char_with_hex() {
        // After the trace_capture char-tag fix this becomes (:CHAR 20170 …)
        // but raw SBCL output still has to read for direct REPL use.
        let v = parse_ok(r"(#\U4ECA :CONTEXT :DEFAULT)");
        let items: Vec<&Sexp> = v.list_iter().unwrap().collect();
        assert_eq!(items[0].as_char(), Some('今'));
        assert_eq!(items[1].as_keyword(), Some("CONTEXT"));
    }

    #[test]
    fn parses_to_normal_char_tagged() {
        // The shape trace_capture.lisp will emit after the char-tag fix.
        let v = parse_ok(r#"((:CHAR 20170) :CONTEXT :DEFAULT)"#);
        let items: Vec<&Sexp> = v.list_iter().unwrap().collect();
        let inner: Vec<&Sexp> = items[0].list_iter().unwrap().collect();
        assert_eq!(inner[0].as_keyword(), Some("CHAR"));
        assert_eq!(inner[1].as_i64(), Some(20170));
    }

    #[test]
    fn parses_basic_split_result() {
        let v = parse_ok(
            r#"(((:WORD . "今日はいい天気ですね") (:MISC . "。")))"#
        );
        let outer: Vec<&Sexp> = v.list_iter().unwrap().collect();
        assert_eq!(outer.len(), 1);
        let pairs: Vec<&Sexp> = outer[0].list_iter().unwrap().collect();
        assert_eq!(pairs.len(), 2);
    }

    #[test]
    fn parses_consecutive_char_groups_result() {
        // (((2 . 5) (7 . 10))) — list of dotted-pair pairs.
        let v = parse_ok("(((2 . 5) (7 . 10)))");
        let outer: Vec<&Sexp> = v.list_iter().unwrap().collect();
        let pairs: Vec<&Sexp> = outer[0].list_iter().unwrap().collect();
        let (a, b) = pairs[0].as_cons().unwrap();
        assert_eq!(a.as_i64(), Some(2));
        assert_eq!(b.as_i64(), Some(5));
        let (c, d) = pairs[1].as_cons().unwrap();
        assert_eq!(c.as_i64(), Some(7));
        assert_eq!(d.as_i64(), Some(10));
    }

    #[test]
    fn parses_simplify_ngrams_result_with_t() {
        // SIMPLIFY-NGRAMS returns two values — the second is T.
        let v = parse_ok(r#"("今日はいい天気ですね. " T)"#);
        let items: Vec<&Sexp> = v.list_iter().unwrap().collect();
        assert_eq!(items[0].as_str(), Some("今日はいい天気ですね. "));
        assert!(items[1].is_t());
    }

    #[test]
    fn rejects_trailing_garbage() {
        assert!(parse("(1 2) extra").is_err());
    }

    #[test]
    fn handles_line_comments() {
        let v = parse_ok("; comment\n(1 2 3)");
        assert_eq!(v.list_iter().unwrap().count(), 3);
    }
}
