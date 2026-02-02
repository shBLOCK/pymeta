use crate::rust_to_py::utils::{PunctEx, TokenBufferEx};
use crate::utils::rust_token::{Group, Ident, Punct, Token, TokenBuffer};
use either::Either;
use proc_macro_error2::abort;
use proc_macro2::LineColumn;
use std::rc::Rc;

#[derive(Debug)]
pub(crate) struct PyLogicalLine {
    newline: Option<LineColumn>,
    marker: Rc<Punct>,
    tokens: Rc<[Token]>,
}

#[derive(Debug)]
pub(crate) struct InlinePyExpr {
    start_marker: Rc<Punct>,
    end_marker: Rc<Punct>,
    tokens: Rc<[Token]>,
}

type IdentWithInlinePyExpr = Box<[Either<Rc<Ident>, InlinePyExpr>]>;

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
    InlinePyExpr(InlinePyExpr),

    /// ```
    /// const FOO_@"A" * 10@_BAR = 42;
    /// ```
    /// TODO: better docs
    IdentWithInlinePyExpr(IdentWithInlinePyExpr),
}

#[derive(Debug)]
pub(crate) enum CodeRegion {
    /// Some Rust code. Can be multi-line, but **can not** contain non-inline Python code.
    ///
    /// This is turned into one `rust()` call in the generated Python code.
    ///
    /// A continuous region of Rust code is broken up into a number of these
    /// to make the resulting Python code more readable.
    RustCode(Box<[RustCode]>),

    /// Some Rust code followed by a multi-line block ([Group]).
    /// The block may contain Python code, both inline and non-inline ones.
    ///
    /// Rust code with multi-line blocks doesn't necessarily need to become this (instead of [Self::RustCode]),
    /// unless the blocks contain non-inline Python.
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
    /// ```
    /// @a = 1
    /// ```
    /// ---
    /// ```
    /// @print("Hello world!")
    /// ```
    /// ---
    /// ```
    /// @def foo(
    ///     x: int,
    ///     y: float
    /// ): print(x + y)
    /// ```
    /// This one won't count as a single "logical line" if the print() is in the next line.
    ///
    /// ---
    /// ```
    /// @a = [
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
    /// (A statement not starting at the beginning of a line is only allowed
    /// if it immediately (or after spaces) follows an indent block.)
    ///
    /// In the generated Python code, the `{}` of the indent block is stripped out,
    /// but explicit indentation is applied to the code within the block.
    PyStmtWithIndentBlock {
        line: PyLogicalLine,
        group: Rc<Group>,
        block: Box<[CodeRegion]>,
    },
}

impl CodeRegion {
    fn skip_py_marker_before_next_logical_newline_or_indent_block(
        tokens: &mut TokenBuffer,
    ) -> Result<&mut TokenBuffer, ()> {
        loop {
            if tokens.current().map_or(true, Token::is_newline) {
                return Err(());
            }
            if tokens.is_indent_block() {
                return Err(());
            }

            if tokens.is_py_marker_end() {
                tokens.seek(1).unwrap();
                return Ok(tokens);
            }

            tokens.seek(1);
        }
    }

    fn find_py_marker_end_before_next_logical_newline_or_indent_block(
        mut tokens: TokenBuffer,
    ) -> Option<Rc<Punct>> {
        Self::skip_py_marker_before_next_logical_newline_or_indent_block(&mut tokens).ok()?;
        Some(tokens.peek(-1).unwrap().punct().unwrap())
    }

    /// Parses a [Self::PyLogicalLine], or a [Self::PyStmtWithIndentBlock] if an indent block is found.
    fn parse_py_logical_line(tokens: &mut TokenBuffer) -> CodeRegion {
        let newline = if tokens.current().unwrap().is_newline() {
            Some(tokens.read_one().unwrap().newline().unwrap())
        } else {
            None
        };
        let marker = tokens.read_one().unwrap().punct().unwrap();
        assert!(marker.is_py_marker());

        let mut region_tokens = Vec::<Token>::new();

        while !tokens.current().map_or(true, Token::is_newline) {
            let token = tokens.read_one().unwrap();
            region_tokens.push(token.clone());
            if tokens.seeked(-1).unwrap().is_indent_block() {
                // found indent block
                let group = tokens.read_one().unwrap().group().unwrap();
                let block = Self::parse(group.tokens());
                return CodeRegion::PyStmtWithIndentBlock {
                    line: PyLogicalLine {
                        newline,
                        marker,
                        tokens: Rc::from(region_tokens),
                    },
                    group,
                    block,
                };
            }
        }

        CodeRegion::PyLogicalLine(PyLogicalLine {
            newline,
            marker,
            tokens: Rc::from(region_tokens),
        })
    }

    /// Parses a [Self::RustCode] or a [Self::RustMultilineBlock].
    fn parse_rust(tokens: &mut TokenBuffer) -> Either<Box<[RustCode]>, CodeRegion> {
        fn parse_inline_py_expr(tokens: &mut TokenBuffer) -> InlinePyExpr {
            let start = tokens.read_one().unwrap().punct().unwrap();
            assert!(start.is_py_marker());
            let mut py_tokens = Vec::new();
            while !tokens.is_py_marker_end() {
                let Some(token) = tokens.read_one() else {
                    abort!(
                        start
                            .span()
                            .inner()
                            .join(tokens.peek(-1).unwrap().span().unwrap().inner()),
                        "Incomplete inline Python expression (unexpected EOF)."
                    )
                };
                if token.is_newline() {
                    abort!(
                        start
                            .span()
                            .inner()
                            .join(tokens.peek(-1).unwrap().span().unwrap().inner()),
                        "Incomplete inline Python expression (unexpected end of line)."
                    )
                }
                py_tokens.push(token.clone());
            }
            InlinePyExpr {
                start_marker: start,
                end_marker: tokens.read_one().unwrap().punct().unwrap(),
                tokens: Rc::from(py_tokens),
            }
        }

        fn parse_ident_with_inline_py_expr(tokens: &mut TokenBuffer) -> IdentWithInlinePyExpr {
            let mut parts = Vec::new();
            while !tokens.reached_end() {
                let token = tokens.current().unwrap();
                if let Some(ident) = token.ident() {
                    parts.push(Either::Left(ident));
                    tokens.seek(1).unwrap();
                } else if tokens.is_py_marker_start() {
                    parts.push(Either::Right(parse_inline_py_expr(tokens)));
                } else {
                    break;
                }
            }
            Box::from(parts)
        }

        fn detect_ident_with_inline_py_expr(mut tokens: TokenBuffer) -> bool {
            let mut found_ident = false;
            let mut found_py_expr = false;
            loop {
                if tokens.is_py_marker_start() {
                    let start = tokens.read_one().unwrap().punct().unwrap();
                    if CodeRegion::skip_py_marker_before_next_logical_newline_or_indent_block(
                        &mut tokens,
                    )
                    .is_err()
                    {
                        abort!(
                            start
                                .span()
                                .inner()
                                .join(tokens.peek(-1).unwrap().span().unwrap().inner()),
                            "Incomplete inline Python expression."
                        );
                    }
                    found_py_expr = true;
                } else if let Some(&Token::Ident(_)) = tokens.current() {
                    found_ident = true;
                } else {
                    return false;
                }

                if found_ident && found_py_expr {
                    return true;
                }
            }
        }

        let mut code = Vec::new();

        loop {
            if detect_ident_with_inline_py_expr(tokens.clone()) {
                code.push(RustCode::IdentWithInlinePyExpr(
                    parse_ident_with_inline_py_expr(tokens),
                ));
            } else if tokens.is_py_marker_start() {
                code.push(RustCode::InlinePyExpr(parse_inline_py_expr(tokens)));
            } else {
                // normal code token or group

                let Some(token) = tokens.current() else { break };
                if token.is_newline() {
                    break;
                }

                // by now, we are committed to use this token, so consume it
                let token = tokens.read_one().unwrap();

                if let Some(group) = token.group() {
                    let parsed = Self::parse(group.tokens());
                    if parsed.iter().any(|region| match region {
                        Self::RustCode(_) => false,
                        Self::RustMultilineBlock { .. } => true,
                        Self::PyLogicalLine(_) => true,
                        Self::PyStmtWithIndentBlock { .. } => true,
                    }) {
                        let multiline = Self::RustMultilineBlock {
                            code: Box::from(code),
                            group,
                            block: parsed,
                        };
                        if let Some(Token::Spaces(_)) = tokens.current() {
                            // consume the space after the multiline block, if there is any
                            tokens.seek(1).unwrap();
                        }
                        return Either::Right(multiline);
                    } else {
                        let code_group = RustCode::Group {
                            group,
                            code: parsed
                                .into_iter()
                                .flat_map(|region| match region {
                                    Self::RustCode(code) => code,
                                    _ => unreachable!(),
                                })
                                .collect(),
                        };
                        code.push(code_group);
                    }
                } else {
                    code.push(RustCode::Code(token.clone()));
                }
            }
        }
        Either::Left(Box::from(code))
    }

    /// Parse tokens into [CodeRegion]s.
    ///
    /// [tokens] must be initially at a [Token::NewLine], or be empty.
    pub fn parse(mut tokens: TokenBuffer) -> Box<[Self]> {
        assert!(
            tokens.current().map_or(true, Token::is_newline),
            "Must start at a `Token::NewLine`."
        );

        let mut regions = Vec::new();

        while !tokens.reached_end() {
            let before_pos = tokens.pos();

            let region = 'parse_one: {
                // parse Python logical line at the start of a new line
                if tokens.current().unwrap().is_newline() {
                    if tokens.have_n_more(2) && tokens.seeked(1).unwrap().is_py_marker_start() {
                        if Self::find_py_marker_end_before_next_logical_newline_or_indent_block(
                            tokens.seeked(2).unwrap(),
                        )
                        .is_none()
                        {
                            break 'parse_one Self::parse_py_logical_line(&mut tokens);
                        }
                    }
                }

                // parse a PyStmtWithIndentBlock immediately after a PyStmtWithIndentBlock,
                // or generate an error if other stuff immediately follows the previous PyStmtWithIndentBlock.
                if let Some(Self::PyStmtWithIndentBlock { .. }) = regions.last() {
                    if tokens.have_n_more(2) && !tokens.current().unwrap().is_newline() {
                        // check for a valid start marker, skipping a Token::Spaces if necessary.
                        if tokens.is_py_marker_start() {
                        } else if tokens.current().unwrap().is_spaces()
                            && tokens.seeked(1).unwrap().is_py_marker_start()
                        {
                            tokens.seek(1).unwrap(); // skip spaces
                        } else {
                            abort!(
                                tokens.get_current_span_for_diagnostics(),
                                "Only another Python statement with an indent block can immediately follow an indent block."
                            );
                        };

                        if let Some(end_marker) =
                            Self::find_py_marker_end_before_next_logical_newline_or_indent_block(
                                tokens.seeked(1).unwrap(),
                            )
                        {
                            let start_marker = tokens.current().unwrap().punct().unwrap();
                            abort!(
                                start_marker.span().inner().join(end_marker.span().inner()),
                                "Python expression can not immediately follow an indent block."
                            );
                        }

                        break 'parse_one Self::parse_py_logical_line(&mut tokens);
                    }
                }

                Self::parse_rust(&mut tokens)
                    .map_left(CodeRegion::RustCode)
                    .into_inner()
            };

            regions.push(region);

            let after_pos = tokens.pos();
            if after_pos == before_pos {
                abort!(
                    tokens.get_current_span_for_diagnostics(),
                    "BUG: CodeRegion parser got stuck, aborting to avoid infinite loop."
                );
            }
        }

        regions.into()
    }
}
