use rustc_literal_escaper::MixedUnit;
use std::borrow::Cow;
use std::fmt::Write;

/// Append `byte` into `string` as if `string` is the content of a Python bytes literal, escape when necessary.
///
/// <https://docs.python.org/3/reference/lexical_analysis.html#escape-sequences>
fn append_byte_escaped_for_python(string: &mut String, byte: u8) {
    match byte {
        b'\\' => string.push_str(r"\\"),
        b'\'' => string.push_str(r"\'"),
        b'"' => string.push_str("\\\""),
        b'\x07' => string.push_str(r"\a"),
        b'\x08' => string.push_str(r"\b"),
        b'\x0c' => string.push_str(r"\f"),
        b'\n' => string.push_str(r"\n"),
        b'\r' => string.push_str(r"\r"),
        b'\t' => string.push_str(r"\t"),
        b'\x0b' => string.push_str(r"\v"),
        byte if byte.is_ascii() && !byte.is_ascii_control() => string.push(byte as char),
        byte => write!(string, r"\x{byte:02x}").unwrap(),
    }
}

/// Append `char` into `string` as if `string` is the content of a Python string literal, escape when necessary.
///
/// <https://docs.python.org/3/reference/lexical_analysis.html#escape-sequences>
fn append_char_escaped_for_python(string: &mut String, char: char) {
    if char.is_ascii() {
        append_byte_escaped_for_python(string, char as u8);
    } else {
        string.push(char);
    }
}

pub(crate) trait ToPyLiteralRepr {
    fn to_py_literal_repr(&self, dest: &mut String);
    fn new_py_literal_repr(&self) -> String {
        let mut string = String::new();
        self.to_py_literal_repr(&mut string);
        string
    }
}
impl ToPyLiteralRepr for char {
    fn to_py_literal_repr(&self, dest: &mut String) {
        dest.push('\'');
        append_char_escaped_for_python(dest, *self);
        dest.push('\'');
    }
}
impl ToPyLiteralRepr for &str {
    fn to_py_literal_repr(&self, dest: &mut String) {
        dest.push('"');
        self.chars().for_each(|c| append_char_escaped_for_python(dest, c));
        dest.push('"');
    }
}
impl ToPyLiteralRepr for u8 {
    fn to_py_literal_repr(&self, dest: &mut String) {
        dest.push_str("b'");
        append_byte_escaped_for_python(dest, *self);
        dest.push('\'');
    }
}
impl ToPyLiteralRepr for &[u8] {
    fn to_py_literal_repr(&self, dest: &mut String) {
        dest.push_str("b\"");
        self.iter().for_each(|&b| append_byte_escaped_for_python(dest, b));
        dest.push('"');
    }
}

enum Unquoted<'a> {
    Normal(&'a str),
    Raw(&'a str),
}

fn unquote_rust_string_repr(repr: &'_ str) -> Unquoted<'_> {
    fn unquote_raw(repr: &str) -> &str {
        let i = repr.find('"').unwrap() + 1;
        &repr[i..(repr.len() - i)]
    }
    match repr.as_bytes() {
        [b'"', content @ .., b'"'] => Unquoted::Normal(str::from_utf8(content).unwrap()),
        [b'r', quoted @ ..] => Unquoted::Raw(unquote_raw(str::from_utf8(quoted).unwrap())),
        [b'b' | b'c', b'"', content @ .., b'"'] => Unquoted::Normal(str::from_utf8(content).unwrap()),
        [b'b' | b'c', b'r', quoted @ ..] => Unquoted::Raw(unquote_raw(str::from_utf8(quoted).unwrap())),
        _ => panic!("Invalid Rust string literal repr: {repr:?}"),
    }
}

pub(crate) fn unescape_rust_str_literal(repr: &str) -> Cow<'_, str> {
    match unquote_rust_string_repr(repr) {
        Unquoted::Normal(repr) => {
            let mut string = String::new();
            rustc_literal_escaper::unescape_str(repr, |_, result| string.push(result.unwrap()));
            string.into()
        }
        Unquoted::Raw(repr) => repr.into(),
    }
}

pub(crate) fn unescape_rust_bytes_literal(repr: &str) -> Cow<'_, [u8]> {
    match unquote_rust_string_repr(repr) {
        Unquoted::Normal(repr) => {
            let mut bytes = Vec::new();
            rustc_literal_escaper::unescape_byte_str(repr, |_, result| bytes.push(result.unwrap()));
            bytes.into()
        }
        Unquoted::Raw(repr) => repr.as_bytes().into(),
    }
}

pub(crate) fn unescape_rust_c_str_literal(repr: &str) -> Cow<'_, [u8]> {
    match unquote_rust_string_repr(repr) {
        Unquoted::Normal(repr) => {
            let mut bytes = Vec::new();
            rustc_literal_escaper::unescape_c_str(repr, |_, result| match result.unwrap() {
                MixedUnit::Char(char) => char
                    .get()
                    .encode_utf8(&mut [0; 4])
                    .as_bytes()
                    .iter()
                    .for_each(|&byte| bytes.push(byte)),
                MixedUnit::HighByte(byte) => bytes.push(byte.get()),
            });
            bytes.into()
        }
        Unquoted::Raw(repr) => repr.as_bytes().into(),
    }
}

pub(crate) fn unescape_rust_char_literal(repr: &str) -> char {
    rustc_literal_escaper::unescape_char(&repr[1..(repr.len() - 1)]).unwrap()
}

pub(crate) fn unescape_rust_byte_literal(repr: &str) -> u8 {
    rustc_literal_escaper::unescape_byte(&repr[2..(repr.len() - 1)]).unwrap()
}
