use crate::rust_to_py::CONCAT_MARKER;
use crate::rust_to_py::code_regions::{
    CodeRegion, IdentWithPyExpr, PyExpr, PyStmt, PyStmtWithIndentBlock, RustCode, RustCodeWithBlock,
};
use crate::rust_to_py::utils::{DelimiterEx, PunctEx};
use crate::utils::SpanEx;
use crate::utils::escape::*;
use crate::utils::py_source::builder::PySourceBuilder;
use crate::utils::py_source::{PySegment, PySource};
use crate::utils::rust_token::{CSpan, Token, TokenBuffer};
use either::Either;
use proc_macro2::{Delimiter, Spacing};
use std::rc::Rc;

const INDENT_SIZE: usize = 4;

#[derive(Debug)]
pub(crate) struct PyMetaModule {
    pub filename: String,
    pub source: PySource,
    pub spans: Box<[Rc<CSpan>]>,
}

#[derive(Debug)]
pub(crate) struct PyMetaExecutable {
    pub main: Rc<PyMetaModule>,
    //TODO: imported modules
}

impl PyMetaExecutable {
    pub fn find_module_from_filename(&self, filename: &str) -> Option<&Rc<PyMetaModule>> {
        if self.main.filename == filename {
            Some(&self.main)
        } else {
            None
        }
    }
}

pub(crate) struct PyCodeGen {
    py: PySourceBuilder,
    spans: Vec<Rc<CSpan>>,
}
impl PyCodeGen {
    pub fn new() -> Self {
        Self {
            py: PySourceBuilder::new(),
            spans: Vec::new(),
        }
    }

    fn rust_literal_repr_to_python_code(repr: String) -> String {
        repr // FIXME: always produce valid Python syntax, or report error
    }

    /// Turn Rust tokens into Python code, assuming that they represent valid Python code.
    fn append_tokens_as_python_code(&mut self, mut tokens: TokenBuffer) {
        fn need_space_between(current_and_previous: &[Token]) -> bool {
            match current_and_previous {
                // between ident and literal
                [
                    ..,
                    Token::Ident(_) | Token::Literal(_),
                    Token::Ident(_) | Token::Literal(_),
                ] => true,

                // between two punct
                [.., Token::Punct(last_punct), Token::Punct(_)] => last_punct.inner().spacing() == Spacing::Alone,

                // before punct
                [.., _, Token::Punct(punct)] => {
                    punct.inner().spacing() == Spacing::Joint
                        || !matches!(punct.inner().as_char(), ';' | ',' | ':' | '.')
                }

                // decorators
                [Token::Punct(at), _] if at.eq_punct('@') => false,

                // single star
                [Token::Punct(star), _] if star.eq_punct('*') => false,
                [Token::Punct(comma), Token::Punct(star), _] if comma.eq_punct(',') && star.eq_punct('*') => false,
                // double star
                [Token::Punct(star1), Token::Punct(star2), _] if star1.eq_punct('*') && star2.eq_punct('*') => false,
                [Token::Punct(comma), Token::Punct(star1), Token::Punct(star2), _]
                    if comma.eq_punct(',') && star1.eq_punct('*') && star2.eq_punct('*') =>
                {
                    false
                }

                // after punct
                [.., Token::Punct(punct), _] => punct.inner().spacing() == Spacing::Alone && !punct.eq_punct('.'),

                // other cases of before ident (e.g. before `as` in `a = foo() as Bar`)
                [.., _, Token::Ident(_)] => true,

                _ => false,
            }
        }

        while let Some(token) = tokens.current() {
            if need_space_between(tokens.slice(..=tokens.pos())) {
                self.py.append(PySegment::new(" ", None));
            }

            // workaround `f"string"` being reserved syntax in Rust
            // `f~"string"` => `f"string"`
            if let [Token::Ident(prefix), Token::Punct(concat), Token::Literal(string), ..] =
                tokens.slice(tokens.pos()..)
                && concat.eq_punct(CONCAT_MARKER)
                && let repr = string.inner().to_string()
                && repr.starts_with('"')
            {
                self.py
                    .append(PySegment::new(prefix.inner().to_string(), Some(prefix.span())));
                self.py.append(PySegment::new(
                    Self::rust_literal_repr_to_python_code(repr),
                    Some(string.span()),
                ));
                tokens.seek(3).unwrap();
                continue;
            }

            match token {
                Token::Ident(ident) => {
                    self.py
                        .append(PySegment::new(ident.inner().to_string(), Some(ident.span())));
                }
                Token::Punct(punct) => {
                    if !punct.eq_punct(';') {
                        self.py.append(PySegment::new(punct.as_str(), Some(punct.span())));
                    }
                }
                Token::Literal(literal) => {
                    self.py.append(PySegment::new(
                        Self::rust_literal_repr_to_python_code(literal.inner().to_string()),
                        Some(literal.span()),
                    ));
                }
                Token::Group(group) => {
                    let delim = group.inner().delimiter();
                    //TODO: figure out what should we do with Delimiter::None
                    if delim != Delimiter::None {
                        self.py
                            .append(PySegment::new(delim.left_str().unwrap(), Some(group.span())));
                        self.append_tokens_as_python_code(group.tokens());
                        self.py
                            .append(PySegment::new(delim.right_str().unwrap(), Some(group.span())));
                    }
                }
            }
            tokens.seek(1).unwrap();
        }
    }

    fn append_py_logical_line(&mut self, line: &PyStmt) {
        self.py.new_line(None);
        self.append_tokens_as_python_code(TokenBuffer::from(&line.tokens));
    }

    fn append_py_stmt_with_indent_block(&mut self, region: &PyStmtWithIndentBlock) {
        self.append_py_logical_line(&region.stmt);
        self.py.push_indent_block(INDENT_SIZE);
        if !region.block.is_empty() {
            self.append_code_regions(region.block.iter());
        } else {
            self.py.new_line(None);
            self.py.append(PySegment::new("pass", Some(region.group.span())));
        }
        self.py.pop_indent_block();
    }

    fn append_span(&mut self, span: Rc<CSpan>) {
        self.spans.push(span.clone());
        let id = self.spans.len() - 1;
        self.py.append(PySegment::new(format!("__spans[{id}]"), Some(span)))
    }

    fn append_inline_py_expr(&mut self, expr: &PyExpr) {
        let (start_span, end_span) = (expr.start_marker.span(), expr.end_marker.span());
        if expr.tokens.is_empty() {
            self.py.append(PySegment::new(
                "None",
                Some(Rc::new(CSpan::from(
                    start_span.inner().join_or_fallback(Some(end_span.inner())),
                ))),
            ));
        } else {
            self.py.append(PySegment::new("(", Some(start_span)));
            self.append_tokens_as_python_code(TokenBuffer::from(&expr.tokens));
            self.py.append(PySegment::new(")", Some(end_span)));
        }
    }

    fn append_ident_with_inline_py_expr(&mut self, parts: &IdentWithPyExpr) {
        assert!(!parts.is_empty());

        let start_span = match parts.first().unwrap() {
            Either::Left(ident) => ident.span().inner(),
            Either::Right(expr) => expr.start_marker.span().inner(),
        };
        let end_span = match parts.last().unwrap() {
            Either::Left(ident) => ident.span().inner(),
            Either::Right(expr) => expr.end_marker.span().inner(),
        };
        let full_span = Rc::new(CSpan::from(start_span.join_or_fallback(Some(end_span))));

        self.py.append(PySegment::new("Ident(f\"", Some(Rc::clone(&full_span))));
        for part in parts {
            match part {
                Either::Left(ident) => {
                    self.py
                        .append(PySegment::new(ident.inner().to_string(), Some(ident.span())));
                }
                Either::Right(expr) => {
                    self.py.append(PySegment::new("{", Some(expr.start_marker.span())));
                    self.append_inline_py_expr(expr);
                    self.py.append(PySegment::new("}", Some(expr.end_marker.span())));
                }
            }
        }
        self.py.append(PySegment::new("\", ", None));
        self.append_span(Rc::clone(&full_span));
        self.py.append(PySegment::new(")", Some(full_span)));
    }

    fn append_rust_code_as_parameter_list_element(&mut self, code: &RustCode) {
        match code {
            RustCode::Code(token) => match token {
                Token::Ident(ident) => {
                    self.py.append(PySegment::new(
                        format!(r#"Ident("{}", "#, ident.inner().to_string()),
                        Some(ident.span()),
                    ));
                    self.append_span(ident.span());
                    self.py.append(PySegment::new(")", Some(ident.span())));
                    self.py.append(PySegment::new(", ", None));
                }
                Token::Punct(punct) => {
                    let char_str = match punct.as_str() {
                        "'" => "\\'",
                        c => c,
                    };
                    let spacing = match punct.inner().spacing() {
                        Spacing::Alone => "alone",
                        Spacing::Joint => "joint",
                    };
                    self.py.append(PySegment::new(
                        format!(r#"Punct('{char_str}', "{spacing}", "#),
                        Some(punct.span()),
                    ));
                    self.append_span(punct.span());
                    self.py.append(PySegment::new(")", Some(punct.span())));
                    self.py.append(PySegment::new(", ", None));
                }
                Token::Literal(literal) => {
                    let repr = literal.inner().to_string();
                    match repr.as_bytes() {
                        [b'"', ..] | [b'r', b'"', ..] => {
                            self.py.append(PySegment::new(r#"StrLiteral("#, Some(literal.span())));
                            self.py.append(PySegment::new(
                                rust_string_repr_to_python_str_repr(&repr),
                                Some(literal.span()),
                            ));
                            self.py.append(PySegment::new(r#", "str", "#, Some(literal.span())));
                        }
                        [b'\'', ..] => {
                            self.py.append(PySegment::new(
                                format!(r#"StrLiteral({}, "chr", "#, rust_char_repr_to_python_str_repr(&repr)),
                                Some(literal.span()),
                            ));
                        }
                        [b'b', b'"', ..] | [b'b', b'r', b'"', ..] => {
                            self.py.append(PySegment::new(r#"BytesLiteral("#, Some(literal.span())));
                            self.py.append(PySegment::new(
                                rust_bytes_repr_to_python_bytes_repr(&repr),
                                Some(literal.span()),
                            ));
                            self.py.append(PySegment::new(r#", "bytes", "#, Some(literal.span())));
                        }
                        [b'b', b'\'', ..] => {
                            self.py.append(PySegment::new(
                                format!(
                                    r#"BytesLiteral({}, "byte", "#,
                                    rust_byte_repr_to_python_bytes_repr(&repr)
                                ),
                                Some(literal.span()),
                            ));
                        }
                        [b'c', ..] => {
                            self.py.append(PySegment::new(r#"BytesLiteral("#, Some(literal.span())));
                            self.py.append(PySegment::new(
                                rust_c_string_repr_to_python_bytes_repr(&repr),
                                Some(literal.span()),
                            ));
                            self.py.append(PySegment::new(r#", "cstr", "#, Some(literal.span())));
                        }
                        repr @ [b'0'..=b'9', ..] => {
                            let is_float = match repr {
                                [b'0', b'x', ..] => false,
                                repr if repr.iter().any(|b| matches!(b, b'.' | b'e' | b'E' | b'f')) => true,
                                _ => false,
                            };

                            let suffix_i = if is_float {
                                repr.iter().rposition(|&b| b == b'f')
                            } else {
                                repr.iter().rposition(|&b| matches!(b, b'u' | b'i'))
                            };
                            let (num, type_obj) = match suffix_i {
                                Some(i) => unsafe {
                                    (
                                        str::from_utf8_unchecked(&repr[..i]),
                                        str::from_utf8_unchecked(&repr[i..]),
                                    )
                                },
                                None => unsafe { (str::from_utf8_unchecked(repr), "None") },
                            };

                            let cls_name = if is_float { "FloatLiteral" } else { "IntLiteral" };

                            self.py.append(PySegment::new(
                                format!(r#"{cls_name}._new("{num}", {num}, {type_obj}, "#),
                                Some(literal.span()),
                            ));
                        }
                        _ => panic!("Failed to parse literal: {repr:?}"),
                    };

                    self.append_span(literal.span());
                    self.py.append(PySegment::new(")", Some(literal.span())));
                    self.py.append(PySegment::new(", ", None));
                }
                Token::Group(_) => unreachable!(), // should be handled by RustCode::Group
            },
            RustCode::Group { group, code } => {
                self.py.append(PySegment::new(
                    format!(
                        r#"Group("{}", Tokens("#,
                        group.delimiter().left_right_str().unwrap_or("")
                    ),
                    Some(group.span()),
                ));
                code.iter()
                    .for_each(|code| self.append_rust_code_as_parameter_list_element(code));
                self.py.append(PySegment::new("), ", Some(group.span())));
                self.append_span(group.span());
                self.py.append(PySegment::new(")", Some(group.span())));
                self.py.append(PySegment::new(", ", None));
            }
            RustCode::PyExpr(expr) => {
                let span = Rc::new(CSpan::from(
                    expr.start_marker
                        .span()
                        .inner()
                        .join_or_fallback(Some(expr.end_marker.span().inner())),
                ));
                self.py.append(PySegment::new("Tokens(", Some(Rc::clone(&span))));
                self.append_inline_py_expr(expr);
                self.py.append(PySegment::new(", span=", Some(Rc::clone(&span))));
                self.append_span(Rc::clone(&span));
                self.py.append(PySegment::new(")", Some(span)));
                self.py.append(PySegment::new(", ", None));
            }
            RustCode::IdentWithPyExpr(parts) => {
                self.append_ident_with_inline_py_expr(parts);
                self.py.append(PySegment::new(", ", None));
            }
        }
    }

    fn append_rust_code_region(&mut self, region: &Vec<RustCode>) {
        let region = &region[..];
        if region.is_empty() {
            self.py.new_line(None);
            self.py.append(PySegment::new("pass", None));
            return;
        }

        self.py.new_line(None);

        self.py.append(PySegment::new("rust(", None));
        region
            .iter()
            .for_each(|code| self.append_rust_code_as_parameter_list_element(code));
        self.py.pop_last_segment_if(|seg| seg.code == ", ");
        self.py.append(PySegment::new(")", None));
    }

    fn append_rust_multiline_block(&mut self, region: &RustCodeWithBlock) {
        let region_code = region.code.as_ref();

        self.py.new_line(None);

        self.py.append(PySegment::new("with rust(", None));
        region_code
            .iter()
            .for_each(|code| self.append_rust_code_as_parameter_list_element(code));
        self.py.append(PySegment::new(
            format!(
                r#"Group("{}", span="#,
                region.group.delimiter().left_right_str().unwrap_or("")
            ),
            None,
        ));
        self.append_span(region.group.span());
        self.py.append(PySegment::new(")):", None));

        self.py.push_indent_block(INDENT_SIZE);
        self.append_code_regions(region.block.iter());
        self.py.pop_indent_block();
    }

    fn append_code_region(&mut self, region: &CodeRegion) {
        match region {
            CodeRegion::RustCode(region) => self.append_rust_code_region(region),
            CodeRegion::RustCodeWithBlock(region) => self.append_rust_multiline_block(region),
            CodeRegion::PyStmt(line) => self.append_py_logical_line(line),
            CodeRegion::PyStmtWithIndentBlock(region) => self.append_py_stmt_with_indent_block(region),
        }
    }

    pub fn append_code_regions<'a>(&mut self, regions: impl Iterator<Item = &'a CodeRegion>) {
        regions.for_each(|region| self.append_code_region(region));
    }

    pub fn finish(self, filename: String) -> PyMetaModule {
        PyMetaModule {
            filename,
            source: self.py.finish(),
            spans: self.spans.into_boxed_slice(),
        }
    }
}
