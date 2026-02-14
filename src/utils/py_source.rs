use crate::utils::rust_token::CSpan;
use crate::utils::SpanEx;
use proc_macro2::Span;
use std::borrow::Cow;
use std::fmt::Write;
use std::iter::repeat_n;
use std::rc::Rc;

#[derive(Debug)]
pub(crate) struct PySegment {
    pub code: Cow<'static, str>,
    pub src_span: Option<Rc<CSpan>>,
}

impl PySegment {
    pub fn new(code: impl Into<Cow<'static, str>>, src_span: Option<Rc<CSpan>>) -> PySegment {
        Self {
            code: code.into(),
            src_span,
        }
    }

    pub fn add_to_string(&self, string: &mut String) {
        string.push_str(&self.code);
    }

    pub fn join_src_spans<'a>(segments: impl Iterator<Item = &'a PySegment>) -> Option<Span> {
        segments
            .map(|seg| seg.src_span.as_ref().map(|s| s.inner()))
            .reduce(|a, b| match (a, b) {
                (Some(a), Some(b)) => Some(a.join_or_fallback(Some(b))),
                (a, b) => a.or(b),
            })
            .flatten()
    }
}

#[derive(Debug)]
pub(crate) struct PyLine {
    pub segments: Box<[Rc<PySegment>]>,
    pub indent: usize,
}

impl PyLine {
    fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    pub fn add_to_string(&self, string: &mut String) {
        string.extend(repeat_n(' ', self.indent));
        self.segments
            .iter()
            .for_each(|segment| segment.add_to_string(string));
        string.push('\n');
    }
}

#[derive(Debug)]
pub(crate) struct PySource {
    pub lines: Box<[PyLine]>,
}

impl PySource {
    pub fn source_code(&self) -> String {
        let mut string = String::new();
        self.lines
            .iter()
            .for_each(|line| line.add_to_string(&mut string));
        string
    }

    pub fn diagnostic_source_dump(&self) -> String {
        if self.lines.is_empty() {
            return String::from("<Python source is empty>");
        }
        let lineno_digits = self.lines.len().ilog10() + 1;
        let mut string = String::new();
        for (lineno, line) in (1..).zip(self.lines.iter()) {
            write!(
                string,
                "{lineno:>width$} | ",
                width = lineno_digits as usize
            )
            .unwrap();
            line.add_to_string(&mut string);
        }
        string.pop(); // pop last newline
        string
    }
}

pub(crate) mod builder {
    use super::{PySegment, PySource};
    use either::Either;
    use std::cell::{RefCell, RefMut};
    use std::mem;
    use std::rc::Rc;

    #[derive(Debug)]
    struct PyLine {
        segments: Vec<Rc<PySegment>>,
        indent: Option<usize>,
    }

    impl PyLine {
        fn new(indent: Option<usize>) -> Self {
            Self {
                segments: Vec::new(),
                indent,
            }
        }

        fn append(&mut self, segment: PySegment) {
            self.segments.push(Rc::new(segment));
        }
    }

    #[derive(Debug)]
    struct IndentBlock {
        content: Vec<Either<PyLine, Rc<RefCell<IndentBlock>>>>,
        indent: usize,
    }

    impl IndentBlock {
        fn new(indent: usize) -> Self {
            Self {
                content: Vec::new(),
                indent,
            }
        }

        fn append(&mut self, segment: PySegment) {
            self.content
                .last_mut()
                .expect("Appending to an empty IndentBlock")
                .as_mut()
                .expect_left("The last element is not a PyLine")
                .append(segment);
        }

        fn new_line(&mut self, indent: Option<usize>) {
            self.content.push(Either::Left(PyLine::new(indent)))
        }

        fn new_indent_block(&mut self, indent: usize) -> Rc<RefCell<IndentBlock>> {
            let block = Rc::new(RefCell::new(IndentBlock::new(indent)));
            self.content.push(Either::Right(Rc::clone(&block)));
            block
        }
    }

    pub(crate) struct PySourceBuilder {
        indent_block_stack: Vec<Rc<RefCell<IndentBlock>>>,
    }

    impl PySourceBuilder {
        pub fn new() -> Self {
            Self {
                indent_block_stack: vec![Rc::new(RefCell::new(IndentBlock::new(0)))],
            }
        }

        fn top(&mut self) -> RefMut<'_, IndentBlock> {
            self.indent_block_stack.last_mut().unwrap().borrow_mut()
        }

        pub fn append(&mut self, segment: PySegment) {
            self.top().append(segment);
        }

        pub fn new_line(&mut self, indent: Option<usize>) {
            self.top().new_line(indent);
        }

        pub fn push_indent_block(&mut self, indent: usize) {
            let block = { self.top().new_indent_block(indent) };
            self.indent_block_stack.push(block);
        }

        pub fn pop_indent_block(&mut self) {
            assert!(
                self.indent_block_stack.len() > 1,
                "root indent block can't be popped (push_indent_block / pop_indent_block mismatch?)"
            );
            self.indent_block_stack.pop();
        }

        pub fn finish(self) -> PySource {
            assert_eq!(
                self.indent_block_stack.len(),
                1,
                "push_indent_block / pop_indent_block mismatch"
            );

            struct Processor {
                lines: Vec<super::PyLine>,
            }
            impl Processor {
                fn process(&mut self, block: &mut IndentBlock, last_indent: usize) {
                    // let mut block = block.borrow_mut();
                    let min_indent = block
                        .content
                        .iter()
                        .filter_map(|it| it.as_ref().left().and_then(|line| line.indent))
                        .min()
                        .unwrap_or(0);

                    // strip common indent
                    for content in block.content.iter_mut() {
                        if let Either::Left(line) = content {
                            line.indent = Some(
                                last_indent
                                    + block.indent
                                    + line.indent.map(|i| i - min_indent).unwrap_or(0),
                            );
                        }
                    }

                    let mut last_indent = last_indent + block.indent;
                    for content in block.content.iter_mut() {
                        match content {
                            Either::Left(line) => {
                                let line = super::PyLine {
                                    segments: mem::take(&mut line.segments).into_boxed_slice(),
                                    indent: line.indent.unwrap(),
                                };
                                last_indent = line.indent;
                                self.lines.push(line);
                            }
                            Either::Right(block) => {
                                self.process(&mut block.borrow_mut(), last_indent)
                            }
                        }
                    }
                }
            }

            let mut processor = Processor { lines: Vec::new() };
            processor.process(&mut self.indent_block_stack[0].borrow_mut(), 0);

            let mut lines = Vec::new();
            let mut was_empty_line = false;
            for line in processor.lines {
                let is_empty_line = line.is_empty();
                if !is_empty_line || !was_empty_line {
                    lines.push(line);
                }
                was_empty_line = is_empty_line;
            }

            PySource {
                lines: lines.into_boxed_slice(),
            }
        }
    }
}
