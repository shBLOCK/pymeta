pub(crate) enum LiteralRepr<'a> {
    Str(&'a str),
    Char(&'a str),
    Bytes(&'a str),
    Byte(&'a str),
    CStr(&'a str),
    Integer { number: &'a str, suffix: Option<&'a str> },
    Float { number: &'a str, suffix: Option<&'a str> },
}
impl LiteralRepr<'_> {
    pub fn parse(repr: &str) -> LiteralRepr<'_> {
        match repr.as_bytes() {
            [b'"', ..] | [b'r', b'"', ..] => LiteralRepr::Str(repr),
            [b'\'', ..] => LiteralRepr::Char(repr),
            [b'b', b'"', ..] | [b'b', b'r', b'"', ..] => LiteralRepr::Bytes(repr),
            [b'b', b'\'', ..] => LiteralRepr::Byte(repr),
            [b'c', ..] => LiteralRepr::CStr(repr),
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
                    LiteralRepr::Float { number, suffix }
                } else {
                    LiteralRepr::Integer { number, suffix }
                }
            }
            _ => panic!("Failed to parse literal: {repr:?}"),
        }
    }
}
