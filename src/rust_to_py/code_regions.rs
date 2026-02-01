use crate::utils::rust_token::{Group, Ident, Punct, Token};
use proc_macro2::LineColumn;
use std::rc::Rc;

pub(crate) struct PyLogicalLine {
    newline: LineColumn,
    marker: Rc<Punct>,
    tokens: Rc<[Token]>,
}

pub(crate) struct InlinePyExpr {
    start_marker: Rc<Punct>,
    end_marker: Rc<Punct>,
    tokens: Rc<[Token]>,
}

pub(crate) enum IdentOrPyExpr {
    Ident(Rc<Ident>),
    PyExpr(InlinePyExpr),
}

pub(crate) enum RustCode {
    Code(Rc<[Token]>),
    Region(CodeRegion),
    Group { group: Rc<Group>, code: Box<[RustCode]> },
}

pub(crate) enum CodeRegion {
    /// Some Rust code. Can be multi-line, but **can not** contain non-inline Python code.
    ///
    /// This is turned into one `rust()` call in the generated Python code.
    ///
    /// A continuous region of Rust code is broken up into a number of these
    /// to make the resulting Python code more readable.
    RustCode(Box<[RustCode]>),

    /// Some Rust code followed by a multi-line block ([Group]). The block **can** contain Python code.
    ///
    /// This is turned into a `with rust():` block in Python.
    /// Using context manager allows any Python code contained within this to work properly,
    /// and emit code to the correct block ([Group]).
    RustMultilineBlock {
        code: Box<[RustCode]>,
        group: Rc<Group>,
        block: Box<[CodeRegion]>,
    },

    /// A "logical line" of Python code.
    ///
    /// A "logical line" is a continuous region of code without line breaks,
    /// but line breaks **within** [Group]s are allowed.
    ///
    /// Examples of "logical lines":
    /// ```python
    /// a = 1
    /// ```
    /// ---
    /// ```python
    /// print("Hello world!")
    /// ```
    /// ---
    /// ```python
    /// def foo(
    ///     x: int,
    ///     y: float
    /// ): print(x + y)  # this won't count if the print() is in the next line
    /// ```
    /// ---
    /// ```python
    /// a = [
    ///     1, 2, 3
    /// ] + [
    ///     "a",
    ///     "b",
    ///     "c"
    /// ]
    /// ```
    PyLogicalLine(PyLogicalLine),

    /// A "logical line" of Python code, followed by an "indent block".
    ///
    /// Something like:
    /// ```python
    /// for i in range(10):{
    ///     ...
    /// }
    /// ```
    ///
    /// In the generated Python code, the `{}` of the indent block is stripped out,
    /// but explicit indentation is applied to the code within the block.
    PyStmtWithIndentBlock {
        line: PyLogicalLine,
        group: Rc<Group>,
        block: Box<[CodeRegion]>,
    },

    /// DOCS TODO
    InlinePyExpr(InlinePyExpr),

    /// DOCS TODO
    IdentWithInlinePyExpr(Box<[IdentOrPyExpr]>),
}

impl CodeRegion {
    pub fn parse_regions() -> Box<[Self]> {
        let mut regions = Vec::new();



        regions.into()
    }
}