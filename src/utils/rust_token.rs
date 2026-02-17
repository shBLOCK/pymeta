use crate::utils::span::CSpan;
use proc_macro2::{Delimiter, Span, TokenTree};
use std::cell::OnceCell;
use std::fmt::{Debug, Formatter};
use std::ops::RangeBounds;
use std::rc::Rc;
use std::slice::SliceIndex;

#[derive(Clone)]
pub(crate) struct Ident {
    ident: proc_macro2::Ident,
    span: OnceCell<Rc<CSpan>>,
}

#[derive(Clone)]
pub(crate) struct Punct {
    punct: proc_macro2::Punct,
    span: OnceCell<Rc<CSpan>>,
}

#[derive(Clone)]
pub(crate) struct Literal {
    literal: proc_macro2::Literal,
    span: OnceCell<Rc<CSpan>>,
}

#[derive(Clone)]
pub(crate) struct Group {
    group: proc_macro2::Group,
    span: OnceCell<Rc<CSpan>>,
    tokens: Rc<[Token]>,
}

impl Punct {
    pub fn eq_punct(&self, ch: char) -> bool {
        self.punct.as_char() == ch
    }
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
        let tokens = Token::tokens_from(value.stream());
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

            #[allow(unused)]
            pub fn inner(&self) -> &proc_macro2::$struct_name {
                &self.$inner_name
            }

            #[allow(unused)]
            pub fn set_span(&mut self, span: Span) {
                self.span.take();
                self.$inner_name.set_span(span);
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
/// Originally this contained `NewLine` and `Spaces` tokens derived from the [Span] information.
/// But [Span] information may not always be reliable, so these have been removed.
#[derive(Clone)]
pub(crate) enum Token {
    Ident(Rc<Ident>),
    Punct(Rc<Punct>),
    Literal(Rc<Literal>),
    Group(Rc<Group>),
}

macro_rules! token_get_token_struct_fn {
    ($name:ident, $fn_name:ident) => {
        #[allow(unused)]
        pub fn $fn_name(&self) -> Option<Rc<$name>> {
            match self {
                Self::$name(it) => Some(Rc::clone(it)),
                _ => None,
            }
        }
    };
}

impl Token {
    pub fn span(&self) -> Rc<CSpan> {
        match self {
            Self::Ident(ident) => ident.span(),
            Self::Punct(punct) => punct.span(),
            Self::Literal(literal) => literal.span(),
            Self::Group(group) => group.span(),
        }
    }

    token_get_token_struct_fn!(Ident, ident);
    token_get_token_struct_fn!(Punct, punct);
    token_get_token_struct_fn!(Literal, literal);
    token_get_token_struct_fn!(Group, group);

    pub fn eq_punct(&self, ch: char) -> bool {
        match self {
            Self::Punct(punct) => punct.eq_punct(ch),
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
    fn tokens_from(iter: impl IntoIterator<Item = TokenTree>) -> Vec<Self> {
        iter.into_iter().map(Token::from).collect()
    }
}

impl Debug for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ident(ident) => ident.fmt(f),
            Self::Punct(punct) => punct.fmt(f),
            Self::Literal(literal) => literal.fmt(f),
            Self::Group(group) => group.fmt(f),
        }
    }
}

/// A version of [proc_macro2::TokenStream] that's more useful for parsing.
#[derive(Clone, Debug)]
pub(crate) struct TokenBuffer {
    tokens: Rc<[Token]>,
    pos: usize,
}

impl TokenBuffer {
    pub fn read_one(&mut self) -> Option<&Token> {
        self.current()?;
        self.pos += 1;
        Some(self.peek(-1).unwrap())
    }

    pub fn seek(&mut self, offset: isize) -> Option<&mut Self> {
        let target = self.pos as isize + offset;
        if !(0..=self.tokens.len() as isize).contains(&target) {
            return None;
        }
        self.pos = target as usize;
        Some(self)
    }

    pub fn seeked(&self, offset: isize) -> Option<Self> {
        let mut tokens = self.clone();
        tokens.seek(offset)?;
        Some(tokens)
    }

    pub fn current(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    pub fn slice(&self, range: impl RangeBounds<usize> + SliceIndex<[Token], Output = [Token]>) -> &[Token] {
        &self.tokens[range]
    }

    pub fn peek(&self, offset: isize) -> Option<&Token> {
        self.tokens.get(self.pos.checked_add_signed(offset)?)
    }

    #[allow(unused)]
    pub fn pos(&self) -> usize {
        self.pos
    }

    #[allow(unused)]
    pub fn set_pos(&mut self, pos: usize) -> Result<(), ()> {
        if pos > self.tokens.len() {
            return Err(());
        }
        self.pos = pos;
        Ok(())
    }

    pub fn exausted(&self) -> bool {
        self.pos >= self.tokens.len()
    }
}

impl FromIterator<TokenTree> for TokenBuffer {
    fn from_iter<T: IntoIterator<Item = TokenTree>>(iter: T) -> Self {
        Self {
            tokens: Rc::from(Token::tokens_from(iter)),
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
