use proc_macro2::{LineColumn, Span};
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

#[allow(unused)]
pub(crate) trait SpanEx {
    fn start_span(&self) -> Span;
    fn end_span(&self) -> Span;

    fn join_or_fallback(self, other: Option<Span>) -> Span;
}

#[allow(clippy::needless_return)]
impl SpanEx for Span {
    fn start_span(&self) -> Span {
        cfg_select! {
            feature = "proc_macro" => self.unwrap().start().into(),
            _ => *self,
        }
    }

    fn end_span(&self) -> Span {
        cfg_select! {
            feature = "proc_macro" => self.unwrap().end().into(),
            _ => *self,
        }
    }

    fn join_or_fallback(self, other: Option<Span>) -> Span {
        Some(self).join_or_fallback(other)
    }
}

/// Caching wrapper around [Span].
///
/// [Span] operations can be expansive, see: <https://github.com/rust-lang/rust/issues/149331#issuecomment-3580649306>
pub(crate) struct CSpan {
    span: Span,
    start: OnceCell<LineColumn>,
    end: OnceCell<LineColumn>,
}
#[allow(unused)]
impl CSpan {
    pub fn inner(&self) -> Span {
        self.span
    }

    pub fn start(&self) -> LineColumn {
        *self.start.get_or_init(|| self.span.start())
    }

    pub fn end(&self) -> LineColumn {
        *self.end.get_or_init(|| self.span.end())
    }

    pub fn start_span(&self) -> CSpan {
        self.span.start_span().into()
    }
    pub fn end_span(&self) -> CSpan {
        self.span.end_span().into()
    }
}

impl From<Span> for CSpan {
    fn from(value: Span) -> Self {
        Self {
            span: value,
            start: OnceCell::new(),
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
