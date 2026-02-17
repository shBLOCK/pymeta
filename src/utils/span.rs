#[cfg(feature = "nightly_proc_macro_span")]
use crate::utils::LineColumn;
use proc_macro2::Span;
use std::cell::OnceCell;
use std::fmt::{Debug, Formatter};

pub(crate) trait SpanOptionEx {
    fn join_or_fallback(self, other: Self) -> Span;
}

impl SpanOptionEx for Option<Span> {
    fn join_or_fallback(self, other: Self) -> Span {
        match (self, other) {
            (Some(lhs), Some(rhs)) => lhs.join(rhs).unwrap_or_else(Span::call_site),
            (Some(lhs), None) => lhs,
            (None, Some(rhs)) => rhs,
            (None, None) => Span::call_site(),
        }
    }
}

pub(crate) trait SpanEx {
    fn join_or_fallback(self, other: Option<Span>) -> Span;
}

impl SpanEx for Span {
    fn join_or_fallback(self, other: Option<Span>) -> Span {
        Some(self).join_or_fallback(other)
    }
}

/// Caching wrapper around [Span].
///
/// [Span] operations can be expansive, see: <https://github.com/rust-lang/rust/issues/149331#issuecomment-3580649306>
pub(crate) struct CSpan {
    span: Span,
    #[cfg(feature = "nightly_proc_macro_span")]
    start: OnceCell<LineColumn>,
    #[cfg(feature = "nightly_proc_macro_span")]
    end: OnceCell<LineColumn>,
}

impl CSpan {
    pub fn inner(&self) -> Span {
        self.span
    }

    #[cfg(feature = "nightly_proc_macro_span")]
    pub fn start(&self) -> LineColumn {
        *self.start.get_or_init(|| self.span.start().into())
    }

    #[cfg(feature = "nightly_proc_macro_span")]
    pub fn end(&self) -> LineColumn {
        *self.end.get_or_init(|| self.span.end().into())
    }
}

impl From<Span> for CSpan {
    fn from(value: Span) -> Self {
        Self {
            span: value,
            #[cfg(feature = "nightly_proc_macro_span")]
            start: OnceCell::new(),
            #[cfg(feature = "nightly_proc_macro_span")]
            end: OnceCell::new(),
        }
    }
}

impl Debug for CSpan {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "C{:?}", self.span)
    }
}

impl From<&CSpan> for Span {
    fn from(value: &CSpan) -> Self {
        value.span
    }
}
