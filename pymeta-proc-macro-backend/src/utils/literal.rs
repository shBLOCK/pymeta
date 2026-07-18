pub(crate) enum StrLiteralKind {
    Str,
    Char,
}
pub(crate) struct StrLiteralRepr<'a> {
    pub kind: StrLiteralKind,
    pub repr: &'a str,
}
pub(crate) enum BytesLiteralKind {
    Bytes,
    Byte,
    CStr,
}
pub(crate) struct BytesLiteralRepr<'a> {
    pub kind: BytesLiteralKind,
    pub repr: &'a str,
}
pub(crate) enum NumLiteralKind {
    Int,
    Float,
}
pub(crate) struct NumLiteralRepr<'a> {
    pub kind: NumLiteralKind,
    pub num: &'a str,
    pub suffix: Option<&'a str>,
}

pub(crate) enum LiteralRepr<'a> {
    Str(StrLiteralRepr<'a>),
    Bytes(BytesLiteralRepr<'a>),
    Num(NumLiteralRepr<'a>),
}

macro_rules! impl_from_repr_subtype {
    ($ty:ident, $ty_struct:ident) => {
        impl<'a> From<$ty_struct<'a>> for LiteralRepr<'a> {
            fn from(value: $ty_struct<'a>) -> Self {
                Self::$ty(value)
            }
        }
    };
}
impl_from_repr_subtype!(Str, StrLiteralRepr);
impl_from_repr_subtype!(Bytes, BytesLiteralRepr);
impl_from_repr_subtype!(Num, NumLiteralRepr);

impl LiteralRepr<'_> {
    pub fn parse(repr: &str) -> LiteralRepr<'_> {
        match repr.as_bytes() {
            [b'"', ..] | [b'r', b'"', ..] => StrLiteralRepr { kind: StrLiteralKind::Str, repr }.into(),
            [b'\'', ..] => StrLiteralRepr { kind: StrLiteralKind::Char, repr }.into(),
            [b'b', b'"', ..] | [b'b', b'r', b'"', ..] => {
                BytesLiteralRepr { kind: BytesLiteralKind::Bytes, repr }.into()
            }
            [b'b', b'\'', ..] => BytesLiteralRepr { kind: BytesLiteralKind::Byte, repr }.into(),
            [b'c', ..] => BytesLiteralRepr { kind: BytesLiteralKind::CStr, repr }.into(),
            repr @ [b'0'..=b'9', ..] => {
                let is_float = match repr {
                    [b'0', b'x', ..] => false,
                    repr if repr.iter().any(|b| matches!(b, b'.' | b'e' | b'E' | b'f')) => true,
                    _ => false,
                };

                let suffix_i = if is_float {
                    repr.iter().rposition(|&b| b == b'f')
                } else {
                    repr.iter().rposition(|&b| matches!(b, b'u' | b'i'))
                };
                let (number, suffix) = match suffix_i {
                    Some(i) => (
                        str::from_utf8(&repr[..i]).unwrap(),
                        Some(str::from_utf8(&repr[i..]).unwrap()),
                    ),
                    None => (str::from_utf8(repr).unwrap(), None),
                };

                if is_float {
                    NumLiteralRepr {
                        kind: NumLiteralKind::Float,
                        num: number,
                        suffix,
                    }
                    .into()
                } else {
                    NumLiteralRepr {
                        kind: NumLiteralKind::Int,
                        num: number,
                        suffix,
                    }
                    .into()
                }
            }
            _ => panic!("Failed to parse literal: {repr:?}"),
        }
    }
}
