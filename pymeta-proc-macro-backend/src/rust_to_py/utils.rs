use super::{PY_MARKER, PY_MARKER_IDENT};
use crate::utils::rust_token::{Punct, Token, TokenBuffer, TokenOptionEx};
use proc_macro2::{Spacing, TokenStream, TokenTree};
use quote::TokenStreamExt;
use std::rc::Rc;

pub(super) trait TokenBufferEx {
    fn is_py_marker_escape(&self) -> bool;
    fn skip_py_marker_escape(&mut self);
    fn read_unescaped_py_marker_escape(&mut self) -> Rc<Punct>;
    fn is_py_marker(&self) -> bool;
    fn read_py_marker(&mut self) -> Rc<Punct>;
}

fn _is_py_marker(token: Option<&Token>) -> bool {
    match token {
        Some(Token::Punct(punct)) if punct.eq_punct(PY_MARKER) => true,
        Some(Token::Ident(ident)) if ident.eq_ident(PY_MARKER_IDENT) => true,
        _ => false,
    }
}

fn as_py_marker(token: Option<&Token>) -> Rc<Punct> {
    match token {
        Some(Token::Punct(punct)) if punct.eq_punct(PY_MARKER) => Rc::clone(punct),
        Some(Token::Ident(ident)) if ident.eq_ident(PY_MARKER_IDENT) => {
            let mut punct = proc_macro2::Punct::new(PY_MARKER, Spacing::Alone);
            punct.set_span(ident.span().inner());
            Rc::new(punct.into())
        }
        _ => panic!("not a py marker"),
    }
}

fn is_current_py_marker_escaped(tokens: &TokenBuffer) -> bool {
    assert!(_is_py_marker(tokens.current()));
    tokens.peek(-1).eq_punct('<') && tokens.peek(1).eq_punct('>')
}

impl TokenBufferEx for TokenBuffer {
    fn is_py_marker_escape(&self) -> bool {
        _is_py_marker(self.peek(1)) && is_current_py_marker_escaped(&self.seeked(1).unwrap())
    }

    fn skip_py_marker_escape(&mut self) {
        self.seek(3).unwrap();
    }

    fn read_unescaped_py_marker_escape(&mut self) -> Rc<Punct> {
        let punct = as_py_marker(self.peek(1));
        self.skip_py_marker_escape();
        punct
    }

    fn is_py_marker(&self) -> bool {
        _is_py_marker(self.current()) && !is_current_py_marker_escaped(self)
    }

    fn read_py_marker(&mut self) -> Rc<Punct> {
        as_py_marker(self.read_one())
    }
}

pub(crate) fn py_markers_to_py_marker_idents(tokens: TokenStream) -> TokenStream {
    let mut new_tokens = TokenStream::new();
    for token in tokens {
        match token {
            TokenTree::Punct(punct) if punct.as_char() == PY_MARKER => {
                new_tokens.append(proc_macro2::Ident::new(PY_MARKER_IDENT, punct.span()));
            }
            TokenTree::Group(group) => {
                let mut new_group =
                    proc_macro2::Group::new(group.delimiter(), py_markers_to_py_marker_idents(group.stream()));
                new_group.set_span(group.span());
                new_tokens.append(new_group);
            }
            token => new_tokens.append(token),
        }
    }
    new_tokens
}
