use crate::abort;
use crate::utils::diagnostic::MultiSpan;
use crate::utils::rust_token::{Group, Ident, Punct, Token, TokenBuffer, TokenOptionEx};
use proc_macro2::{Delimiter, Spacing, Span, TokenStream};
use quote::ToTokens;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

#[derive(Debug)]
pub struct ParseError<E: Display + Debug> {
    spans: Vec<Span>,
    error: E,
}
impl<E: Display + Debug> ParseError<E> {
    pub fn new(spans: impl MultiSpan, error: E) -> Self {
        Self { spans: spans.into_spans(), error }
    }

    pub fn abort(self) -> ! {
        abort!(&self.spans, "{}", self.error)
    }
}

pub type ParseResult<T, E> = Result<T, ParseError<E>>;

#[derive(Clone, Debug)]
pub struct DoubleColon(pub Rc<Punct>, pub Rc<Punct>);
impl DoubleColon {
    pub fn try_parse(tokens: &mut TokenBuffer) -> ParseResult<Self, &'static str> {
        tokens.try_run_or_rewind(|tokens| {
            let a = tokens
                .read_one()
                .expect_punct(':')
                .map_err(|t| ParseError::new(t, "expected `:`"))?;
            let b = tokens
                .read_one()
                .expect_punct(':')
                .map_err(|t| ParseError::new(t, "expected `:`"))?;
            if a.inner().spacing() != Spacing::Joint {
                return Err(ParseError::new(a.span(), "wrong punct spacing"));
            }
            if b.inner().spacing() != Spacing::Alone {
                return Err(ParseError::new(b.span(), "wrong punct spacing"));
            }
            Ok(Self(a, b))
        })
    }
}
impl ToTokens for DoubleColon {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.0.to_tokens(tokens);
        self.1.to_tokens(tokens);
    }
}

#[derive(Debug)]
pub(crate) struct RustSimplePath {
    pub root: Option<DoubleColon>,
    pub segments: Box<[Rc<Ident>]>,
    pub separators: Box<[DoubleColon]>,
}
impl RustSimplePath {
    pub fn try_parse(tokens: &mut TokenBuffer) -> ParseResult<Self, &'static str> {
        tokens.try_run_or_rewind(|tokens| {
            let mut root = None;
            let mut segments = Vec::new();
            let mut separators = Vec::new();
            for i in 0.. {
                let sep = DoubleColon::try_parse(tokens);

                if i == 0 {
                    root = sep.as_ref().ok().cloned();
                }
                if sep.is_ok() || i == 0 {
                    if let Some(Token::Ident(seg)) = tokens.read_one() {
                        segments.push(seg.clone());
                        if i != 0 {
                            separators.push(sep.unwrap());
                        }
                    } else {
                        return Err(ParseError::new(
                            tokens.get_current_span_for_diagnostics(),
                            "invalid Rust path: expected identifier",
                        ));
                    }
                } else {
                    break;
                }
            }
            if segments.is_empty() {
                return Err(ParseError::new(
                    tokens.get_current_span_for_diagnostics(),
                    "invalid Rust path: empty",
                ));
            }
            Ok(Self {
                root,
                segments: segments.into(),
                separators: separators.into(),
            })
        })
    }

    pub fn is_root(&self) -> bool {
        self.root.is_some()
    }

    pub fn total_span(&self) -> Span {
        let start = self
            .root
            .as_ref()
            .map(|it| it.0.span())
            .unwrap_or_else(|| self.segments[0].span())
            .inner();
        let end = self.segments.last().unwrap().span().inner();
        start.join(end).unwrap_or(start)
    }

    // pub fn map_last_segment(&self, f: impl FnOnce(&Rc<Ident>) -> Ident) -> Self {
    //     let mut segments = self.segments.to_vec();
    //     let last = Rc::new(f(segments.last().unwrap()));
    //     *segments.last_mut().unwrap() = last.clone();
    //     let mut tokens = self.tokens.to_vec();
    //     *tokens.last_mut().unwrap() = last.into();
    //     Self {
    //         segments: segments.into(),
    //         is_root: self.is_root,
    //         tokens: tokens.into(),
    //     }
    // }
}

impl PartialEq for RustSimplePath {
    #[allow(clippy::cmp_owned)]
    fn eq(&self, other: &Self) -> bool {
        self.is_root() == other.is_root()
            && self.segments.len() == other.segments.len()
            && self
                .segments
                .iter()
                .zip(other.segments.iter())
                .all(|(a, b)| a.inner().to_string() == b.inner().to_string())
    }
}

impl Eq for RustSimplePath {}

impl Hash for RustSimplePath {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.is_root().hash(state);
        self.segments.iter().for_each(|s| s.inner().to_string().hash(state));
    }
}

impl ToTokens for RustSimplePath {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for (i, segment) in self.segments.iter().enumerate() {
            if i == 0 {
                self.root.to_tokens(tokens);
            } else {
                self.separators[i - 1].to_tokens(tokens);
            }
            segment.to_tokens(tokens);
        }
    }
}

impl Display for RustSimplePath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (i, segment) in self.segments.iter().enumerate() {
            if i != 0 || self.is_root() {
                f.write_str("::")?;
            }
            f.write_str(segment.inner().to_string().as_str())?;
        }
        Ok(())
    }
}

pub struct RustAttribute {
    pub hash: Rc<Punct>,
    pub group: Rc<Group>,
    pub path: RustSimplePath,
}
impl RustAttribute {
    pub fn try_parse(tokens: &mut TokenBuffer) -> ParseResult<Self, &'static str> {
        tokens.try_run_or_rewind(|tokens| {
            let hash = tokens
                .read_one()
                .expect_punct('#')
                .map_err(|t| ParseError::new(t, "expected `#`"))?;
            let group = tokens
                .read_one()
                .expect_group(Delimiter::Bracket)
                .map_err(|t| ParseError::new(t, "expected `[...]`"))?;
            let path = RustSimplePath::try_parse(&mut group.tokens())?;
            Ok(Self { hash, group, path })
        })
    }
}
impl ToTokens for RustAttribute {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.hash.to_tokens(tokens);
        self.group.to_tokens(tokens);
    }
}

pub struct RustVis {
    pub pub_ident: Rc<Ident>,
    pub params_group: Option<Rc<Group>>,
}
impl RustVis {
    pub fn try_parse(kw_alias: &str, tokens: &mut TokenBuffer) -> ParseResult<Self, &'static str> {
        tokens.try_run_or_rewind(|tokens| {
            let pub_ident = tokens
                .read_one()
                .expect_ident(kw_alias)
                .map_err(|t| ParseError::new(t, "expected vis keyword"))?;
            let pub_ident = if kw_alias != "pub" {
                Rc::new(proc_macro2::Ident::new("pub", pub_ident.span().inner()).into())
            } else {
                pub_ident
            };
            let params_group = tokens.read_one().expect_group(Delimiter::Parenthesis).ok();
            Ok(Self { pub_ident, params_group })
        })
    }

    pub fn is_pub(&self) -> bool {
        self.params_group.is_none()
    }
}
impl ToTokens for RustVis {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.pub_ident.to_tokens(tokens);
        self.params_group.to_tokens(tokens);
    }
}
