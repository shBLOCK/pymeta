use crate::utils::parse_buffer::ParseBuffer;
use crate::utils::span::{CSpan, SpanEx};
use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};
use quote::TokenStreamExt;
use std::cell::OnceCell;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;

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

#[allow(unused)]
impl Ident {
    pub fn eq_ident<'a>(&self, ident: impl Into<&'a str>) -> bool {
        self.ident.to_string() == ident.into()
    }
}

impl Punct {
    pub fn eq_punct(&self, ch: char) -> bool {
        self.punct.as_char() == ch
    }
}

impl Group {
    pub fn tokens(&self) -> TokenBuffer {
        TokenBuffer::from(Rc::clone(&self.tokens))
    }

    pub fn delimiter(&self) -> Delimiter {
        self.group.delimiter()
    }
}

impl From<proc_macro2::Group> for Group {
    fn from(value: proc_macro2::Group) -> Self {
        // TODO: check if this works properly when group delimiter is Delimiter::None
        let tokens = value.stream().into_iter().map(Token::from).collect();
        Self {
            group: value,
            span: OnceCell::new(),
            tokens,
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

        impl quote::ToTokens for $struct_name {
            fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
                tokens.append(self.$inner_name.clone());
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

#[allow(unused)]
impl Token {
    pub fn span(&self) -> Rc<CSpan> {
        match self {
            Self::Ident(ident) => ident.span(),
            Self::Punct(punct) => punct.span(),
            Self::Literal(literal) => literal.span(),
            Self::Group(group) => group.span(),
        }
    }

    pub fn eq_ident<'a>(&self, ident: impl Into<&'a str>) -> bool {
        match self {
            Self::Ident(it) => it.eq_ident(ident),
            _ => false,
        }
    }

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

macro_rules! token_struct_common {
    ($name:ident, $fn_name:ident) => {
        impl Token {
            #[allow(unused)]
            pub fn $fn_name(&self) -> Result<Rc<$name>, &Token> {
                match self {
                    Self::$name(it) => Ok(Rc::clone(it)),
                    token => Err(token),
                }
            }
        }
        
        #[allow(unused)]
        impl From<Rc<$name>> for Token {
            fn from(it: Rc<$name>) -> Self {
                Self::$name(it)
            }
        }
        
        #[allow(unused)]
        impl From<&$name> for TokenTree {
            fn from(it: &$name) -> Self {
                it.inner().clone().into()
            }
        }
    };
}

token_struct_common!(Ident, ident);
token_struct_common!(Punct, punct);
token_struct_common!(Literal, literal);
token_struct_common!(Group, group);

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

impl From<&Token> for TokenTree {
    fn from(value: &Token) -> Self {
        match value {
            Token::Ident(ident) => ident.inner().clone().into(),
            Token::Punct(punct) => punct.inner().clone().into(),
            Token::Literal(literal) => literal.inner().clone().into(),
            Token::Group(group) => group.inner().clone().into(),
        }
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

pub(crate) type TokenBuffer = ParseBuffer<Token>;

impl TokenBuffer {
    pub fn get_current_span_for_diagnostics(&self) -> Span {
        if let Some(token) = self.current() {
            return token.span().inner();
        }
        self.peek(-1)
            .map(|token| token.span().inner().end_span())
            .unwrap_or_else(Span::call_site)
    }

    pub fn into_last_token(mut self) -> Option<Token> {
        if self.slice(self.pos()..).len() != 1 {
            return None;
        }
        self.read_one().cloned()
    }
}

impl FromIterator<TokenTree> for TokenBuffer {
    fn from_iter<T: IntoIterator<Item = TokenTree>>(iter: T) -> Self {
        Self::from(iter.into_iter().map(Token::from).collect::<Rc<[Token]>>())
    }
}

impl quote::ToTokens for TokenBuffer {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(self.slice(..).iter().map(TokenTree::from))
    }
}

#[allow(unused)]
pub(crate) trait TokenOptionEx {
    fn ident(&self) -> Result<Rc<Ident>, &Self>;
    fn punct(&self) -> Result<Rc<Punct>, &Self>;
    fn literal(&self) -> Result<Rc<Literal>, &Self>;
    fn group(&self) -> Result<Rc<Group>, &Self>;

    fn eq_punct(&self, c: char) -> bool;
    fn eq_group(&self, delimiter: Delimiter) -> bool;

    fn expect_punct(&self, c: char) -> Result<Rc<Punct>, &Self>;
    fn expect_ident_by(&self, f: impl FnOnce(&str) -> bool) -> Result<Rc<Ident>, &Self>;
    fn expect_ident<'a>(&self, ident: impl Into<&'a str>) -> Result<Rc<Ident>, &Self>;
    fn expect_group_by(&self, f: impl FnOnce(Delimiter) -> bool) -> Result<Rc<Group>, &Self>;
    fn expect_group(&self, delimiter: Delimiter) -> Result<Rc<Group>, &Self>;

    fn maybe_unwrap_none_group(&self) -> Option<Token>;
}

impl TokenOptionEx for Option<&Token> {
    fn ident(&self) -> Result<Rc<Ident>, &Self> {
        self.and_then(|it| it.ident().ok()).ok_or(self)
    }

    fn punct(&self) -> Result<Rc<Punct>, &Self> {
        self.and_then(|it| it.punct().ok()).ok_or(self)
    }

    fn literal(&self) -> Result<Rc<Literal>, &Self> {
        self.and_then(|it| it.literal().ok()).ok_or(self)
    }

    fn group(&self) -> Result<Rc<Group>, &Self> {
        self.and_then(|it| it.group().ok()).ok_or(self)
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

    fn expect_punct(&self, c: char) -> Result<Rc<Punct>, &Self> {
        match self {
            Some(Token::Punct(punct)) if punct.eq_punct(c) => Ok(punct.clone()),
            _ => Err(self),
        }
    }
    
    fn expect_ident_by(&self, f: impl FnOnce(&str) -> bool) -> Result<Rc<Ident>, &Self> {
        match self {
            Some(Token::Ident(it)) if f(it.inner().to_string().as_str()) => Ok(it.clone()),
            _ => Err(self),
        }
    }

    fn expect_ident<'a>(&self, ident: impl Into<&'a str>) -> Result<Rc<Ident>, &Self> {
        self.expect_ident_by(|s| s == ident.into())
    }

    fn expect_group_by(&self, f: impl FnOnce(Delimiter) -> bool) -> Result<Rc<Group>, &Self> {
        match self {
            Some(Token::Group(it)) if f(it.delimiter()) => Ok(it.clone()),
            _ => Err(self),
        }
    }

    fn expect_group(&self, delimiter: Delimiter) -> Result<Rc<Group>, &Self> {
        self.expect_group_by(|it| it == delimiter)
    }

    fn maybe_unwrap_none_group(&self) -> Option<Token> {
        if let Some(Token::Group(group)) = self
            && group.delimiter() == Delimiter::None
        {
            Some(
                group
                    .tokens()
                    .into_last_token()
                    .expect("this Delimiter::None group does not contain a single token"),
            )
        } else {
            self.cloned()
        }
    }
}

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

pub(crate) trait DelimiterEx {
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
