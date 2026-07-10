pub(crate) mod escape;
pub(crate) mod parse_buffer;
pub(crate) mod parsing;
pub(crate) mod rust_token;
pub(crate) mod span;
pub mod diagnostic;

macro_rules! match_unwrap {
    ($var:ident in $pattern:pat = $expr:expr) => {{
        let $pattern = $expr else { unreachable!() };
        $var
    }};
}

pub(crate) use match_unwrap;
use std::cmp::max;
use std::str::FromStr;

// #[cfg(feature = "nightly_proc_macro_span")]
// #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
// pub(crate) struct LineColumn {
//     /// 1-indexed
//     pub line: u32,
//     /// 0-indexed
//     pub column: u32,
// }
// 
// #[cfg(feature = "nightly_proc_macro_span")]
// impl From<proc_macro2::LineColumn> for LineColumn {
//     fn from(value: proc_macro2::LineColumn) -> Self {
//         Self {
//             line: value.line as u32,
//             column: value.column as u32,
//         }
//     }
// }
// 
// #[cfg(feature = "nightly_proc_macro_span")]
// impl From<LineColumn> for proc_macro2::LineColumn {
//     fn from(value: LineColumn) -> Self {
//         Self {
//             line: value.line as usize,
//             column: value.column as usize,
//         }
//     }
// }

#[allow(unused)]
pub(crate) trait ResultOrOption<T, E> {
    fn is_good(&self) -> bool;
    fn is_bad(&self) -> bool;
}

impl<T, E> ResultOrOption<T, E> for Result<T, E> {
    fn is_good(&self) -> bool {
        self.is_ok()
    }

    fn is_bad(&self) -> bool {
        self.is_err()
    }
}

impl<T> ResultOrOption<T, ()> for Option<T> {
    fn is_good(&self) -> bool {
        self.is_some()
    }

    fn is_bad(&self) -> bool {
        self.is_none()
    }
}

fn longest_true_chain(mut iter: impl Iterator<Item = bool>) -> usize {
    let mut max_value = 0;
    let mut current = 0;
    while let Some(value) = iter.next() {
        if value {
            current += 1;
        } else {
            max_value = max(max_value, current);
            current = 0;
        }
    }
    max_value
}

pub(crate) trait LiteralRawStringExt {
    fn raw_string(string: &str) -> Self;
    fn raw_string_value(&self) -> String;
}
impl LiteralRawStringExt for proc_macro2::Literal {
    fn raw_string(string: &str) -> Self {
        let num_hashes = longest_true_chain(string.chars().map(|c| c == '#')) + 1;
        let hashes = std::iter::repeat_n('#', num_hashes);
        let mut src = String::from("r");
        src.extend(hashes.clone());
        src.push('"');
        src.push_str(string);
        src.push('"');
        src.extend(hashes);
        Self::from_str(&src).unwrap()
    }

    fn raw_string_value(&self) -> String {
        let string = self.to_string();
        let string = string.trim_start_matches('r').trim_matches('#');
        String::from(&string[1..string.len() - 1])
    }
}
