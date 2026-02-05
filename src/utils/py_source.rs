use crate::utils::rust_token::CSpan;
use std::borrow::Cow;
use std::mem;
use std::rc::Rc;

#[derive(Debug)]
pub(crate) enum PySegment {
    Code {
        code: Cow<'static, str>,
        src_span: Option<Rc<CSpan>>,
    },
    Spaces(usize),
}

impl PySegment {
    pub fn code(code: impl Into<Cow<'static, str>>, src_span: Option<Rc<CSpan>>) -> PySegment {
        Self::Code {
            code: code.into(),
            src_span,
        }
    }

    pub fn spaces(n: usize) -> Self {
        Self::Spaces(n)
    }

    pub fn add_to_string(&self, string: &mut String) {
        match self {
            Self::Code { code, .. } => string.push_str(code),
            Self::Spaces(n) => (0..*n).for_each(|_| string.push(' ')),
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            Self::Code { code, .. } => code.is_empty(),
            Self::Spaces(_) => true,
        }
    }
}

#[derive(Debug)]
struct PyLine {
    segments: Vec<PySegment>,
    indent: usize,
}

impl PyLine {
    fn new(indent: usize) -> Self {
        Self {
            segments: Vec::new(),
            indent,
        }
    }

    fn append(&mut self, segment: PySegment) {
        self.segments.push(segment);
    }

    fn is_empty(&self) -> bool {
        self.segments.iter().all(PySegment::is_empty)
    }

    fn add_to_string(&self, string: &mut String) {
        (0..self.indent).for_each(|_| string.push(' '));
        self.segments
            .iter()
            .for_each(|segment| segment.add_to_string(string));
        string.push('\n');
    }
}

#[derive(Debug)]
pub(crate) struct PySource {
    lines: Vec<PyLine>,
}

impl PySource {
    pub fn new() -> Self {
        Self { lines: Vec::new() }
    }

    fn new_line(&mut self, indent: usize) {
        self.lines.push(PyLine::new(indent));
    }

    fn append(&mut self, segment: PySegment) {
        self.lines
            .last_mut()
            .expect("Can't append to an empty PySourceCode")
            .append(segment);
    }

    pub fn indent_all(&mut self, n: isize) {
        for line in &mut self.lines {
            if line.is_empty() {
                continue;
            }
            line.indent = (line.indent as isize + n)
                .try_into()
                .unwrap_or_else(|_| panic!("Indent overflow, adding {n} to {}.", line.indent));
        }
    }

    pub fn strip_common_indent(&mut self) {
        let Some(min) = self
            .lines
            .iter()
            .filter(|line| !line.is_empty())
            .map(|line| line.indent)
            .min()
        else {
            return;
        };
        self.indent_all(-(min as isize));
    }

    pub fn strip_repeating_empty_lines(&mut self) {
        let old_lines = mem::replace(&mut self.lines, Vec::new());
        let mut old_lines = old_lines.into_iter().peekable();
        while let Some(line) = old_lines.next() {
            if line.is_empty() {
                while old_lines.peek().map(PyLine::is_empty).unwrap_or(false) {
                    old_lines.next();
                }
            }
            self.lines.push(line);
        }
    }

    pub fn source_code(&self) -> String {
        let mut string = String::new();
        self.lines
            .iter()
            .for_each(|line| line.add_to_string(&mut string));
        string
    }
}

struct IndentBlock {
    py: PySource,
    indent: usize,
}

pub(crate) struct PySourceBuilder {
    indent_block_stack: Vec<IndentBlock>,
}

impl PySourceBuilder {
    pub fn new() -> Self {
        Self {
            indent_block_stack: vec![IndentBlock {
                py: PySource::new(),
                indent: 0,
            }],
        }
    }

    pub fn new_line(&mut self, indent: usize) {
        self.indent_block_stack
            .last_mut()
            .unwrap()
            .py
            .new_line(indent);
    }

    pub fn append(&mut self, segment: PySegment) {
        self.indent_block_stack
            .last_mut()
            .unwrap()
            .py
            .append(segment);
    }

    pub fn push_indent_block(&mut self, indent: usize) {
        self.indent_block_stack.push(IndentBlock {
            py: PySource::new(),
            indent,
        });
    }

    pub fn pop_indent_block(&mut self) {
        assert!(
            self.indent_block_stack.len() > 1,
            "push_indent / pop_indent mismatch, root indent block can't be popped."
        );

        let mut block = self.indent_block_stack.pop().unwrap();
        let top = self.indent_block_stack.last_mut().unwrap();

        block.py.strip_common_indent();
        let prev_line_indent = top.py.lines.last().map_or(0, |line| line.indent);
        block
            .py
            .indent_all(prev_line_indent as isize + block.indent as isize);

        top.py.lines.extend(block.py.lines);
    }

    pub fn finish(mut self) -> PySource {
        assert_eq!(
            self.indent_block_stack.len(),
            1,
            "push_indent / pop_indent mismatch."
        );
        let mut code = self.indent_block_stack.pop().unwrap().py;
        code.strip_common_indent();
        code.strip_repeating_empty_lines();
        code
    }
}
