use super::PY_MARKER;
use crate::utils::rust_token::{Punct, Token, TokenBuffer};
use crate::utils::span::CSpan;
use proc_macro2::Delimiter;
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
    fn eq_punct(&self, c: char) -> bool;
    fn eq_group(&self, delimiter: Delimiter) -> bool;
}
impl TokenOptionEx for Option<&Token> {
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
