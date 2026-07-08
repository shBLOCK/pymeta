use super::py_source::builder::PySourceBuilder;
use super::py_source::{PySource, PySrcSegment};
use crate::rust_to_py::CONCAT_MARKER;
use crate::rust_to_py::code_region::{
    CodeRegion, IdentWithPyExpr, PyExpr, PySegment, PyStmt, PyStmtWithIndentBlock, RustCode, RustCodeWithBlock,
};
use crate::utils::escape::*;
use crate::utils::parse_buffer::ParseBuffer;
use crate::utils::rust_token::{DelimiterEx, PunctEx, Token};
use crate::utils::span::{CSpan, SpanEx};
use either::Either;
use proc_macro2::{Delimiter, Spacing, Span};
use std::rc::Rc;
use proc_macro_error3::{Diagnostic, Level as DiagnosticLevel};

const INDENT_SIZE: usize = 4;

/// Represents a Python module that's generated from the proc-macro input.
///
/// Currently, there's only the main module, but in the future imported modules will also be represented by this.
///
/// Aside from source code and metadata,
/// this struct also carries additional information that need to be passed into the Python module
/// when invoking the module.
#[derive(Debug)]
pub(crate) struct PyMetaModule {
    pub filename: String,
    pub source: PySource,
    pub spans: Box<[Rc<CSpan>]>,
}

impl PyMetaModule {
    pub fn emit_source_dump(&self) {
        Diagnostic::spanned(
            Span::call_site(), // TODO: refer to actual span for non-main module
            DiagnosticLevel::Warning,
            format!(
                "PyMeta source dump of \"{filename}\":\n{dump}",
                filename = self.filename,
                dump = self.source.diagnostic_source_dump()
            ),
        )
        .emit();
    }
}

/// Represents the final "executable" ready to be executed by a Python implementation.
///
/// Currently, this only contains the `main` module, but in the future imported modules will also be here.
#[derive(Debug)]
pub(crate) struct PyMetaExecutable {
    pub main: Rc<PyMetaModule>,
    // pub modules:
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

/// Builder struct for generating a [PyMetaModule].
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
    fn append_py_segments_as_python_code(&mut self, mut segments: ParseBuffer<PySegment>) {
        use PySegment::Token as PT;

        fn need_space_between(current_and_previous: &[PySegment]) -> bool {
            match current_and_previous {
                // between ident and literal
                [
                    ..,
                    PT(Token::Ident(_)) | PT(Token::Literal(_)),
                    PT(Token::Ident(_)) | PT(Token::Literal(_)),
                ] => true,

                // between two punct
                [.., PT(Token::Punct(last_punct)), PT(Token::Punct(_))] => {
                    last_punct.inner().spacing() == Spacing::Alone
                }

                // before punct
                [.., _, PT(Token::Punct(punct))] => {
                    punct.inner().spacing() == Spacing::Joint
                        || !matches!(punct.inner().as_char(), ';' | ',' | ':' | '.')
                }

                // decorators
                [PT(Token::Punct(at)), _] if at.eq_punct('@') => false,

                // single star
                [PT(Token::Punct(star)), _] if star.eq_punct('*') => false,
                [PT(Token::Punct(comma)), PT(Token::Punct(star)), _] if comma.eq_punct(',') && star.eq_punct('*') => {
                    false
                }
                // double star
                [PT(Token::Punct(star1)), PT(Token::Punct(star2)), _] if star1.eq_punct('*') && star2.eq_punct('*') => {
                    false
                }
                [
                    PT(Token::Punct(comma)),
                    PT(Token::Punct(star1)),
                    PT(Token::Punct(star2)),
                    _,
                ] if comma.eq_punct(',') && star1.eq_punct('*') && star2.eq_punct('*') => false,

                // after punct
                [.., PT(Token::Punct(punct)), _] => punct.inner().spacing() == Spacing::Alone && !punct.eq_punct('.'),

                // other cases of before ident (e.g. before `as` in `a = foo() as Bar`)
                [.., _, PT(Token::Ident(_))] => true,

                _ => false,
            }
        }

        while let Some(segment) = segments.current() {
            if need_space_between(segments.slice(..=segments.pos())) {
                self.py.append(PySrcSegment::new(" ", None));
            }

            // workaround `f"string"` being reserved syntax in Rust
            // `f~"string"` => `f"string"`
            if let [
                PT(Token::Ident(prefix)),
                PT(Token::Punct(concat)),
                PT(Token::Literal(string)),
                ..,
            ] = segments.slice(segments.pos()..)
                && concat.eq_punct(CONCAT_MARKER)
                && let repr = string.inner().to_string()
                && repr.starts_with('"')
            {
                self.py
                    .append(PySrcSegment::new(prefix.inner().to_string(), Some(prefix.span())));
                self.py.append(PySrcSegment::new(
                    Self::rust_literal_repr_to_python_code(repr),
                    Some(string.span()),
                ));
                segments.seek(3).unwrap();
                continue;
            }

            match segment {
                PT(Token::Ident(ident)) => {
                    self.py
                        .append(PySrcSegment::new(ident.inner().to_string(), Some(ident.span())));
                }
                PT(Token::Punct(punct)) => {
                    if !punct.eq_punct(';') {
                        self.py.append(PySrcSegment::new(punct.as_str(), Some(punct.span())));
                    }
                }
                PT(Token::Literal(literal)) => {
                    self.py.append(PySrcSegment::new(
                        Self::rust_literal_repr_to_python_code(literal.inner().to_string()),
                        Some(literal.span()),
                    ));
                }
                PT(Token::Group(_)) => unreachable!("Group tokens should have been converted into `PySegment::Group`s"),
                PySegment::Group { group, segments: group_segments } => {
                    let delim = match group.inner().delimiter() {
                        Delimiter::None => Delimiter::Parenthesis,
                        it => it,
                    };
                    self.py
                        .append(PySrcSegment::new(delim.left_str().unwrap(), Some(group.span())));
                    self.append_py_segments_as_python_code(Rc::clone(group_segments).into());
                    self.py
                        .append(PySrcSegment::new(delim.right_str().unwrap(), Some(group.span())));
                }
                PySegment::MetaExpr(meta_expr) => {
                    todo!()
                }
            }
            segments.seek(1).unwrap();
        }
    }

    fn append_py_logical_line(&mut self, line: &PyStmt) {
        self.py.new_line(None);
        self.append_py_segments_as_python_code(Rc::clone(&line.segments).into());
    }

    fn append_py_stmt_with_indent_block(&mut self, region: &PyStmtWithIndentBlock) {
        self.append_py_logical_line(&region.stmt);
        self.py.push_indent_block(INDENT_SIZE);
        if !region.block.is_empty() {
            self.append_code_regions(region.block.iter());
        } else {
            self.py.new_line(None);
            self.py.append(PySrcSegment::new("pass", Some(region.group.span())));
        }
        self.py.pop_indent_block();
    }

    fn append_span(&mut self, span: Rc<CSpan>) {
        self.spans.push(span.clone());
        let id = self.spans.len() - 1;
        self.py.append(PySrcSegment::new(format!("__spans[{id}]"), Some(span)))
    }

    fn append_inline_py_expr(&mut self, expr: &PyExpr) {
        let (start_span, end_span) = (expr.start_marker.span(), expr.end_marker.span());
        if expr.segments.is_empty() {
            self.py.append(PySrcSegment::new(
                "None",
                Some(Rc::new(CSpan::from(
                    start_span.inner().join_or_fallback(Some(end_span.inner())),
                ))),
            ));
        } else {
            self.py.append(PySrcSegment::new("(", Some(start_span)));
            self.append_py_segments_as_python_code(ParseBuffer::from(Rc::clone(&expr.segments)));
            self.py.append(PySrcSegment::new(")", Some(end_span)));
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

        self.py
            .append(PySrcSegment::new("Ident(f\"", Some(Rc::clone(&full_span))));
        for part in parts {
            match part {
                Either::Left(ident) => {
                    self.py
                        .append(PySrcSegment::new(ident.inner().to_string(), Some(ident.span())));
                }
                Either::Right(expr) => {
                    self.py.append(PySrcSegment::new("{", Some(expr.start_marker.span())));
                    self.append_inline_py_expr(expr);
                    self.py.append(PySrcSegment::new("}", Some(expr.end_marker.span())));
                }
            }
        }
        self.py.append(PySrcSegment::new("\", ", None));
        self.append_span(Rc::clone(&full_span));
        self.py.append(PySrcSegment::new(")", Some(full_span)));
    }

    fn append_rust_code_as_parameter_list_element(&mut self, code: &RustCode) {
        match code {
            RustCode::Code(token) => match token {
                Token::Ident(ident) => {
                    self.py.append(PySrcSegment::new(
                        format!(r#"Ident("{}", "#, ident.inner()),
                        Some(ident.span()),
                    ));
                    self.append_span(ident.span());
                    self.py.append(PySrcSegment::new(")", Some(ident.span())));
                    self.py.append(PySrcSegment::new(", ", None));
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
                    self.py.append(PySrcSegment::new(
                        format!(r#"Punct('{char_str}', "{spacing}", "#),
                        Some(punct.span()),
                    ));
                    self.append_span(punct.span());
                    self.py.append(PySrcSegment::new(")", Some(punct.span())));
                    self.py.append(PySrcSegment::new(", ", None));
                }
                Token::Literal(literal) => {
                    let repr = literal.inner().to_string();
                    match repr.as_bytes() {
                        [b'"', ..] | [b'r', b'"', ..] => {
                            self.py
                                .append(PySrcSegment::new(r#"StrLiteral("#, Some(literal.span())));
                            self.py.append(PySrcSegment::new(
                                rust_string_repr_to_python_str_repr(&repr),
                                Some(literal.span()),
                            ));
                            self.py.append(PySrcSegment::new(r#", "str", "#, Some(literal.span())));
                        }
                        [b'\'', ..] => {
                            self.py.append(PySrcSegment::new(
                                format!(r#"StrLiteral({}, "chr", "#, rust_char_repr_to_python_str_repr(&repr)),
                                Some(literal.span()),
                            ));
                        }
                        [b'b', b'"', ..] | [b'b', b'r', b'"', ..] => {
                            self.py
                                .append(PySrcSegment::new(r#"BytesLiteral("#, Some(literal.span())));
                            self.py.append(PySrcSegment::new(
                                rust_bytes_repr_to_python_bytes_repr(&repr),
                                Some(literal.span()),
                            ));
                            self.py
                                .append(PySrcSegment::new(r#", "bytes", "#, Some(literal.span())));
                        }
                        [b'b', b'\'', ..] => {
                            self.py.append(PySrcSegment::new(
                                format!(
                                    r#"BytesLiteral({}, "byte", "#,
                                    rust_byte_repr_to_python_bytes_repr(&repr)
                                ),
                                Some(literal.span()),
                            ));
                        }
                        [b'c', ..] => {
                            self.py
                                .append(PySrcSegment::new(r#"BytesLiteral("#, Some(literal.span())));
                            self.py.append(PySrcSegment::new(
                                rust_c_string_repr_to_python_bytes_repr(&repr),
                                Some(literal.span()),
                            ));
                            self.py.append(PySrcSegment::new(r#", "cstr", "#, Some(literal.span())));
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

                            self.py.append(PySrcSegment::new(
                                format!(r#"{cls_name}._new("{num}", {num}, {type_obj}, "#),
                                Some(literal.span()),
                            ));
                        }
                        _ => panic!("Failed to parse literal: {repr:?}"),
                    };

                    self.append_span(literal.span());
                    self.py.append(PySrcSegment::new(")", Some(literal.span())));
                    self.py.append(PySrcSegment::new(", ", None));
                }
                Token::Group(_) => unreachable!(), // should be handled by RustCode::Group
            },
            RustCode::Group { group, code } => {
                self.py.append(PySrcSegment::new(
                    format!(
                        r#"Group("{}", Tokens("#,
                        group.delimiter().left_right_str().unwrap_or("")
                    ),
                    Some(group.span()),
                ));
                code.iter()
                    .for_each(|code| self.append_rust_code_as_parameter_list_element(code));
                self.py.append(PySrcSegment::new("), ", Some(group.span())));
                self.append_span(group.span());
                self.py.append(PySrcSegment::new(")", Some(group.span())));
                self.py.append(PySrcSegment::new(", ", None));
            }
            RustCode::PyExpr(expr) => {
                let span = Rc::new(CSpan::from(
                    expr.start_marker
                        .span()
                        .inner()
                        .join_or_fallback(Some(expr.end_marker.span().inner())),
                ));
                self.py.append(PySrcSegment::new("Tokens(", Some(Rc::clone(&span))));
                self.append_inline_py_expr(expr);
                self.py.append(PySrcSegment::new(", span=", Some(Rc::clone(&span))));
                self.append_span(Rc::clone(&span));
                self.py.append(PySrcSegment::new(")", Some(span)));
                self.py.append(PySrcSegment::new(", ", None));
            }
            RustCode::IdentWithPyExpr(parts) => {
                self.append_ident_with_inline_py_expr(parts);
                self.py.append(PySrcSegment::new(", ", None));
            }
        }
    }

    fn append_rust_code_region(&mut self, region: &[RustCode]) {
        if region.is_empty() {
            self.py.new_line(None);
            self.py.append(PySrcSegment::new("pass", None));
            return;
        }

        self.py.new_line(None);

        self.py.append(PySrcSegment::new("rust(", None));
        region
            .iter()
            .for_each(|code| self.append_rust_code_as_parameter_list_element(code));
        self.py.pop_last_segment_if(|seg| seg.code == ", ");
        self.py.append(PySrcSegment::new(")", None));
    }

    fn append_rust_multiline_block(&mut self, region: &RustCodeWithBlock) {
        let region_code = region.code.as_ref();

        self.py.new_line(None);

        self.py.append(PySrcSegment::new("with rust(", None));
        region_code
            .iter()
            .for_each(|code| self.append_rust_code_as_parameter_list_element(code));
        self.py.append(PySrcSegment::new(
            format!(
                r#"Group("{}", span="#,
                region.group.delimiter().left_right_str().unwrap_or("")
            ),
            None,
        ));
        self.append_span(region.group.span());
        self.py.append(PySrcSegment::new(")):", None));

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
            CodeRegion::MetaStmt(meta_stmt) => meta_stmt.body.gen_py_code(self),
            CodeRegion::PurePyBlock(block) => self.append_code_regions(block.content.iter()),
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

    pub fn gen_from_code_regions<'a>(filename: String, regions: impl Iterator<Item = &'a CodeRegion>) -> PyMetaModule {
        let mut generator = Self::new();
        generator.append_code_regions(regions);
        generator.finish(filename)
    }
}
