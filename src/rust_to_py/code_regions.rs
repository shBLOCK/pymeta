use crate::utils::rust_token::{Group, Ident, Punct, Token};
use either::Either;
use std::rc::Rc;

#[derive(Debug)]
pub(crate) struct PyStmt {
    pub _marker: Rc<Punct>,
    pub tokens: Rc<[Token]>,
}

#[derive(Debug)]
pub(crate) struct PyExpr {
    pub start_marker: Rc<Punct>,
    pub end_marker: Rc<Punct>,
    pub tokens: Rc<[Token]>,
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
    /// const FOO = @u16(2 ** 10)@;
    /// ```
    /// TODO: better docs
    PyExpr(PyExpr),

    /// ```
    /// const FOO_@"A" * 10@_BAR = 42;
    /// ```
    /// TODO: better docs
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
    /// @for i in range(10):{
    ///     ...
    /// }
    /// ```
    /// Or:
    /// ```
    /// @if foo:{
    ///     ...
    /// } @else if bar:{
    ///     ...
    /// } @else:{
    ///     ...
    /// }
    /// ```
    ///
    /// In the generated Python code, the `{}` of the indent block is stripped out,
    /// but explicit indentation is applied to the code within the block.
    PyStmtWithIndentBlock(PyStmtWithIndentBlock),
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
    use super::{CodeRegion, PyExpr, PyStmt, PyStmtWithIndentBlock, RustCode, RustCodeWithBlock};
    use crate::rust_to_py::CONCAT_MARKER;
    use crate::rust_to_py::utils::TokenBufferEx;
    use crate::utils::match_unwrap;
    use crate::utils::rust_token::{Token, TokenBuffer};
    use either::Either;
    use proc_macro_error2::abort;
    use std::rc::Rc;

    pub(crate) struct CodeRegionParser {
        regions: Vec<CodeRegion>,
    }

    enum ParsePyResult {
        Expr(PyExpr),
        Stmt(PyStmt),
        StmtWithIndentBlock(PyStmtWithIndentBlock),
    }

    impl CodeRegionParser {
        pub(crate) fn new() -> Self {
            Self { regions: Vec::new() }
        }

        fn parse_py(tokens: &mut TokenBuffer) -> Option<ParsePyResult> {
            if !tokens.is_py_marker_start() {
                return None;
            }
            let start = tokens.read_one().unwrap().punct().unwrap();

            let mut py_tokens = Vec::new();

            loop {
                if tokens.peek(-1).unwrap().eq_punct(';') {
                    return Some(ParsePyResult::Stmt(PyStmt {
                        _marker: start,
                        tokens: py_tokens.into(),
                    }));
                }

                if tokens.is_py_marker_end() {
                    let end = tokens.read_one().unwrap().punct().unwrap();
                    return Some(ParsePyResult::Expr(PyExpr {
                        start_marker: start,
                        end_marker: end,
                        tokens: py_tokens.into(),
                    }));
                }

                if tokens.is_indent_block() {
                    py_tokens.push(tokens.read_one().unwrap().clone());
                    let group = tokens.read_one().unwrap().group().unwrap();
                    let group_tokens = group.tokens();
                    return Some(ParsePyResult::StmtWithIndentBlock(PyStmtWithIndentBlock {
                        stmt: PyStmt {
                            _marker: start,
                            tokens: py_tokens.into(),
                        },
                        group,
                        block: CodeRegionParser::new().parse(group_tokens).into(),
                    }));
                }

                if tokens.is_py_marker_escape() {
                    py_tokens.push(Token::Punct(tokens.read_unescaped_py_marker_escape()));
                } else {
                    let Some(token) = tokens.read_one() else {
                        abort!(
                            tokens.peek(-1).unwrap().span().inner().unwrap().end(),
                            "Incomplete Python segment."
                        );
                    };
                    py_tokens.push(token.clone())
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
            while !tokens.exausted() {
                if let Some(py) = Self::parse_py(&mut tokens) {
                    match (self.regions.last_mut(), py) {
                        (_, ParsePyResult::Stmt(stmt)) => self.regions.push(CodeRegion::PyStmt(stmt)),
                        (_, ParsePyResult::StmtWithIndentBlock(stmt)) => {
                            self.regions.push(CodeRegion::PyStmtWithIndentBlock(stmt))
                        }
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
                    }
                } else {
                    let token = if tokens.is_py_marker_escape() {
                        &Token::Punct(tokens.read_unescaped_py_marker_escape())
                    } else {
                        tokens.read_one().unwrap()
                    };
                    match token {
                        Token::Group(group) => {
                            let group_regions = Self::new().parse(group.tokens());
                            if group_regions
                                .iter()
                                .all(|region| matches!(region, CodeRegion::RustCode(_)))
                            {
                                // simple region with rust code
                                let group_code = RustCode::Group {
                                    group: Rc::clone(group),
                                    code: group_regions
                                        .into_iter()
                                        .map(|region| {
                                            if let CodeRegion::RustCode(code) = region {
                                                code
                                            } else {
                                                unreachable!()
                                            }
                                        })
                                        .flatten()
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
                                //TODO: only do this for braces, or only when the region is too long
                                if token.eq_punct(';') {
                                    self.regions.push(CodeRegion::RustCode(Vec::new()));
                                }
                            }
                        }
                    }
                }
            }
            self.regions = self
                .regions
                .into_iter()
                .filter(|region| !matches!(region, CodeRegion::RustCode(code) if code.is_empty()))
                .collect();

            self.regions.into()
        }
    }
}
