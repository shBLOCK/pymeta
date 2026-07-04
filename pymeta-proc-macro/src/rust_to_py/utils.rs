use super::PY_MARKER;
use crate::utils::rust_token::{Group, Ident, Literal, Punct, Token, TokenBuffer};
use crate::utils::span::CSpan;
use proc_macro2::{Delimiter, Spacing, Span, TokenStream, TokenTree};
use quote::TokenStreamExt;
use std::rc::Rc;

pub(crate) trait PunctEx {
    fn as_str(&self) -> &'static str;
}
impl PunctEx for Punct {
    fn as_str(&self) -> &'static str {
        match self.inner().as_char() {
            '!' => "!",
            '#' => "#",
            '$' => "$",
            '%' => "%",
            '&' => "&",
            '\'' => "'",
            '*' => "*",
            '+' => "+",
            ',' => ",",
            '-' => "-",
            '.' => ".",
            '/' => "/",
            ':' => ":",
            ';' => ";",
            '<' => "<",
            '=' => "=",
            '>' => ">",
            '?' => "?",
            '@' => "@",
            '^' => "^",
            '|' => "|",
            '~' => "~",
            _ => unreachable!(),
        }
    }
}

pub(super) trait TokenOptionEx {
    fn ident(&self) -> Option<Rc<Ident>>;
    fn punct(&self) -> Option<Rc<Punct>>;
    fn literal(&self) -> Option<Rc<Literal>>;
    fn group(&self) -> Option<Rc<Group>>;

    fn eq_punct(&self, c: char) -> bool;
    fn eq_group(&self, delimiter: Delimiter) -> bool;

    fn expect_punct(&self, c: char) -> Option<Rc<Punct>>;
    fn expect_ident_by(&self, f: impl FnOnce(&str) -> bool) -> Option<Rc<Ident>>;
    fn expect_ident<'a>(&self, ident: impl Into<&'a str>) -> Option<Rc<Ident>>;
    fn expect_group_by(&self, f: impl FnOnce(Delimiter) -> bool) -> Option<Rc<Group>>;
    fn expect_group(&self, delimiter: Delimiter) -> Option<Rc<Group>>;
}
impl TokenOptionEx for Option<&Token> {
    fn ident(&self) -> Option<Rc<Ident>> {
        self.and_then(|it| it.ident())
    }

    fn punct(&self) -> Option<Rc<Punct>> {
        self.and_then(|it| it.punct())
    }

    fn literal(&self) -> Option<Rc<Literal>> {
        self.and_then(|it| it.literal())
    }

    fn group(&self) -> Option<Rc<Group>> {
        self.and_then(|it| it.group())
    }

    fn eq_punct(&self, c: char) -> bool {
        match self {
            Some(t) => t.eq_punct(c),
            None => false,
        }
    }

    fn eq_group(&self, delimiter: Delimiter) -> bool {
        match self {
            Some(t) => t.eq_group(delimiter),
            None => false,
        }
    }

    fn expect_punct(&self, c: char) -> Option<Rc<Punct>> {
        match self {
            Some(Token::Punct(punct)) if punct.eq_punct(c) => Some(punct.clone()),
            _ => None,
        }
    }

    fn expect_ident_by(&self, f: impl FnOnce(&str) -> bool) -> Option<Rc<Ident>> {
        match self {
            Some(Token::Ident(it)) if f(it.inner().to_string().as_str()) => Some(it.clone()),
            _ => None,
        }
    }

    fn expect_ident<'a>(&self, ident: impl Into<&'a str>) -> Option<Rc<Ident>> {
        self.expect_ident_by(|s| s == ident.into())
    }

    fn expect_group_by(&self, f: impl FnOnce(Delimiter) -> bool) -> Option<Rc<Group>> {
        match self {
            Some(Token::Group(it)) if f(it.delimiter()) => Some(it.clone()),
            _ => None,
        }
    }

    fn expect_group(&self, delimiter: Delimiter) -> Option<Rc<Group>> {
        self.expect_group_by(|it| it == delimiter)
    }
}

pub(super) trait TokenBufferEx {
    fn is_current_py_marker_escaped(&self) -> bool;
    fn is_py_marker_escape(&self) -> bool;
    fn py_marker_escape_span(&self) -> Rc<CSpan>;
    fn skip_py_marker_escape(&mut self);
    fn read_unescaped_py_marker_escape(&mut self) -> Rc<Punct>;
    fn is_py_marker_start(&self) -> bool;
    fn is_py_marker_end(&self) -> bool;
    fn is_indent_block(&self) -> bool;
}
impl TokenBufferEx for TokenBuffer {
    fn is_current_py_marker_escaped(&self) -> bool {
        assert!(self.current().eq_punct(PY_MARKER));
        self.peek(-1).eq_punct('<') && self.peek(1).eq_punct('>')
    }

    fn is_py_marker_escape(&self) -> bool {
        self.peek(1).eq_punct(PY_MARKER) && self.seeked(1).unwrap().is_current_py_marker_escaped()
    }

    fn py_marker_escape_span(&self) -> Rc<CSpan> {
        let start = self.current().unwrap().span();
        let end = self.peek(2).unwrap().span();
        Rc::new(CSpan::from(
            start
                .inner()
                .join(end.inner())
                .unwrap_or_else(|| self.peek(1).unwrap().span().inner()),
        ))
    }

    fn skip_py_marker_escape(&mut self) {
        self.seek(3).unwrap();
    }

    fn read_unescaped_py_marker_escape(&mut self) -> Rc<Punct> {
        let mut punct = self.peek(1).unwrap().clone().punct().unwrap();
        Rc::make_mut(&mut punct).set_span(self.py_marker_escape_span().inner());
        self.skip_py_marker_escape();
        punct
    }

    fn is_py_marker_start(&self) -> bool {
        self.current().eq_punct(PY_MARKER) && !self.is_current_py_marker_escaped()
    }

    fn is_py_marker_end(&self) -> bool {
        self.current().eq_punct(PY_MARKER) && !self.is_current_py_marker_escaped()
    }

    fn is_indent_block(&self) -> bool {
        self.current().eq_punct(':') && self.peek(1).eq_group(Delimiter::Brace)
    }
}

pub(super) trait DelimiterEx {
    fn left_str(self) -> Option<&'static str>;
    fn right_str(self) -> Option<&'static str>;
    fn left_right_str(self) -> Option<&'static str>;
}
impl DelimiterEx for Delimiter {
    fn left_str(self) -> Option<&'static str> {
        match self {
            Delimiter::Parenthesis => Some("("),
            Delimiter::Brace => Some("{"),
            Delimiter::Bracket => Some("["),
            Delimiter::None => None,
        }
    }

    fn right_str(self) -> Option<&'static str> {
        match self {
            Delimiter::Parenthesis => Some(")"),
            Delimiter::Brace => Some("}"),
            Delimiter::Bracket => Some("]"),
            Delimiter::None => None,
        }
    }

    fn left_right_str(self) -> Option<&'static str> {
        match self {
            Delimiter::Parenthesis => Some("()"),
            Delimiter::Brace => Some("{}"),
            Delimiter::Bracket => Some("[]"),
            Delimiter::None => None,
        }
    }
}

#[derive(Debug)]
pub(crate) struct SimpleRustPath {
    segments: Box<[Rc<Ident>]>,
    is_root: bool,
    tokens: Box<[Token]>,
}

impl SimpleRustPath {
    pub fn parse(tokens: &mut TokenBuffer) -> Result<Self, (Span, String)> {
        tokens.try_run_or_rewind(|tokens| {
            let mut segments = Vec::new();
            let mut path_tokens = Vec::new();
            let mut is_first = true;
            let mut is_root = false;
            loop {
                let is_delim = if let [p1t @ Token::Punct(p1), p2t @ Token::Punct(p2), ..] =
                    tokens.slice(tokens.pos()..)
                    && p1.eq_punct(':')
                    && p2.eq_punct(':')
                    && p1.inner().spacing() == Spacing::Joint
                    && p2.inner().spacing() == Spacing::Alone
                {
                    path_tokens.push(p1t.clone());
                    path_tokens.push(p2t.clone());
                    tokens.seek(2).unwrap();
                    true
                } else {
                    false
                };

                if is_first {
                    is_root = is_delim;
                }
                if is_delim || is_first {
                    if let Some(seg_token @ Token::Ident(seg)) = tokens.read_one() {
                        segments.push(seg.clone());
                        path_tokens.push(seg_token.clone());
                    } else {
                        return Err((
                            tokens.get_current_span_for_diagnostics(),
                            "Invalid Rust path: expected identifier".into(),
                        ));
                    }
                } else {
                    break;
                }

                is_first = false;
            }
            Ok(Self {
                segments: segments.into_boxed_slice(),
                is_root,
                tokens: path_tokens.into_boxed_slice(),
            })
        })
    }
}

impl PartialEq for SimpleRustPath {
    fn eq(&self, other: &Self) -> bool {
        self.is_root == other.is_root
            && self.segments.len() == other.segments.len()
            && self
                .segments
                .iter()
                .zip(other.segments.iter())
                .all(|(a, b)| a.inner().to_string() == b.inner().to_string())
    }
}

impl Eq for SimpleRustPath {}

impl quote::ToTokens for SimpleRustPath {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append_all(self.tokens.iter().map(TokenTree::from));
    }
}
