use crate::utils::rust_token::{Ident, Token, TokenBuffer};
use proc_macro2::{Spacing, Span, TokenStream, TokenTree};
use quote::TokenStreamExt;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

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
    #[allow(clippy::cmp_owned)]
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

impl Hash for SimpleRustPath {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.is_root.hash(state);
        self.segments.iter().for_each(|s| s.inner().to_string().hash(state));
    }
}

impl quote::ToTokens for SimpleRustPath {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append_all(self.tokens.iter().map(TokenTree::from));
    }
}

impl Display for SimpleRustPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for segment in &self.segments {
            if !first || self.is_root {
                f.write_str("::")?;
            }
            f.write_str(segment.inner().to_string().as_str())?;
            first = false;
        }
        Ok(())
    }
}
