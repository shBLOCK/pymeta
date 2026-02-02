use proc_macro2::{Delimiter, LineColumn, Span, TokenTree};
use std::cell::OnceCell;
use std::fmt::{Debug, Formatter};
use std::ops::Range;
use std::rc::Rc;

/// Caching wrapper around [Span].
///
/// [Span] operations can be expansive, see: https://github.com/rust-lang/rust/issues/149331#issuecomment-3580649306
pub(crate) struct CSpan {
    span: Span,
    _byte_range: OnceCell<Range<usize>>,
    _start: OnceCell<LineColumn>,
    _end: OnceCell<LineColumn>,
}

impl CSpan {
    pub fn inner(&self) -> Span {
        self.span
    }

    pub fn byte_range(&self) -> Range<usize> {
        self._byte_range
            .get_or_init(|| self.span.byte_range())
            .clone()
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

pub(crate) struct Ident {
    ident: proc_macro2::Ident,
    span: OnceCell<Rc<CSpan>>,
}

pub(crate) struct Punct {
    punct: proc_macro2::Punct,
    span: OnceCell<Rc<CSpan>>,
}

pub(crate) struct Literal {
    literal: proc_macro2::Literal,
    span: OnceCell<Rc<CSpan>>,
}

pub(crate) struct Group {
    group: proc_macro2::Group,
    span: OnceCell<Rc<CSpan>>,
    tokens: Rc<[Token]>,
}

impl Group {
    pub fn tokens(&self) -> TokenBuffer {
        TokenBuffer::from(&self.tokens)
    }

    pub fn delimiter(&self) -> Delimiter {
        self.group.delimiter()
    }
}

impl From<proc_macro2::Group> for Group {
    fn from(value: proc_macro2::Group) -> Self {
        // TODO: check if this works properly when group delimiter is Delimiter::None
        let tokens = Token::tokens_from(
            value.stream(),
            Some(value.span_open().end()),
            Some(value.span_close().start()),
        );
        Self {
            group: value,
            span: OnceCell::new(),
            tokens: Rc::from(tokens),
        }
    }
}

impl Debug for Group {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Group")
            .field("delimiter", &self.group.delimiter())
            .field("span", &self.group.span())
            .field("tokens", &self.tokens)
            .finish()
    }
}

macro_rules! impl_token_struct_common {
    ($struct_name:ident, $inner_name:ident) => {
        impl $struct_name {
            pub fn span(&self) -> Rc<CSpan> {
                Rc::clone(
                    self.span
                        .get_or_init(|| Rc::new(CSpan::from(self.$inner_name.span()))),
                )
            }
            
            pub fn inner(&self) -> &proc_macro2::$struct_name {
                &self.$inner_name
            }
        }
    };
}
impl_token_struct_common!(Ident, ident);
impl_token_struct_common!(Punct, punct);
impl_token_struct_common!(Literal, literal);
impl_token_struct_common!(Group, group);

macro_rules! delegate_token_struct_common {
    ($struct_name:ident, $inner_name:ident) => {
        impl Debug for $struct_name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                self.$inner_name.fmt(f)
            }
        }

        impl From<proc_macro2::$struct_name> for $struct_name {
            fn from(value: proc_macro2::$struct_name) -> Self {
                Self {
                    $inner_name: value,
                    span: OnceCell::new(),
                }
            }
        }
    };
}
delegate_token_struct_common!(Ident, ident);
delegate_token_struct_common!(Punct, punct);
delegate_token_struct_common!(Literal, literal);

/// Extended version of [TokenTree].
///
/// Most notably, adding the semi-tokens [Self::NewLine] and [Self::Spaces].
#[derive(Clone)]
pub(crate) enum Token {
    Ident(Rc<Ident>),
    Punct(Rc<Punct>),
    Literal(Rc<Literal>),
    Group(Rc<Group>),
    NewLine(LineColumn),
    Spaces(u16),
}

macro_rules! token_get_token_struct_fn {
    ($name:ident, $fn_name:ident) => {
        pub fn $fn_name(&self) -> Option<Rc<$name>> {
            match self {
                Self::$name(it) => Some(Rc::clone(it)),
                _ => None,
            }
        }
    };
}

impl Token {
    pub fn span(&self) -> Option<Rc<CSpan>> {
        match self {
            Self::Ident(ident) => Some(ident.span()),
            Self::Punct(punct) => Some(punct.span()),
            Self::Literal(literal) => Some(literal.span()),
            Self::Group(group) => Some(group.span()),
            _ => None,
        }
    }

    token_get_token_struct_fn!(Ident, ident);
    token_get_token_struct_fn!(Punct, punct);
    token_get_token_struct_fn!(Literal, literal);
    token_get_token_struct_fn!(Group, group);

    pub fn newline(&self) -> Option<LineColumn> {
        match self {
            Self::NewLine(lc) => Some(*lc),
            _ => None,
        }
    }

    pub fn is_newline(&self) -> bool {
        match self {
            Self::NewLine(_) => true,
            _ => false,
        }
    }

    pub fn spaces(&self) -> Option<u16> {
        match self {
            Self::Spaces(s) => Some(*s),
            _ => None,
        }
    }

    pub fn is_spaces(&self) -> bool {
        match self {
            Self::Spaces(_) => true,
            _ => false,
        }
    }

    pub fn is_whitespace(&self) -> bool {
        match self {
            Self::Spaces(_) | Self::NewLine(_) => true,
            _ => false,
        }
    }

    pub fn eq_punct(&self, ch: char) -> bool {
        match self {
            Self::Punct(punct) => punct.punct.as_char() == ch,
            _ => false,
        }
    }
    
    pub fn eq_group(&self, delimiter: Delimiter) -> bool {
        match self {
            Self::Group(group) => group.group.delimiter() == delimiter,
            _ => false,
        }
    }
}

impl From<TokenTree> for Token {
    fn from(value: TokenTree) -> Self {
        match value {
            TokenTree::Ident(ident) => Self::Ident(Rc::new(ident.into())),
            TokenTree::Punct(punct) => Self::Punct(Rc::new(punct.into())),
            TokenTree::Literal(literal) => Self::Literal(Rc::new(literal.into())),
            TokenTree::Group(group) => Self::Group(Rc::new(group.into())),
        }
    }
}

impl Token {
    fn tokens_from(
        iter: impl IntoIterator<Item = TokenTree>,
        start_pos: Option<LineColumn>,
        end_pos: Option<LineColumn>,
    ) -> Vec<Self> {
        let mut tokens = Vec::new();

        let mut last_line = start_pos.map(|lc| lc.line);
        let mut last_column = start_pos.map(|lc| lc.column);

        let mut insert_spacing_tokens =
            |tokens: &mut Vec<Token>, start: LineColumn, end: LineColumn| {
                if Some(start.line) != last_line {
                    // add NewLine tokens
                    if let Some(last_line) = last_line {
                        for line in (last_line + 1)..start.line {
                            tokens.push(Token::NewLine(LineColumn { line, column: 0 }));
                        }
                    }
                    tokens.push(Token::NewLine(LineColumn {
                        line: start.line,
                        column: start.column,
                    }));
                } else if let Some(last_column) = last_column {
                    // add Spaces token
                    let spaces = start.column - last_column;
                    if spaces > 0 {
                        tokens.push(Token::Spaces(spaces as u16));
                    }
                }

                last_line = Some(end.line);
                last_column = Some(end.column);
            };

        for tt in iter {
            let token = Token::from(tt);
            let span = token.span().unwrap();
            insert_spacing_tokens(&mut tokens, span.start(), span.end());
            tokens.push(token);
        }
        if let Some(end_pos) = end_pos {
            insert_spacing_tokens(&mut tokens, end_pos, end_pos);
        }

        tokens
    }
}

impl Debug for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ident(ident) => ident.fmt(f),
            Self::Punct(punct) => punct.fmt(f),
            Self::Literal(literal) => literal.fmt(f),
            Self::Group(group) => group.fmt(f),
            Self::NewLine(lc) => f.debug_tuple("NewLine").field(lc).finish(),
            Self::Spaces(n) => f.debug_tuple("Spaces").field(n).finish(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TokenBuffer {
    tokens: Rc<[Token]>,
    pos: usize,
}

impl TokenBuffer {
    pub fn reset(&mut self) {
        self.pos = 0;
    }
    
    pub fn read_one(&mut self) -> Option<&Token> {
        self.current()?;
        self.pos += 1;
        Some(self.peek(-1).unwrap())
    }

    pub fn seek(&mut self, offset: isize) -> Option<&mut Self> {
        if !(0..=self.tokens.len() as isize).contains(&(self.pos as isize + offset)) {
            return None;
        }
        self.pos = (self.pos as isize + offset) as usize;
        Some(self)
    }

    pub fn seeked(&self, offset: isize) -> Option<Self> {
        let mut tokens = self.clone();
        tokens.seek(offset)?;
        Some(tokens)
    }
    
    pub fn seek_to_end_of_line(&mut self) -> &mut Self {
        while !self.peek(1).map_or(true, |t| t.is_newline()) {
            self.pos += 1;
        }
        self
    }

    pub fn current(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    pub fn peek(&self, offset: isize) -> Option<&Token> {
        self.tokens.get(self.pos.checked_add_signed(offset)?)
    }
    
    pub fn have_n_more(&self, n: usize) -> bool {
        self.pos + n <= self.tokens.len()
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn reached_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    pub fn iter(&self) -> core::slice::Iter<Token> {
        self.tokens.iter()
    }
    
    /// Find the "closest" span that makes sense for diagnostics purposes.
    /// 
    /// This is for use in contexts like "error occurred near here".
    pub fn get_current_span_for_diagnostics(&self) -> Span {
        if let Some(span) = self.current().and_then(Token::span) {
            return span.inner();
        }
        for search_direction in [1, -1] {
            for i in 1.. {
                let Some(token) = self.peek(i * search_direction) else { break };
                if let Some(span) = token.span() {
                    return span.inner();
                }
            }
        }
        Span::call_site()
    }
}

impl FromIterator<TokenTree> for TokenBuffer {
    fn from_iter<T: IntoIterator<Item = TokenTree>>(iter: T) -> Self {
        Self {
            tokens: Rc::from(Token::tokens_from(iter, None, None)),
            pos: 0,
        }
    }
}

impl From<&Rc<[Token]>> for TokenBuffer {
    fn from(value: &Rc<[Token]>) -> Self {
        Self {
            tokens: Rc::clone(value),
            pos: 0,
        }
    }
}
