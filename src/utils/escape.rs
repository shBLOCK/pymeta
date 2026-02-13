use rustc_literal_escaper::MixedUnit;
use std::fmt::Write;

/// Append `byte` into `string` as if `string` is the content of a Python bytes literal, escape when necessary.
///
/// https://docs.python.org/3/reference/lexical_analysis.html#escape-sequences
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
/// https://docs.python.org/3/reference/lexical_analysis.html#escape-sequences
fn append_char_escaped_for_python(string: &mut String, char: char) {
    if char.is_ascii() {
        append_byte_escaped_for_python(string, char as u8);
    } else {
        string.push(char);
    }
}

enum Unquoted<'a> {
    Normal(&'a str),
    Raw(&'a str),
}

fn unquote_rust_string_repr(repr: &str) -> Unquoted {
    fn unquote_raw(repr: &str) -> &str {
        let i = repr.find('"').unwrap() + 1;
        &repr[i..(repr.len() - i)]
    }
    match repr.as_bytes() {
        [b'"', content @ .., b'"'] => {
            Unquoted::Normal(unsafe { str::from_utf8_unchecked(content) })
        }
        [b'r', quoted @ ..] => {
            Unquoted::Raw(unquote_raw(unsafe { str::from_utf8_unchecked(quoted) }))
        }
        [b'b' | b'c', b'"', content @ .., b'"'] => {
            Unquoted::Normal(unsafe { str::from_utf8_unchecked(content) })
        }
        [b'b' | b'c', b'r', quoted @ ..] => {
            Unquoted::Raw(unquote_raw(unsafe { str::from_utf8_unchecked(quoted) }))
        }
        _ => panic!("Invalid Rust string literal repr: {repr:?}"),
    }
}

pub(crate) fn rust_string_repr_to_python_str_repr(repr: &str) -> String {
    let mut py_repr = String::from("\"");

    match unquote_rust_string_repr(repr) {
        Unquoted::Normal(repr) => {
            rustc_literal_escaper::unescape_str(repr, |_, result| {
                append_char_escaped_for_python(&mut py_repr, result.expect("Unescape failed"));
            });
        }
        Unquoted::Raw(repr) => {
            repr.chars()
                .for_each(|char| append_char_escaped_for_python(&mut py_repr, char));
        }
    }

    py_repr += "\"";
    py_repr
}

pub(crate) fn rust_bytes_repr_to_python_bytes_repr(repr: &str) -> String {
    let mut py_repr = String::from("b\"");

    match unquote_rust_string_repr(repr) {
        Unquoted::Normal(repr) => {
            rustc_literal_escaper::unescape_byte_str(repr, |_, result| {
                append_byte_escaped_for_python(&mut py_repr, result.expect("Unescape failed"))
            });
        }
        Unquoted::Raw(repr) => {
            repr.chars()
                .for_each(|char| append_byte_escaped_for_python(&mut py_repr, char as u8));
        }
    }

    py_repr += "\"";
    py_repr
}

pub(crate) fn rust_c_string_repr_to_python_bytes_repr(repr: &str) -> String {
    let mut py_repr = String::from("b\"");

    match unquote_rust_string_repr(repr) {
        Unquoted::Normal(repr) => {
            rustc_literal_escaper::unescape_c_str(repr, |_, result| {
                match result.expect("Unescape failed") {
                    MixedUnit::Char(char) => char
                        .get()
                        .encode_utf8(&mut [0; 4])
                        .as_bytes()
                        .into_iter()
                        .for_each(|&byte| append_byte_escaped_for_python(&mut py_repr, byte)),
                    MixedUnit::HighByte(byte) => {
                        append_byte_escaped_for_python(&mut py_repr, byte.get())
                    }
                }
            });
        }
        Unquoted::Raw(repr) => {
            repr.chars()
                .for_each(|char| append_byte_escaped_for_python(&mut py_repr, char as u8));
        }
    }

    py_repr += "\"";
    py_repr
}

pub(crate) fn rust_char_repr_to_python_str_repr(repr: &str) -> String {
    let mut py_repr = String::from("'");
    append_char_escaped_for_python(
        &mut py_repr,
        rustc_literal_escaper::unescape_char(&repr[1..(repr.len() - 1)]).expect("Unescape failed"),
    );
    py_repr += "'";
    py_repr
}

pub(crate) fn rust_byte_repr_to_python_bytes_repr(repr: &str) -> String {
    let mut py_repr = String::from("b'");
    append_byte_escaped_for_python(
        &mut py_repr,
        rustc_literal_escaper::unescape_byte(&repr[2..(repr.len() - 1)]).expect("Unescape failed"),
    );
    py_repr += "'";
    py_repr
}
