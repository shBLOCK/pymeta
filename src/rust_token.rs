use proc_macro2::{LineColumn, Span, TokenTree};
use std::cell::OnceCell;
use std::fmt::{Debug, Formatter};
use std::ops::Range;

/// Cached Span
pub(crate) struct CSpan {
    span: Span,
    _byte_range: OnceCell<Range<usize>>,
    _start: OnceCell<LineColumn>,
    _end: OnceCell<LineColumn>,
}

impl CSpan {
    pub fn byte_range(&self) -> Range<usize> {
        self._byte_range.get_or_init(|| self.span.byte_range()).clone()
    }

    pub fn start(&self) -> LineColumn {
        *self._start.get_or_init(|| self.span.start())
    }

    pub fn end(&self) -> LineColumn {
        *self._end.get_or_init(|| self.span.end())
    }
}

impl From<Span> for CSpan {
    fn from(value: Span) -> Self {
        Self {
            span: value,
            _byte_range: OnceCell::new(),
            _start: OnceCell::new(),
            _end: OnceCell::new(),
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

pub(crate) struct Token {
    tt: TokenTree,
    span: OnceCell<CSpan>,
}

impl Token {
    pub fn span(&self) -> &CSpan {
        self.span.get_or_init(|| CSpan::from(self.tt.span()))
    }
}

impl From<TokenTree> for Token {
    fn from(value: TokenTree) -> Self {
        Self {
            tt: value,
            span: OnceCell::new(),
        }
    }
}

impl Debug for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.tt.fmt(f)
    }
}

#[derive(Debug)]
pub(crate) struct TokenBuffer {
    tokens: Vec<Token>,
    pos: usize,
}

impl TokenBuffer {
    pub fn seek(&mut self, offset: isize) -> Option<&Token> {
        if !(0..self.tokens.len() as isize).contains(&(self.pos as isize + offset)) {
            return None;
        }
        self.pos = (self.pos as isize + offset) as usize;
        Some(&self.tokens[self.pos])
    }

    pub fn current(&self) -> &Token {
        &self.tokens[self.pos]
    }

    pub fn peek(&self, offset: isize) -> Option<&Token> {
        self.tokens.get(self.pos.checked_add_signed(offset)?)
    }
}

impl FromIterator<TokenTree> for TokenBuffer {
    fn from_iter<T: IntoIterator<Item=TokenTree>>(iter: T) -> Self {
        Self {
            tokens: iter.into_iter().map(Token::from).collect(),
            pos: 0,
        }
    }
}