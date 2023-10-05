use super::*;
use internal::State;

type ParseResult<T> = Result<T, ParseError>;

use ParseErrorKind as K;

pub(super) fn parse(str: &str) -> ParseResult<Value> {
    let mut parser = Parser::new(str);
    let element = parser.parse_element()?;
    if parser.state.peek_char().is_ok() {
        return Err(parser.state.error(K::UnexpectedChar));
    }
    Ok(element)
}

struct Parser<'s> {
    state: State<'s>,
}

impl Parser<'_> {
    fn new(str: &str) -> Parser<'_> {
        Parser {
            state: State::new(str),
        }
    }

    fn parse_element(&mut self) -> ParseResult<Value> {
        self.skip_ws();
        let value = self.parse_value()?;
        self.skip_ws();
        Ok(value)
    }

    fn parse_value(&mut self) -> ParseResult<Value> {
        let peeked = self.state.peek_char()?;
        match peeked {
            'n' => self.expect_str("null").and(Ok(Value::Null)),
            't' => self.expect_str("true").and(Ok(Value::Boolean(true))),
            'f' => self.expect_str("false").and(Ok(Value::Boolean(false))),
            '-' | '0'..='9' => self.parse_number().map(Value::Number),
            '"' => self.parse_string().map(Value::String),
            '[' => self.parse_array().map(Value::Array),
            '{' => self.parse_object().map(Value::Object),
            _ => Err(self.state.error(K::UnexpectedChar)),
        }
    }

    fn expect_str(&mut self, str: &'static str) -> ParseResult<()> {
        for c in str.chars() {
            self.expect_char(c)?;
        }
        Ok(())
    }

    fn expect_char(&mut self, expected: char) -> ParseResult<()> {
        let peeked = self.state.peek_char()?;
        if peeked != expected {
            return Err(self.state.error(K::UnexpectedChar));
        }
        self.state.skip_char(peeked);
        Ok(())
    }

    fn parse_number(&mut self) -> ParseResult<Num> {
        let num_error = self.state.error(K::TooBigNumber);
        let mut buf = String::new();
        macro_rules! consume_char {
            ($buf:ident, $peeked:ident) => {{
                $buf.push($peeked);
                self.state.skip_char($peeked);
            }};
        }
        macro_rules! accept_digits {
            ($buf:ident) => {
                while let Ok(digit @ '0'..='9') = self.state.peek_char() {
                    consume_char!($buf, digit);
                }
            };
        }
        macro_rules! require_digits {
            ($buf:ident) => {{
                let peeked = self.state.peek_char()?;
                if !matches!(peeked, '0'..='9') {
                    return Err(self.state.error(K::UnexpectedChar));
                }
                consume_char!($buf, peeked);
                accept_digits!($buf);
            }};
        }

        // integer: /[-]?(0|[1-9][0-9]*)/
        if let Ok(minus @ '-') = self.state.peek_char() {
            consume_char!(buf, minus);
        }
        let peeked = self.state.peek_char()?;
        match peeked {
            '0' => consume_char!(buf, peeked),
            '1'..='9' => {
                consume_char!(buf, peeked);
                accept_digits!(buf);
            }
            _ => return Err(self.state.error(K::UnexpectedChar)),
        }

        // fraction: /([.][0-9]+)?/
        if let Ok(dot @ '.') = self.state.peek_char() {
            consume_char!(buf, dot);
            require_digits!(buf);
        }

        // exponent: /([Ee][+-]?[0-9]+)?/
        if let Ok(e @ ('E' | 'e')) = self.state.peek_char() {
            consume_char!(buf, e);
            if let Ok(sign @ ('+' | '-')) = self.state.peek_char() {
                consume_char!(buf, sign);
            }
            require_digits!(buf);
        }

        let f: f64 = buf.parse().expect("valid f64 grammar");
        debug_assert!(!f.is_nan()); // only finite or infinite (too big)
        Num::new(f).ok_or(num_error)
    }

    fn parse_string(&mut self) -> ParseResult<Str> {
        self.expect_char('"')?;
        let mut buf = String::new();
        loop {
            let peeked = self.state.peek_char()?;
            if peeked == '"' {
                self.state.skip_char(peeked);
                break;
            }
            if peeked == '\\' {
                buf.push(self.parse_escape()?);
            } else if peeked >= char::from(MIN_VALID_STRING_CHAR) {
                buf.push(peeked);
                self.state.skip_char(peeked);
            } else {
                return Err(self.state.error(K::UnexpectedChar));
            }
        }
        Ok(Str::from(buf))
    }

    fn parse_escape(&mut self) -> ParseResult<char> {
        let utf16_decode_error = self.state.error(K::InvalidUtf16SurrogatePair);
        self.expect_char('\\')?;
        let peeked = self.state.peek_char()?;
        if let Ok(byte) = u8::try_from(peeked) {
            if let Some(raw) = PARSE_ESCAPE[usize::from(byte)] {
                self.state.skip_char(peeked);
                return Ok(char::from(raw));
            }
        }
        if peeked == 'u' {
            self.state.skip_char(peeked);
            let unit = self.parse_hex_4()?;
            if let Ok(decoded) = char::decode_utf16([unit]).next().expect("not empty") {
                return Ok(decoded);
            }
            // expect second half of surrogate pair
            self.expect_char('\\')?;
            self.expect_char('u')?;
            let unit_2 = self.parse_hex_4()?;
            let result = char::decode_utf16([unit, unit_2])
                .next()
                .expect("not empty");
            return result.or(Err(utf16_decode_error));
        }
        Err(self.state.error(K::UnexpectedChar))
    }

    fn parse_hex_4(&mut self) -> ParseResult<u16> {
        let mut buf: u16 = 0;
        for i in (0..4).rev() {
            let peeked = self.state.peek_char()?;
            let Some(hex) = peeked.to_digit(16) else {
                return Err(self.state.error(K::UnexpectedChar));
            };
            let hex = u16::try_from(hex).expect("fits in");
            debug_assert_eq!(hex, hex & 0xF);
            buf |= hex << (4 * i);
            self.state.skip_char(peeked);
        }
        Ok(buf)
    }

    fn parse_array(&mut self) -> ParseResult<Arr> {
        self.expect_char('[')?;
        self.skip_ws();
        let mut buf = Vec::new();
        loop {
            let peeked = self.state.peek_char()?;
            if peeked == ']' {
                self.state.skip_char(peeked);
                break;
            }
            if !buf.is_empty() {
                self.expect_char(',')?;
            }
            buf.push(self.parse_element()?);
        }
        Ok(Arr::from(buf))
    }

    fn parse_object(&mut self) -> ParseResult<Obj> {
        self.expect_char('{')?;
        self.skip_ws();
        let mut buf = Vec::new();
        loop {
            let peeked = self.state.peek_char()?;
            if peeked == '}' {
                self.state.skip_char(peeked);
                break;
            }
            if !buf.is_empty() {
                self.expect_char(',')?;
            }
            buf.push(self.parse_member()?);
        }
        Ok(Obj::from_iter(buf))
    }

    fn parse_member(&mut self) -> ParseResult<(Str, Value)> {
        self.skip_ws();
        let key = self.parse_string()?;
        self.skip_ws();
        self.expect_char(':')?;
        let value = self.parse_element()?;
        Ok((key, value))
    }

    fn skip_ws(&mut self) {
        while let Ok(ws @ (' ' | '\n' | '\r' | '\t')) = self.state.peek_char() {
            self.state.skip_char(ws);
        }
    }
}

mod internal {
    use super::*;
    use std::iter::Peekable;
    use std::str::Chars;

    pub(super) struct State<'s> {
        chars: Peekable<Chars<'s>>,
        position: ParseErrorPosition,
    }

    impl State<'_> {
        const ONE: usize = 1;

        pub(super) fn new(str: &str) -> State<'_> {
            State {
                chars: str.chars().peekable(),
                position: ParseErrorPosition {
                    line: Self::ONE,
                    column: Self::ONE,
                },
            }
        }

        pub(super) fn peek_char(&mut self) -> ParseResult<char> {
            match self.chars.peek() {
                Some(&peeked) => Ok(peeked),
                None => Err(self.error(K::PrematureEof)),
            }
        }

        pub(super) fn skip_char(&mut self, peeked: char) {
            let next = self.chars.next().expect("should have just peeked");
            debug_assert_eq!(next, peeked);
            if next == '\n' {
                self.position.line += 1;
                self.position.column = Self::ONE;
            } else {
                self.position.column += 1;
            }
        }

        pub(super) fn error(&self, kind: K) -> ParseError {
            ParseError {
                kind,
                position: self.position,
            }
        }
    }
}
