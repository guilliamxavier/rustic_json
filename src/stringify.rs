use super::*;
use std::fmt::{Display, Formatter, Result, Write};

impl Display for Value {
    /// Formats a `Value` into JSON (compact or pretty-printed).
    ///
    /// This does compact formatting by default,
    /// and pretty-printing for the "alternate" form (`#` flag).
    ///
    /// # Examples
    ///
    /// ```
    /// use rustic_json::Value;
    /// use rustic_json::{Arr, Num, Obj, Str};
    ///
    /// let value = Value::Array(Arr::from([
    ///     Value::Null,
    ///     Value::Boolean(true),
    ///     Value::Boolean(false),
    ///     Value::Number(Num::from(0)),
    ///     Value::Number(Num::new(12.34).expect("finite number")),
    ///     Value::Number(Num::new(-56e-78).expect("finite number")),
    ///     Value::String(Str::from("Foo:\n- bar/café ♥'🧡")),
    ///     Value::Array(Arr::from([
    ///         Value::Object(Obj::from([
    ///             (Str::from("<\0>"), Value::String(Str::from(""))),
    ///             (Str::from(r#"<">"#), Value::Array(Arr::new())),
    ///             (Str::from(r"<\>"), Value::Object(Obj::new())),
    ///         ])),
    ///     ])),
    /// ]));
    /// assert_eq!(
    ///     format!("{}", value),
    ///     r#"[null,true,false,0,12.34,-5.6e-77,"Foo:\n- bar/café ♥'🧡",[{"<\u0000>":"","<\">":[],"<\\>":{}}]]"#
    /// );
    /// // or pretty-print:
    /// assert_eq!(format!("{:#}", value), r#"[
    ///     null,
    ///     true,
    ///     false,
    ///     0,
    ///     12.34,
    ///     -5.6e-77,
    ///     "Foo:\n- bar/café ♥'🧡",
    ///     [
    ///         {
    ///             "<\u0000>": "",
    ///             "<\">": [],
    ///             "<\\>": {}
    ///         }
    ///     ]
    /// ]"#);
    /// ```
    #[doc(alias("stringify", "encode", "serialize"))]
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result {
        helper::write_value(fmt, self, 0)
    }
}

mod helper {
    use super::*;

    pub(super) fn write_value(fmt: &mut Formatter, value: &Value, depth: usize) -> Result {
        match value {
            Value::Null => fmt.write_str("null"),
            Value::Boolean(b) => write!(fmt, "{}", *b),
            Value::Number(num) => write_number(fmt, *num),
            Value::String(str) => write_string(fmt, str),
            Value::Array(arr) => write_array(fmt, arr, depth),
            Value::Object(obj) => write_object(fmt, obj, depth),
        }
    }

    fn write_number(fmt: &mut Formatter<'_>, num: Num) -> Result {
        let debug = format!("{:?}", num.get());
        fmt.write_str(debug.strip_suffix(".0").unwrap_or(&debug))
    }

    fn write_string(fmt: &mut Formatter<'_>, str: &Str) -> Result {
        fmt.write_char('"')?;
        for c in str.chars() {
            if let Ok(byte) = u8::try_from(c) {
                if let Some(escape) = STRINGIFY_ESCAPE[usize::from(byte)] {
                    write!(fmt, "\\{}", char::from(escape))?;
                    continue;
                }
                if byte < MIN_VALID_STRING_CHAR {
                    write!(fmt, "\\u{:04x}", byte)?;
                    continue;
                }
            }
            fmt.write_char(c)?;
        }
        fmt.write_char('"')
    }

    fn write_array(fmt: &mut Formatter<'_>, arr: &Arr, depth: usize) -> Result {
        fmt.write_char('[')?;
        if !arr.is_empty() {
            {
                let depth = depth + 1;
                for (i, element) in arr.iter().enumerate() {
                    if i != 0 {
                        fmt.write_char(',')?;
                    }
                    pretty_writeln_indent(fmt, depth)?;
                    write_value(fmt, element, depth)?;
                }
            }
            pretty_writeln_indent(fmt, depth)?;
        }
        fmt.write_char(']')
    }

    fn write_object(fmt: &mut Formatter<'_>, obj: &Obj, depth: usize) -> Result {
        fmt.write_char('{')?;
        if !obj.is_empty() {
            {
                let depth = depth + 1;
                for (i, (key, value)) in obj.iter().enumerate() {
                    if i != 0 {
                        fmt.write_char(',')?;
                    }
                    pretty_writeln_indent(fmt, depth)?;
                    write_string(fmt, key)?;
                    fmt.write_char(':')?;
                    if is_pretty(fmt) {
                        fmt.write_char(' ')?;
                    }
                    write_value(fmt, value, depth)?;
                }
            }
            pretty_writeln_indent(fmt, depth)?;
        }
        fmt.write_char('}')
    }

    fn pretty_writeln_indent(fmt: &mut Formatter, depth: usize) -> Result {
        if is_pretty(fmt) {
            writeln!(fmt)?;
            for _ in 0..depth {
                fmt.write_str("    ")?;
            }
        }
        Ok(())
    }

    fn is_pretty(fmt: &Formatter) -> bool {
        fmt.alternate()
    }
}
