use proc_macro2::Span;

pub(crate) mod logging;
pub(crate) mod py_source;
pub(crate) mod rust_token;

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

macro_rules! match_unwrap {
    ($var:ident in $pattern:pat = $expr:expr) => {{
        let $pattern = $expr else { unreachable!() };
        $var
    }};
}
pub(crate) use match_unwrap;

// pub(crate) trait FloatEx: Sized {
//     fn finite(self) -> Option<Self>;
// }
// macro_rules! impl_float_ex {
//     ($typ:ty) => {
//         impl FloatEx for $typ {
//             fn finite(self) -> Option<Self> {
//                 match self.is_finite() {
//                     true => Some(self),
//                     false => None,
//                 }
//             }
//         }
//     };
// }
// impl_float_ex!(f32);
// impl_float_ex!(f64);
