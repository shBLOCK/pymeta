use crate::rust_to_py::meta::expr::MetaExpr;
use crate::rust_to_py::meta::stmt::MetaStmt;
use crate::utils::rust_token::{Group, Ident, Punct, Token};
use either::Either;
use std::rc::Rc;

#[derive(Debug)]
pub(crate) enum PySegment {
    Token(Token),
    Group {
        group: Rc<Group>,
        segments: Rc<[PySegment]>,
    },
    MetaExpr(MetaExpr),
    Quote {
        outer_group: Rc<Group>,
        _inner_group: Rc<Group>,
        content: Box<[CodeRegion]>,
    },
}

#[derive(Debug)]
pub(crate) struct PyStmt {
    pub _marker: Option<Rc<Punct>>,
    pub segments: Rc<[PySegment]>,
}

#[derive(Debug)]
pub(crate) struct PyExpr {
    pub start_marker: Rc<Punct>,
    pub end_marker: Rc<Punct>,
    pub segments: Rc<[PySegment]>,
}

pub(crate) type IdentWithPyExpr = Vec<Either<Rc<Ident>, PyExpr>>;

#[derive(Debug)]
pub(crate) enum RustCode {
    Code(Token),
    Group {
        group: Rc<Group>,
        code: Box<[RustCode]>,
    },

    /// ```
    /// const FOO = $u16(2 ** 10)$;
    /// ```
    PyExpr(PyExpr),

    /// ```
    /// const FOO~$"A" * 10$~BAR = 42;
    /// ```
    IdentWithPyExpr(IdentWithPyExpr),
}

#[derive(Debug)]
pub(crate) struct RustCodeWithBlock {
    pub code: Box<[RustCode]>,
    pub group: Rc<Group>,
    pub block: Box<[CodeRegion]>,
}

#[derive(Debug)]
pub(crate) struct PyStmtWithIndentBlock {
    pub stmt: PyStmt,
    pub group: Rc<Group>,
    pub block: Box<[CodeRegion]>,
}

#[derive(Debug)]
pub(crate) struct PurePyBlock {
    pub _marker: Rc<Punct>,
    pub _group: Rc<Group>,
    pub content: Box<[CodeRegion]>,
}

#[derive(Debug)]
pub(crate) enum CodeRegion {
    /// Some Rust code. **Can not** contain non-inline Python code.
    ///
    /// This is turned into one `rust()` call in the generated Python code.
    ///
    /// A continuous region of Rust code may be broken up into a number of these
    /// to make the resulting Python code more readable.
    RustCode(Vec<RustCode>),

    /// Some Rust code followed by a block ([Group]).
    /// The block may contain Python code, both inline and non-inline ones.
    ///
    /// This is turned into a `with rust():` block in Python.
    /// Using context manager allows any Python code contained within this to work properly,
    /// and emit code to the correct block ([Group]).
    RustCodeWithBlock(RustCodeWithBlock),

    /// A Python statement.
    PyStmt(PyStmt),

    /// A Python statement, followed by an "indent block".
    ///
    /// Something like:
    /// ```
    /// $for i in range(10):{
    ///     ...
    /// }
    /// ```
    /// Or:
    /// ```
    /// $if foo:{
    ///     ...
    /// } $else if bar:{
    ///     ...
    /// } $else:{
    ///     ...
    /// }
    /// ```
    ///
    /// In the generated Python code, the `{}` of the indent block is stripped out,
    /// but explicit indentation is applied to the code within the block.
    PyStmtWithIndentBlock(PyStmtWithIndentBlock),

    MetaStmt(MetaStmt),

    PurePyBlock(PurePyBlock),
}

macro_rules! impl_code_region_from_inner {
    ($name:ident, $inner:ty) => {
        #[allow(unused)]
        impl From<$inner> for CodeRegion {
            fn from(value: $inner) -> Self {
                Self::$name(value)
            }
        }
    };
}
impl_code_region_from_inner!(RustCode, Vec<RustCode>);
impl_code_region_from_inner!(RustCodeWithBlock, RustCodeWithBlock);
impl_code_region_from_inner!(PyStmt, PyStmt);
impl_code_region_from_inner!(PyStmtWithIndentBlock, PyStmtWithIndentBlock);

pub(crate) mod parser {
    use super::{
        CodeRegion, PurePyBlock, PyExpr, PySegment, PyStmt, PyStmtWithIndentBlock, RustCode, RustCodeWithBlock,
    };
    use crate::abort;
    use crate::rust_to_py::CONCAT_MARKER;
    use crate::rust_to_py::meta::expr::MetaExpr;
    use crate::rust_to_py::meta::stmt::{ImportMetaStmt, MetaStmt, MetaStmtBody};
    use crate::rust_to_py::utils::TokenBufferEx;
    use crate::utils::match_unwrap;
    use crate::utils::parsing::RustSimplePath;
    use crate::utils::rust_token::{Token, TokenBuffer, TokenOptionEx};
    use either::Either;
    use proc_macro2::Delimiter;
    use std::rc::Rc;

    #[derive(Clone)]
    pub(crate) struct CodeRegionParserSettings {
        pub pure_python_mode: bool,
    }
    impl Default for CodeRegionParserSettings {
        fn default() -> Self {
            Self { pure_python_mode: false }
        }
    }

    pub(crate) struct CodeRegionParserCtx {
        pub import_paths: Vec<Rc<RustSimplePath>>,
    }

    impl CodeRegionParserCtx {
        pub fn new() -> Self {
            Self { import_paths: Vec::new() }
        }
    }

    /// [Self::parse] parses a [TokenBuffer] into [CodeRegion]s
    pub(crate) struct CodeRegionParser<'a> {
        settings: CodeRegionParserSettings,
        regions: Vec<CodeRegion>,
        ctx: &'a mut CodeRegionParserCtx,
    }

    enum ParsePyResult {
        Expr(PyExpr),
        Stmt(PyStmt),
        StmtWithIndentBlock(PyStmtWithIndentBlock),
        MetaStmt(MetaStmt),
        PurePyBlock(PurePyBlock),
    }

    impl CodeRegionParser<'_> {
        pub(crate) fn new(settings: CodeRegionParserSettings, ctx: &mut CodeRegionParserCtx) -> CodeRegionParser<'_> {
            CodeRegionParser { settings, regions: Vec::new(), ctx }
        }

        fn new_derived(parent: &mut Self, f: impl FnOnce(&mut CodeRegionParserSettings)) -> CodeRegionParser<'_> {
            let mut settings = parent.settings.clone();
            f(&mut settings);
            CodeRegionParser::new(settings, parent.ctx)
        }

        fn parse_py_segment(&mut self, tokens: &mut TokenBuffer) -> PySegment {
            if tokens.is_py_marker_escape() {
                PySegment::Token(Token::Punct(tokens.read_unescaped_py_marker_escape()))
            } else if let Some(meta_expr) = MetaExpr::parse(tokens) {
                PySegment::MetaExpr(meta_expr)
            } else if let Some((outer, inner)) = tokens.read_double_group(Delimiter::Brace) {
                let content = Self::new_derived(self, |s| s.pure_python_mode = false).parse(inner.tokens());
                PySegment::Quote {
                    outer_group: outer,
                    _inner_group: inner,
                    content,
                }
            } else {
                match tokens.read_one() {
                    Some(Token::Group(group)) => {
                        let mut group_tokens = group.tokens();
                        let mut segments = Vec::new();
                        while !group_tokens.exhausted() {
                            segments.push(self.parse_py_segment(&mut group_tokens));
                        }
                        PySegment::Group {
                            group: group.clone(),
                            segments: Rc::from(segments),
                        }
                    }
                    Some(token) => PySegment::Token(token.clone()),
                    None => abort!(tokens.get_current_span_for_diagnostics(), "Incomplete Python segment."),
                }
            }
        }

        fn parse_py(&mut self, tokens: &mut TokenBuffer) -> Option<ParsePyResult> {
            let start_marker = if !self.settings.pure_python_mode {
                if !tokens.is_py_marker() {
                    return None;
                }
                Some(tokens.read_py_marker())
            } else {
                None
            };

            // pure Python block
            if !self.settings.pure_python_mode
                && let Ok(group) = tokens.current().expect_group(Delimiter::Brace)
            {
                tokens.seek(1);
                let content = Self::new_derived(self, |s| s.pure_python_mode = true).parse(group.tokens());
                return Some(ParsePyResult::PurePyBlock(PurePyBlock {
                    _marker: start_marker.unwrap(),
                    _group: group,
                    content,
                }));
            }

            if let Some(meta_stmt) = MetaStmt::try_parse(tokens) {
                #[allow(irrefutable_let_patterns)]
                if let MetaStmtBody::Import(ImportMetaStmt { path, .. }) = &meta_stmt.body {
                    if !self.ctx.import_paths.contains(path) {
                        self.ctx.import_paths.push(path.clone());
                    }
                }
                return Some(ParsePyResult::MetaStmt(meta_stmt));
            }

            let mut py_segments = Vec::new();

            loop {
                if !self.settings.pure_python_mode && tokens.is_py_marker() {
                    let end = tokens.read_py_marker();
                    return Some(ParsePyResult::Expr(PyExpr {
                        start_marker: start_marker.unwrap(),
                        end_marker: end,
                        segments: py_segments.into(),
                    }));
                }

                // indent block
                if let Some((colon, group, block)) = tokens.try_run_or_rewind(|tokens| {
                    let colon = tokens.read_one().expect_punct(':').ok()?;
                    let pure_py_marker = if tokens.is_py_marker() {
                        Some(tokens.read_py_marker())
                    } else {
                        None
                    };
                    let group = tokens.read_one().expect_group(Delimiter::Brace).ok()?;
                    let block = Self::new_derived(self, |s| {
                        if pure_py_marker.is_some() {
                            s.pure_python_mode = true;
                        }
                    })
                    .parse(group.tokens());
                    Some((colon, group, block))
                }) {
                    py_segments.push(PySegment::Token(Token::Punct(colon)));
                    return Some(ParsePyResult::StmtWithIndentBlock(PyStmtWithIndentBlock {
                        stmt: PyStmt {
                            _marker: start_marker,
                            segments: py_segments.into(),
                        },
                        group,
                        block,
                    }));
                };

                py_segments.push(self.parse_py_segment(tokens));

                if tokens.peek(-1).eq_punct(';') {
                    return Some(ParsePyResult::Stmt(PyStmt {
                        _marker: start_marker,
                        segments: py_segments.into(),
                    }));
                }
            }
        }

        fn get_or_put_rust_code_region(&mut self) -> &mut Vec<RustCode> {
            if let Some(CodeRegion::RustCode(_)) = self.regions.last() {
                match_unwrap!(code in CodeRegion::RustCode(code) = self.regions.last_mut().unwrap())
            } else {
                match_unwrap!(code in CodeRegion::RustCode(code) = self.regions.push_mut(CodeRegion::RustCode(Vec::new())))
            }
        }

        pub(crate) fn parse(mut self, mut tokens: TokenBuffer) -> Box<[CodeRegion]> {
            while !tokens.exhausted() {
                if let Some(py) = self.parse_py(&mut tokens) {
                    match (self.regions.last_mut(), py) {
                        (_, ParsePyResult::Stmt(stmt)) => self.regions.push(CodeRegion::PyStmt(stmt)),
                        (_, ParsePyResult::StmtWithIndentBlock(stmt)) => {
                            self.regions.push(CodeRegion::PyStmtWithIndentBlock(stmt))
                        }
                        (_, ParsePyResult::MetaStmt(meta_stmt)) => self.regions.push(CodeRegion::MetaStmt(meta_stmt)),
                        (Some(CodeRegion::RustCode(code)), ParsePyResult::Expr(expr)) => {
                            match &mut code[..] {
                                [
                                    ..,
                                    RustCode::IdentWithPyExpr(code),
                                    RustCode::Code(Token::Punct(concat)),
                                ] if concat.eq_punct(CONCAT_MARKER) => {
                                    let _concat_marker = code.pop();
                                    code.push(Either::Right(expr));
                                }
                                [
                                    ..,
                                    RustCode::Code(Token::Ident(_)),
                                    RustCode::Code(Token::Punct(concat)),
                                ] if concat.eq_punct(CONCAT_MARKER) => {
                                    let _concat_marker = code.pop();
                                    let RustCode::Code(Token::Ident(ident)) = code.pop().unwrap() else {
                                        unreachable!()
                                    }; // ident
                                    code.push(RustCode::IdentWithPyExpr(vec![
                                        Either::Left(Rc::clone(&ident)),
                                        Either::Right(expr),
                                    ]));
                                }
                                _ => code.push(RustCode::PyExpr(expr)),
                            }
                        }
                        (_, ParsePyResult::Expr(expr)) => {
                            self.regions.push(CodeRegion::RustCode(vec![RustCode::PyExpr(expr)]))
                        }
                        (_, ParsePyResult::PurePyBlock(block)) => self.regions.push(CodeRegion::PurePyBlock(block)),
                    }
                } else {
                    assert!(!self.settings.pure_python_mode);
                    let token = if tokens.is_py_marker_escape() {
                        &Token::Punct(tokens.read_unescaped_py_marker_escape())
                    } else {
                        tokens.read_one().unwrap()
                    };
                    match token {
                        Token::Group(group) => {
                            let group_regions = Self::new_derived(&mut self, |_| {}).parse(group.tokens());
                            if group_regions
                                .iter()
                                .all(|region| matches!(region, CodeRegion::RustCode(_)))
                            {
                                // simple region with rust code
                                let group_code = RustCode::Group {
                                    group: Rc::clone(group),
                                    code: group_regions
                                        .into_iter()
                                        .flat_map(|region| {
                                            if let CodeRegion::RustCode(code) = region {
                                                code
                                            } else {
                                                unreachable!()
                                            }
                                        })
                                        .collect(),
                                };
                                self.get_or_put_rust_code_region().push(group_code);
                            } else {
                                // a region with python statements
                                let code = self
                                    .regions
                                    .pop_if(|region| matches!(region, CodeRegion::RustCode(_)))
                                    .map(|region| match_unwrap!(code in CodeRegion::RustCode(code) = region));

                                self.regions.push(CodeRegion::RustCodeWithBlock(RustCodeWithBlock {
                                    code: code.unwrap_or_else(Vec::new).into_boxed_slice(),
                                    group: Rc::clone(group),
                                    block: group_regions,
                                }));
                            }
                        }
                        token => {
                            if let Token::Ident(ident) = token
                                && let Some(CodeRegion::RustCode(code)) = self.regions.last_mut()
                                && let [
                                    ..,
                                    RustCode::PyExpr(_) | RustCode::IdentWithPyExpr(_),
                                    RustCode::Code(Token::Punct(concat)),
                                ] = &code[..]
                                && concat.eq_punct(CONCAT_MARKER)
                            {
                                // Python expr followed by CONCAT_MARKER and then by the current token which is an ident => make a IdentWithPyExpr
                                let _concat_marker = code.pop();
                                // if last is PyExpr, turn it into a IdentWithPyExpr
                                if let Some(expr) = code.pop_if(|expr| matches!(expr, RustCode::PyExpr(_))) {
                                    code.push(RustCode::IdentWithPyExpr(vec![Either::Right(
                                        match_unwrap!(expr in RustCode::PyExpr(expr) = expr),
                                    )]));
                                }
                                match_unwrap!(iwp in Some(RustCode::IdentWithPyExpr(iwp)) = code.last_mut())
                                    .push(Either::Left(Rc::clone(ident)));
                            } else {
                                // a normal token
                                self.get_or_put_rust_code_region().push(RustCode::Code(token.clone()));

                                // start a new region on semicolons to make the resulting Python code more readable
                                //TODO: only do this for braces, or only when the region is too long (avoid doing it for `[f32; 3]` for example)
                                if token.eq_punct(';') {
                                    self.regions.push(CodeRegion::RustCode(Vec::new()));
                                }
                            }
                        }
                    }
                }
            }
            self.regions
                .retain(|region| !matches!(region, CodeRegion::RustCode(code) if code.is_empty()));

            self.regions.into()
        }

        // pub(crate) fn parse(tokens: TokenBuffer, ctx: &'_ mut CodeRegionParserCtx) -> Box<[CodeRegion]> {
        //     Self::new(ctx)._parse(tokens)
        // }
    }
}
