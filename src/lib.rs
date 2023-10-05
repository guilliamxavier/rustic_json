//! Rudimentary implementation of [JSON] in [Rust], **for demonstration purpose**.
//!
//! [JSON]: https://www.json.org
//! [Rust]: https://www.rust-lang.org
//!
//! [Repository](https://github.com/guilliamxavier/rustic_json)
//!
//! The main item is the [`Value`] enum, which can be:
//! - constructed:
//!   - by parsing JSON data via [its `FromStr` impl](Value#impl-FromStr-for-Value),
//!   - or manually, optionally via its various \[`Try`\]`From` impls or with the [`json!`] macro;
//! - modified manually (through pattern matching);
//! - and formatted into JSON via [its `Display` impl](Value#impl-Display-for-Value).

#![forbid(unsafe_code)]

macro_rules! value_impl_from {
    ($param:tt: $typ:ty => $body:expr) => {
        impl From<$typ> for Value {
            #[inline]
            fn from($param: $typ) -> Self {
                $body
            }
        }
    };
}

macro_rules! value_enum {
    ($($variant:ident$(($typ:ty))?,)+) => {
        /// Representation of a JSON value.
        #[derive(Debug, PartialEq, Eq, Clone)]
        pub enum Value {
            $($variant$(($typ))?,)+
        }

        $($(value_impl_from!(val: $typ => Self::$variant(val));)?)+
    };
}

value_enum! {
    Null,
    Boolean(bool),
    Number(Num),
    String(Str),
    Array(Arr),
    Object(Obj),
}

mod num;

pub use num::Num;
pub type Str = std::borrow::Cow<'static, str>;
pub type Arr = Vec<Value>;
pub type Obj = std::collections::BTreeMap<Str, Value>;

value_impl_from!(_: () => Self::Null);

impl TryFrom<f64> for Value {
    /// NaN or infinity.
    type Error = f64;

    #[inline]
    fn try_from(f: f64) -> Result<Self, Self::Error> {
        Num::new(f).ok_or(f).map(Self::Number)
    }
}
value_impl_from!(i: i32 => Self::Number(Num::from(i)));
value_impl_from!(u: u32 => Self::Number(Num::from(u)));

value_impl_from!(str: &'static str => Self::String(Str::from(str)));
value_impl_from!(string: String => Self::String(Str::from(string)));

/// Convenience macro for constructing a [`Value`] from a JSON-like literal.
///
/// This also:
/// - interpolates variables/constants and parenthesized expressions
///   _(negative numbers also require parentheses)_,
/// - allows trailing commas in objects and arrays,
/// - automatically supports comments.
///
/// # Panics
///
/// This will panic (at runtime) for invalid numbers (NaN or infinity).
///
/// # Examples
///
/// Basic literals:
///
/// ```
/// use rustic_json::json;
/// use rustic_json::Value;
/// use rustic_json::{Arr, Num, Obj, Str};
///
/// assert_eq!(
///     json!({
///         "a": null,
///         "b": true,
///         "c": false,
///         "d": 1234,
///         "e": 0.5,
///         "f": "hello",
///         "g": [{}],
///         "h": {"":[]}
///     }),
///     Value::Object(Obj::from([
///         (Str::from("a"), Value::Null),
///         (Str::from("b"), Value::Boolean(true)),
///         (Str::from("c"), Value::Boolean(false)),
///         (Str::from("d"), Value::Number(Num::from(1234))),
///         (Str::from("e"), Value::Number(Num::new(0.5).expect("finite number"))),
///         (Str::from("f"), Value::String(Str::from("hello"))),
///         (Str::from("g"), Value::Array(Arr::from([Value::Object(Obj::new())]))),
///         (Str::from("h"), Value::Object(Obj::from([(Str::from(""), Value::Array(Arr::new()))]))),
///     ]))
/// );
/// ```
///
/// Interpolation, trailing commas, comments:
///
/// ```
/// # use rustic_json::json;
/// # use rustic_json::Value;
/// # use rustic_json::{Arr, Num, Obj, Str};
/// #
/// let string_key: String = "oof".chars().rev().collect();
/// let string_value: String = "bar".to_ascii_uppercase();
/// const EMPTY_LIST: Arr = Arr::new();
/// assert_eq!(
///     json!({
///         "unit": (),
///         "bool_expression": (string_key == "foo" && string_value == "BAR"),
///         "negative_number": (-1234),
///         string_key: string_value,
///         (concat!("nested", '_', "lists")): [
///             EMPTY_LIST,
///             EMPTY_LIST, // <-- trailing comma (in array)
///         ], // <-- trailing comma (in object)
///     }),
///     Value::Object(Obj::from([
///         (Str::from("unit"), Value::Null),
///         (Str::from("bool_expression"), Value::Boolean(true)),
///         (Str::from("negative_number"), Value::Number(Num::from(-1234))),
///         (Str::from("foo"), Value::String(Str::from("BAR"))),
///         (Str::from("nested_lists"), Value::Array(Arr::from([
///             Value::Array(Arr::new()),
///             Value::Array(Arr::new()),
///         ]))),
///     ]))
/// );
/// ```
///
/// Panic (invalid number):
///
/// ```should_panic
/// # use rustic_json::json;
/// #
/// let _ = json!({ "imaginary_number": (f64::sqrt(-1.0)) });
/// ```
///
/// Compilation error (missing parentheses around expression):
///
/// ```compile_fail
/// # use rustic_json::json;
/// #
/// let _ = json!({ "negative_one": -1 });
/// ```
#[macro_export]
macro_rules! json {
    ({}) => {
        $crate::Value::Object($crate::Obj::new())
    };
    ({ $($key:tt : $value:tt),+ $(,)? }) => {
        $crate::Value::Object($crate::Obj::from([$(($crate::Str::from($key), json!($value))),+]))
    };
    ([]) => {
        $crate::Value::Array($crate::Arr::new())
    };
    ([ $($element:tt),+ $(,)? ]) => {
        $crate::Value::Array($crate::Arr::from([$(json!($element)),+]))
    };
    (null) => {
        $crate::Value::Null
    };
    ($other:expr) => {
        $crate::Value::try_from($other).expect(stringify!($other))
    };
}

macro_rules! escape_tables {
    ($($escape:literal: $raw:literal,)+ + $($extra:literal,)+) => {
        static PARSE_ESCAPE: [Option<u8>; u8::MAX as usize + 1] = {
            let mut tmp = [None; u8::MAX as usize + 1];
            $(tmp[$escape as usize] = Some($raw);)+
            $(tmp[$extra as usize] = Some($extra);)+
            tmp
        };
        static STRINGIFY_ESCAPE: [Option<u8>; u8::MAX as usize + 1] = {
            let mut tmp = [None; u8::MAX as usize + 1];
            $(tmp[$raw as usize] = Some($escape);)+
            tmp
        };
    };
}

escape_tables! {
    b'"': b'"',
    b'\\': b'\\',
    b'b': b'\x08',
    b'f': b'\x0C',
    b'n': b'\n',
    b'r': b'\r',
    b't': b'\t',
    +
    b'/',
}

const MIN_VALID_STRING_CHAR: u8 = b'\x20';

mod parse;
mod stringify;

pub use parse::{ParseError, ParseErrorKind, ParseErrorPosition};
