use super::*;
use std::error::Error;
use std::fmt::{self, Display};
use std::str::FromStr;

impl FromStr for Value {
    type Err = ParseError;

    /// Parses JSON data into a `Value`.
    ///
    /// # Examples
    ///
    /// Valid JSON:
    ///
    /// ```
    /// use rustic_json::Value;
    /// use rustic_json::{Arr, Num, Obj, Str};
    ///
    /// assert_eq!(
    ///     r#"
    ///     [
    ///         null,
    ///         true,
    ///         false,
    ///         0,
    ///         12.34,
    ///         -56E-78,
    ///         "Foo:\n- bar/café ♥'🧡",
    ///         "Foo:\u000a- bar\/caf\u00E9 \u2665'\uD83E\uDDE1",
    ///         [
    ///             {
    ///                 "<\u0000>": "",
    ///                 "<\">" : [ ],
    ///                 "<\\>":{}
    ///             }
    ///         ]
    ///     ]
    ///     "#.parse::<Value>(),
    ///     Ok(Value::Array(Arr::from([
    ///         Value::Null,
    ///         Value::Boolean(true),
    ///         Value::Boolean(false),
    ///         Value::Number(Num::from(0)),
    ///         Value::Number(Num::new(12.34).expect("finite number")),
    ///         Value::Number(Num::new(-5.6e-77).expect("finite number")),
    ///         Value::String(Str::from("Foo:\n- bar/café ♥'🧡")),
    ///         Value::String(Str::from("Foo:\n- bar/café ♥'🧡")),
    ///         Value::Array(Arr::from([
    ///             Value::Object(Obj::from([
    ///                 (Str::from("<\0>"), Value::String(Str::from(""))),
    ///                 (Str::from(r#"<">"#), Value::Array(Arr::new())),
    ///                 (Str::from(r"<\>"), Value::Object(Obj::new())),
    ///             ])),
    ///         ])),
    ///     ])))
    /// );
    /// ```
    ///
    /// Invalid JSON:
    ///
    /// ```
    /// # use rustic_json::Value;
    /// use rustic_json::{ParseError, ParseErrorKind, ParseErrorPosition};
    ///
    /// macro_rules! m {
    ///     ($str:expr, $err:ident, $msg:literal, $line:literal, $col:literal) => {{
    ///         let res = $str.parse::<Value>();
    ///         assert_eq!(res, Err(ParseError {
    ///             kind: ParseErrorKind::$err,
    ///             position: ParseErrorPosition { line: $line, column: $col } }
    ///         ));
    ///         assert_eq!(
    ///             format!("{}", res.unwrap_err()),
    ///             concat!($msg, " at line ", $line, " column ", $col)
    ///         );
    ///     }};
    /// }
    /// m!("", PrematureEof, "premature end of data", 1, 1);
    /// m!("nul", PrematureEof, "premature end of data", 1, 4);
    /// m!("nulx", UnexpectedChar, "unexpected character", 1, 4);
    /// m!("-e", UnexpectedChar, "unexpected character", 1, 2);
    /// m!("1.e2", UnexpectedChar, "unexpected character", 1, 3);
    /// m!("[1.0e]", UnexpectedChar, "unexpected character", 1, 6);
    /// m!("1E400", TooBigNumber, "too big number", 1, 1);
    /// m!(r#""foo\u123xbar""#, UnexpectedChar, "unexpected character", 1, 10);
    /// m!(r#""foo\uD800bar""#, UnexpectedChar, "unexpected character", 1, 11);
    /// m!(r#""foo\uD800\uD7FFbar""#, InvalidUtf16SurrogatePair, "invalid UTF-16 surrogate pair", 1, 5);
    /// m!(r#""foo\xbar""#, UnexpectedChar, "unexpected character", 1, 6);
    /// m!(r#""foo
    /// bar""#, UnexpectedChar, "unexpected character", 1, 5);
    /// m!("[1;2;3]", UnexpectedChar, "unexpected character", 1, 3);
    /// m!("[
    ///     1,
    ///     2,
    /// ]", UnexpectedChar, "unexpected character", 4, 1);
    /// m!("[
    ///     1,
    ///     2,
    /// ", PrematureEof, "premature end of data", 4, 1);
    /// m!(r#"{a:1,b:2,c:3}"#, UnexpectedChar, "unexpected character", 1, 2);
    /// m!(r#"{"a"=1,"b"=2,"c"=3}"#, UnexpectedChar, "unexpected character", 1, 5);
    /// m!(r#"{"a":1;"b":2;"c":3}"#, UnexpectedChar, "unexpected character", 1, 7);
    /// m!(r#"{
    ///     "a":1,
    ///     "b":2,
    /// }"#, UnexpectedChar, "unexpected character", 4, 1);
    /// m!(r#"{
    ///     "a":1,
    ///     "b":2,
    /// "#, PrematureEof, "premature end of data", 4, 1);
    /// m!("(1,2,3)", UnexpectedChar, "unexpected character", 1, 1);
    /// m!("[1,2,3].", UnexpectedChar, "unexpected character", 1, 8);
    /// ```
    #[doc(alias("parse", "decode", "deserialize"))]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        helper::parse(s)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub position: ParseErrorPosition,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {}", self.kind, self.position)
    }
}

impl Error for ParseError {}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ParseErrorKind {
    PrematureEof,
    UnexpectedChar,
    TooBigNumber,
    InvalidUtf16SurrogatePair,
}

impl Display for ParseErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::PrematureEof => "premature end of data",
            Self::UnexpectedChar => "unexpected character",
            Self::TooBigNumber => "too big number",
            Self::InvalidUtf16SurrogatePair => "invalid UTF-16 surrogate pair",
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct ParseErrorPosition {
    /// 1-based.
    pub line: usize,
    /// 1-based, char offset.
    pub column: usize,
}

impl Display for ParseErrorPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {} column {}", self.line, self.column)
    }
}

mod helper;
