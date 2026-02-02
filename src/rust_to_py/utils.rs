use super::PY_MARKER;
use crate::utils::rust_token::{Punct, Token, TokenBuffer};
use proc_macro2::Delimiter;

pub(crate) trait PunctEx {
    fn is_py_marker(&self) -> bool;
}
impl PunctEx for Punct {
    fn is_py_marker(&self) -> bool {
        self.inner().as_char() == PY_MARKER
    }
}

pub(super) trait TokenEx {
    fn is_py_marker(&self) -> bool;
}
impl TokenEx for Token {
    fn is_py_marker(&self) -> bool {
        match self {
            Token::Punct(punct) => punct.is_py_marker(),
            _ => false,
        }
    }
}

pub(super) trait TokenOptionEx {
    fn is_whitespace(&self) -> bool;
    fn is_py_marker(&self) -> bool;
    fn eq_punct(&self, c: char) -> bool;
    fn eq_group(&self, delimiter: Delimiter) -> bool;
}
impl TokenOptionEx for Option<&Token> {
    fn is_whitespace(&self) -> bool {
        match self {
            Some(token) => token.is_whitespace(),
            None => true,
        }
    }

    fn is_py_marker(&self) -> bool {
        match self {
            Some(token) => token.is_py_marker(),
            None => false,
        }
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
}

pub(super) trait TokenBufferEx {
    fn is_current_py_marker_escaped(&self) -> bool;
    fn is_py_marker_escape(&self) -> bool;
    fn is_py_marker_start(&self) -> bool;
    fn is_py_marker_end(&self) -> bool;
    fn is_indent_block(&self) -> bool;
}
impl TokenBufferEx for TokenBuffer {
    fn is_current_py_marker_escaped(&self) -> bool {
        assert!(self.current().is_py_marker());
        self.peek(-1).eq_punct('<') && self.peek(1).eq_punct('>')
    }

    fn is_py_marker_escape(&self) -> bool {
        self.peek(1).is_py_marker() && self.seeked(1).unwrap().is_current_py_marker_escaped()
    }

    fn is_py_marker_start(&self) -> bool {
        self.current().is_py_marker() && !self.peek(1).is_whitespace() && !self.is_current_py_marker_escaped()
    }

    fn is_py_marker_end(&self) -> bool {
        self.current().is_py_marker() && !self.peek(-1).is_whitespace() && !self.is_current_py_marker_escaped()
    }

    fn is_indent_block(&self) -> bool {
        self.current().eq_punct(':') && self.peek(1).eq_group(Delimiter::Brace)
    }
}
