pub(crate) mod escape;
pub(crate) mod rust_token;
pub(crate) mod span;
pub(crate) mod parse_buffer;

macro_rules! match_unwrap {
    ($var:ident in $pattern:pat = $expr:expr) => {{
        let $pattern = $expr else { unreachable!() };
        $var
    }};
}
pub(crate) use match_unwrap;

#[cfg(feature = "nightly_proc_macro_span")]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct LineColumn {
    /// 1-indexed
    pub line: u32,
    /// 0-indexed
    pub column: u32,
}

#[cfg(feature = "nightly_proc_macro_span")]
impl From<proc_macro2::LineColumn> for LineColumn {
    fn from(value: proc_macro2::LineColumn) -> Self {
        Self {
            line: value.line as u32,
            column: value.column as u32,
        }
    }
}

#[cfg(feature = "nightly_proc_macro_span")]
impl From<LineColumn> for proc_macro2::LineColumn {
    fn from(value: LineColumn) -> Self {
        Self {
            line: value.line as usize,
            column: value.column as usize,
        }
    }
}


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