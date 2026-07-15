use super::py_source::builder::PySourceBuilder;
use super::py_source::{PySource, PySrcSegment};
use crate::rust_to_py::code_region::{
    CodeRegion, IdentWithPyExpr, PyExpr, PySegment, PyStmt, PyStmtWithIndentBlock, RustCode, RustCodeWithBlock,
};
use crate::rust_to_py::{CONCAT_MARKER, PY_GLOBAL_OBJS_ARRAY_NAME};
use crate::utils::diagnostic::{Diagnostic, DiagnosticLevel};
use crate::utils::escape::*;
use crate::utils::parse_buffer::ParseBuffer;
use crate::utils::rust_token::{DelimiterEx, PunctEx, Token};
use crate::utils::span::{CSpan, SpanEx};
use either::Either;
use proc_macro2::{Delimiter, Spacing, Span};
use std::rc::Rc;

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
    pub _package: Option<String>,
    pub name: String,
    pub filename: String,
    pub source: PySource,
}

impl PyMetaModule {
    //TODO: allow Python-side reflection into the PyMeta executable, and do user-controlled source dump from there
    #[allow(unused)]
    pub fn emit_source_dump(&self) {
        Diagnostic::new(
            Span::call_site(), // TODO: refer to actual span for non-main module
            DiagnosticLevel::Note,
            format!(
                "PyMeta source dump of \"{filename}\":\n{dump}",
                filename = self.filename,
                dump = self.source.diagnostic_source_dump()
            ),
        )
        .emit();
    }
}

#[derive(Debug)]
pub(crate) enum PyObj {
    Span(Rc<CSpan>),
}

/// Represents the final "executable" ready to be executed by a Python implementation.
///
/// Currently, this only contains the `main` module, but in the future imported modules will also be here.
#[derive(Debug)]
pub(crate) struct PyMetaExecutable {
    pub main: Rc<PyMetaModule>,
    pub modules: Box<[Rc<PyMetaModule>]>,
    pub objs: Box<[PyObj]>,
}

impl PyMetaExecutable {
    pub fn new(main: Rc<PyMetaModule>, modules: Box<[Rc<PyMetaModule>]>, codegen_ctx: PyCodeGenContext) -> Self {
        Self {
            main,
            modules,
            objs: codegen_ctx.objs.into(),
        }
    }

    pub fn find_module_from_filename(&self, filename: &str) -> Option<&Rc<PyMetaModule>> {
        if self.main.filename == filename {
            Some(&self.main)
        } else {
            self.modules.iter().find(|it| it.filename == filename)
        }
    }
}

pub(crate) struct PyCodeGenContext {
    objs: Vec<PyObj>,
    uid_counter: usize,
}
impl PyCodeGenContext {
    pub fn new() -> Self {
        Self { objs: Vec::new(), uid_counter: 0 }
    }

    fn add_obj(&mut self, obj: PyObj) -> usize {
        self.objs.push(obj);
        self.objs.len() - 1
    }

    fn next_uid(&mut self) -> usize {
        let value = self.uid_counter;
        self.uid_counter += 1;
        value
    }
}

/// Builder struct for generating a [PyMetaModule].
pub(crate) struct PyCodeGen<'a> {
    pub py: PySourceBuilder,
    ctx: &'a mut PyCodeGenContext,
}
impl PyCodeGen<'_> {
    pub fn new(ctx: &mut PyCodeGenContext) -> PyCodeGen<'_> {
        PyCodeGen { py: PySourceBuilder::new(), ctx }
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

                // neighboring meta stuff
                [.., _, PySegment::MetaExpr(..) | PySegment::Quote { .. }]
                | [.., PySegment::MetaExpr(..) | PySegment::Quote { .. }, _] => true,

                _ => false,
            }
        }

        while let Some(segment) = segments.current() {
            if need_space_between(segments.slice(..=segments.pos())) {
                self.py.append(" ");
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
                self.py.append(prefix);
                self.py.append(PySrcSegment::new(
                    Self::rust_literal_repr_to_python_code(repr),
                    string.span(),
                ));
                segments.seek(3).unwrap();
                continue;
            }

            match segment {
                PT(Token::Ident(ident)) => {
                    self.py.append(ident);
                }
                PT(Token::Punct(punct)) => {
                    // if !punct.eq_punct(';') {
                    self.py.append(punct);
                    // }
                }
                PT(Token::Literal(literal)) => {
                    self.py.append(PySrcSegment::new(
                        Self::rust_literal_repr_to_python_code(literal.inner().to_string()),
                        literal.span(),
                    ));
                }
                PT(Token::Group(_)) => unreachable!("Group tokens should have been converted into `PySegment::Group`s"),
                PySegment::Group { group, segments: group_segments } => {
                    let delim = match group.inner().delimiter() {
                        Delimiter::None => Delimiter::Parenthesis,
                        it => it,
                    };
                    self.py.append(PySrcSegment::new(
                        delim.left_str().unwrap(),
                        Rc::new(group.span().start_span()),
                    ));
                    self.append_py_segments_as_python_code(Rc::clone(group_segments).into());
                    self.py.append(PySrcSegment::new(
                        delim.right_str().unwrap(),
                        Rc::new(group.span().end_span()),
                    ));
                }
                PySegment::MetaExpr(meta_expr) => {
                    let _ = meta_expr;
                    unreachable!("not yet implemented");
                }
                PySegment::Quote { outer_group, content, .. } => {
                    let span = outer_group.span();
                    let func_name = format_args!("__pymeta_quote_{}__", self.ctx.next_uid());

                    let mut content_codegen = Self::new(self.ctx);
                    content_codegen.py.new_line(None);
                    content_codegen
                        .py
                        .append(PySrcSegment::new(format!("def {func_name}():"), span.clone()));
                    content_codegen.py.push_indent_block(INDENT_SIZE);
                    content_codegen.py.new_line(None);
                    content_codegen
                        .py
                        .append(("with Tokens() as __pymeta_quote_result__:", span.clone()));
                    content_codegen.py.push_indent_block(INDENT_SIZE);
                    content_codegen.append_code_regions(content.iter());
                    content_codegen.py.pop_indent_block();
                    content_codegen.py.new_line(None);
                    content_codegen
                        .py
                        .append(("return __pymeta_quote_result__;", span.clone()));
                    content_codegen.py.pop_indent_block();

                    self.py.insert_before_current_line(content_codegen.py);
                    self.py.append(PySrcSegment::new(format!("{func_name}()"), span));
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
            self.py.append(("pass", region.group.span()));
        }
        self.py.pop_indent_block();
    }

    fn append_obj(&mut self, obj: PyObj, span: impl Into<Option<Rc<CSpan>>>) {
        let id = self.ctx.add_obj(obj);
        self.py.append(PySrcSegment::new(
            format!("{PY_GLOBAL_OBJS_ARRAY_NAME}[{id}]"),
            span.into(),
        ));
    }

    fn append_span_obj(&mut self, span: Rc<CSpan>) {
        self.append_obj(PyObj::Span(Rc::clone(&span)), span);
    }

    fn append_inline_py_expr(&mut self, expr: &PyExpr) {
        let (start_span, end_span) = (expr.start_marker.span(), expr.end_marker.span());
        if expr.segments.is_empty() {
            self.py.append((
                "None",
                Rc::new(CSpan::from(start_span.inner().join_or_fallback(Some(end_span.inner())))),
            ));
        } else {
            self.py.append(("(", start_span));
            self.append_py_segments_as_python_code(ParseBuffer::from(Rc::clone(&expr.segments)));
            self.py.append((")", end_span));
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

        self.py.append(("Ident(f\"", Rc::clone(&full_span)));
        for part in parts {
            match part {
                Either::Left(ident) => {
                    self.py.append(ident);
                }
                Either::Right(expr) => {
                    self.py.append(("{", expr.start_marker.span()));
                    self.append_inline_py_expr(expr);
                    self.py.append(("}", expr.end_marker.span()));
                }
            }
        }
        self.py.append("\", ");
        self.append_span_obj(Rc::clone(&full_span));
        self.py.append((")", full_span));
    }

    fn append_rust_code_as_parameter_list_element(&mut self, code: &RustCode) {
        match code {
            RustCode::Code(token) => match token {
                Token::Ident(ident) => {
                    self.py.append(PySrcSegment::new(
                        format!(r#"Ident("{}", "#, ident.inner()),
                        ident.span(),
                    ));
                    self.append_span_obj(ident.span());
                    self.py.append((")", ident.span()));
                    self.py.append(", ");
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
                        punct.span(),
                    ));
                    self.append_span_obj(punct.span());
                    self.py.append((")", punct.span()));
                    self.py.append(", ");
                }
                Token::Literal(literal) => {
                    let repr = literal.inner().to_string();
                    match repr.as_bytes() {
                        [b'"', ..] | [b'r', b'"', ..] => {
                            self.py.append((r#"StrLiteral("#, literal.span()));
                            self.py.append(PySrcSegment::new(
                                rust_string_repr_to_python_str_repr(&repr),
                                literal.span(),
                            ));
                            self.py.append((r#", "str", "#, literal.span()));
                        }
                        [b'\'', ..] => {
                            self.py.append(PySrcSegment::new(
                                format!(r#"StrLiteral({}, "chr", "#, rust_char_repr_to_python_str_repr(&repr)),
                                literal.span(),
                            ));
                        }
                        [b'b', b'"', ..] | [b'b', b'r', b'"', ..] => {
                            self.py.append((r#"BytesLiteral("#, literal.span()));
                            self.py.append(PySrcSegment::new(
                                rust_bytes_repr_to_python_bytes_repr(&repr),
                                literal.span(),
                            ));
                            self.py.append((r#", "bytes", "#, literal.span()));
                        }
                        [b'b', b'\'', ..] => {
                            self.py.append(PySrcSegment::new(
                                format!(
                                    r#"BytesLiteral({}, "byte", "#,
                                    rust_byte_repr_to_python_bytes_repr(&repr)
                                ),
                                literal.span(),
                            ));
                        }
                        [b'c', ..] => {
                            self.py.append((r#"BytesLiteral("#, literal.span()));
                            self.py.append(PySrcSegment::new(
                                rust_c_string_repr_to_python_bytes_repr(&repr),
                                literal.span(),
                            ));
                            self.py.append((r#", "cstr", "#, literal.span()));
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
                                literal.span(),
                            ));
                        }
                        _ => panic!("Failed to parse literal: {repr:?}"),
                    };

                    self.append_span_obj(literal.span());
                    self.py.append((")", literal.span()));
                    self.py.append(", ");
                }
                Token::Group(_) => unreachable!(), // should be handled by RustCode::Group
            },
            RustCode::Group { group, code } => {
                self.py.append(PySrcSegment::new(
                    format!(
                        r#"Group("{}", Tokens("#,
                        group.delimiter().left_right_str().unwrap_or("")
                    ),
                    group.span(),
                ));
                code.iter()
                    .for_each(|code| self.append_rust_code_as_parameter_list_element(code));
                self.py.append(("), ", group.span()));
                self.append_span_obj(group.span());
                self.py.append((")", group.span()));
                self.py.append(", ");
            }
            RustCode::PyExpr(expr) => {
                let span = Rc::new(CSpan::from(
                    expr.start_marker
                        .span()
                        .inner()
                        .join_or_fallback(Some(expr.end_marker.span().inner())),
                ));
                self.py.append(("Tokens(", Rc::clone(&span)));
                self.append_inline_py_expr(expr);
                self.py.append((", span=", Rc::clone(&span)));
                self.append_span_obj(Rc::clone(&span));
                self.py.append((")", span));
                self.py.append(", ");
            }
            RustCode::IdentWithPyExpr(parts) => {
                self.append_ident_with_inline_py_expr(parts);
                self.py.append(", ");
            }
        }
    }

    fn append_rust_code_region(&mut self, region: &[RustCode]) {
        if region.is_empty() {
            self.py.new_line(None);
            self.py.append("pass");
            return;
        }

        self.py.new_line(None);

        self.py.append("emit(");
        region
            .iter()
            .for_each(|code| self.append_rust_code_as_parameter_list_element(code));
        self.py.pop_last_segment_if(|seg| seg.code == ", ");
        self.py.append(")");
    }

    fn append_rust_multiline_block(&mut self, region: &RustCodeWithBlock) {
        let region_code = region.code.as_ref();

        self.py.new_line(None);

        self.py.append("with emit(");
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
        self.append_span_obj(region.group.span());
        self.py.append(")):");

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
            CodeRegion::MetaStmt(meta_stmt) => meta_stmt.codegen(self),
            CodeRegion::PurePyBlock(block) => self.append_code_regions(block.content.iter()),
        }
    }

    pub fn append_code_regions<'a>(&mut self, regions: impl Iterator<Item = &'a CodeRegion>) {
        regions.for_each(|region| self.append_code_region(region));
    }

    pub fn finish(self, package: Option<String>, name: String, filename: String) -> PyMetaModule {
        PyMetaModule {
            _package: package,
            name,
            filename,
            source: self.py.finish(),
        }
    }

    pub fn gen_from_code_regions<'a>(
        package: Option<String>,
        name: String,
        filename: String,
        regions: impl Iterator<Item = &'a CodeRegion>,
        ctx: &mut PyCodeGenContext,
    ) -> PyMetaModule {
        let mut generator = Self::new(ctx);
        generator.append_code_regions(regions);
        generator.finish(package, name, filename)
    }
}
