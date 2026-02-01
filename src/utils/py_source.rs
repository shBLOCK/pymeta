use proc_macro2::Span;

const INDENT_SIZE: u8 = 4;

pub(crate) enum PySegment {
    Code {
        code: String,
        src_span: Option<Span>,
    },
    Spaces(u16),
}

impl PySegment {
    pub fn code(code: String, src_span: Option<Span>) -> PySegment {
        PySegment::Code { code, src_span }
    }

    pub fn spaces(n: u16) -> PySegment {
        PySegment::Spaces(n)
    }
}

struct PyLine {
    segments: Vec<PySegment>,
    indent: u16,
}

impl PyLine {
    fn new(indent: u16) -> Self {
        Self {
            segments: Vec::new(),
            indent,
        }
    }

    fn append(&mut self, segment: PySegment) {
        self.segments.push(segment);
    }

    fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }
}

pub(crate) struct PySource {
    lines: Vec<PyLine>,
}

impl PySource {
    pub fn new() -> Self {
        Self { lines: Vec::new() }
    }

    fn new_line(&mut self, indent: u16) {
        self.lines.push(PyLine::new(indent));
    }

    fn append(&mut self, segment: PySegment) {
        self.lines
            .last_mut()
            .expect("Can't append to an empty PySourceCode")
            .append(segment);
    }

    pub fn indent_all(&mut self, n: i16) {
        for line in &mut self.lines {
            if line.is_empty() {
                continue;
            }
            line.indent = (line.indent as i32 + n as i32)
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
        self.indent_all(-(min as i16));
    }
}

pub(crate) struct PySourceBuilder {
    indent_block_stack: Vec<PySource>,
}

impl PySourceBuilder {
    pub fn new() -> Self {
        Self {
            indent_block_stack: vec![PySource::new()],
        }
    }

    pub fn new_line(&mut self, indent: u16) {
        self.indent_block_stack.last_mut().unwrap().new_line(indent);
    }

    pub fn append(&mut self, segment: PySegment) {
        self.indent_block_stack.last_mut().unwrap().append(segment);
    }

    pub fn push_indent_block(&mut self) {
        self.indent_block_stack.push(PySource::new());
    }

    pub fn pop_indent_block(&mut self) {
        assert!(
            self.indent_block_stack.len() > 1,
            "push_indent / pop_indent mismatch, root indent block can't be popped."
        );

        let mut block = self.indent_block_stack.pop().unwrap();
        let top = self.indent_block_stack.last_mut().unwrap();

        block.strip_common_indent();
        let prev_line_indent = top.lines.last().map(|line| line.indent).unwrap_or(0);
        block.indent_all(prev_line_indent as i16 + INDENT_SIZE as i16);

        top.lines.extend(block.lines);
    }

    pub fn finish(mut self) -> PySource {
        assert_eq!(
            self.indent_block_stack.len(),
            1,
            "push_indent / pop_indent mismatch."
        );
        self.indent_block_stack.pop().unwrap()
    }
}
