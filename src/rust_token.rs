use proc_macro2::{LineColumn, Span, TokenTree};
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
    pub fn span(&self) -> Span {
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

/// Extended version of [TokenTree].
///
/// Most notably, adding the semi-tokens [Self::NewLine] and [Self::Spaces].
pub(crate) enum Token {
    Ident {
        ident: proc_macro2::Ident,
        span: OnceCell<CSpan>,
    },
    Punct {
        punct: proc_macro2::Punct,
        span: OnceCell<CSpan>,
    },
    Literal {
        literal: proc_macro2::Literal,
        span: OnceCell<CSpan>,
    },
    Group {
        group: proc_macro2::Group,
        span: OnceCell<CSpan>,
        tokens: Rc<Vec<Token>>,
    },
    NewLine {
        line: u16,
        indent: u16,
    },
    Spaces(u16),
}

impl Token {
    pub fn tt(&self) -> Option<TokenTree> {
        match self {
            Self::Ident { ident, .. } => Some(ident.clone().into()),
            Self::Punct { punct, .. } => Some(punct.clone().into()),
            Self::Literal { literal, .. } => Some(literal.clone().into()),
            Self::Group { group, .. } => Some(group.clone().into()),
            _ => None,
        }
    }

    pub fn span(&self) -> Option<&CSpan> {
        match self {
            Self::Ident { ident, span, .. } => Some(span.get_or_init(|| CSpan::from(ident.span()))),
            Self::Punct { punct, span, .. } => Some(span.get_or_init(|| CSpan::from(punct.span()))),
            Self::Literal { literal, span, .. } => {
                Some(span.get_or_init(|| CSpan::from(literal.span())))
            }
            Self::Group { group, span, .. } => Some(span.get_or_init(|| CSpan::from(group.span()))),
            _ => None,
        }
    }

    pub fn group(&self) -> Option<(&proc_macro2::Group, TokenBuffer)> {
        match self {
            Self::Group { group, tokens, .. } => Some((
                group,
                TokenBuffer {
                    tokens: Rc::clone(tokens),
                    pos: 0,
                },
            )),
            _ => None,
        }
    }

    pub fn punct(&self) -> Option<&proc_macro2::Punct> {
        match self {
            Self::Punct { punct, .. } => Some(punct),
            _ => None,
        }
    }

    pub fn eq_punct(&self, ch: char) -> bool {
        if let Some(punct) = self.punct() {
            punct.as_char() == ch
        } else {
            false
        }
    }
}

impl From<TokenTree> for Token {
    fn from(value: TokenTree) -> Self {
        match value {
            TokenTree::Group(group) => {
                // TODO: check if this works properly when group delimiter is Delimiter::None
                let tokens = Self::tokens_from(
                    group.stream(),
                    Some(group.span_open().end()),
                    Some(group.span_close().start()),
                );
                Self::Group {
                    group,
                    span: OnceCell::new(),
                    tokens: Rc::new(tokens),
                }
            }
            TokenTree::Ident(ident) => Self::Ident {
                ident,
                span: OnceCell::new(),
            },
            TokenTree::Punct(punct) => Self::Punct {
                punct,
                span: OnceCell::new(),
            },
            TokenTree::Literal(literal) => Self::Literal {
                literal,
                span: OnceCell::new(),
            },
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
                            tokens.push(Token::NewLine {
                                line: line as u16,
                                indent: 0,
                            });
                        }
                    }
                    tokens.push(Token::NewLine {
                        line: start.line as u16,
                        indent: start.column as u16,
                    });
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
            Self::Ident { ident, .. } => ident.fmt(f),
            Self::Punct { punct, .. } => punct.fmt(f),
            Self::Literal { literal, .. } => literal.fmt(f),
            Self::Group { group, tokens, .. } => f
                .debug_struct("Group")
                .field("delimiter", &group.delimiter())
                .field("span", &group.span())
                .field("tokens", tokens)
                .finish(),
            Self::NewLine { line, indent } => f
                .debug_struct("NewLine")
                .field("line", line)
                .field("indent", indent)
                .finish(),
            Self::Spaces(n) => f.debug_tuple("Spaces").field(n).finish(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TokenBuffer {
    tokens: Rc<Vec<Token>>,
    pos: usize,
}

impl TokenBuffer {
    pub fn reset(&mut self) {
        self.pos = 0;
    }

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

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn reached_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }
}

impl FromIterator<TokenTree> for TokenBuffer {
    fn from_iter<T: IntoIterator<Item = TokenTree>>(iter: T) -> Self {
        Self {
            tokens: Rc::new(Token::tokens_from(iter, None, None)),
            pos: 0,
        }
    }
}
