use crate::rust_to_py::py_code_gen::PyMetaModule;
use crate::utils::py_source::PySegment;
use either::Either;
use proc_macro2::Span;
use std::cell::OnceCell;
use std::ops::Deref;
use std::rc::Rc;

#[derive(Debug)]
pub(crate) struct SourceLocation {
    pub file: Either<Rc<PyMetaModule>, String>,
    pub start_line: usize,
    pub start_column: Option<usize>,
    pub end_line: Option<usize>,
    pub end_column: Option<usize>,
    src_span_cache: OnceCell<Option<Span>>,
}

impl SourceLocation {
    pub fn new(
        file: Either<Rc<PyMetaModule>, String>,
        start_line: usize,
        start_column: Option<usize>,
        end_line: Option<usize>,
        end_column: Option<usize>,
    ) -> Self {
        Self {
            file,
            start_line,
            start_column,
            end_line,
            end_column,
            src_span_cache: OnceCell::new(),
        }
    }

    pub fn segments(&self) -> Option<Vec<Rc<PySegment>>> {
        if let Either::Left(ref module) = self.file {
            let mut segments = Vec::new();
            let end_line = self.end_line.unwrap_or(self.start_line);
            for lineno in self.start_line..=end_line {
                let Some(line) = module.source.lines.get(lineno - 1) else {
                    break;
                };
                let mut column = line.indent;
                for segment in &line.segments {
                    let seg_start = column;
                    column += segment.code.chars().count();
                    let seg_end = column - 1;
                    if lineno == self.start_line
                        && let Some(start_column) = self.start_column
                        && seg_end < start_column
                    {
                        continue;
                    }
                    if lineno == end_line
                        && let Some(end_column) = self.end_column
                        && seg_start > end_column
                    {
                        continue;
                    }
                    segments.push(Rc::clone(segment));
                }
            }
            Some(segments)
        } else {
            None
        }
    }

    pub fn src_span(&self) -> Option<Span> {
        *self.src_span_cache.get_or_init(|| {
            if let Either::Left(ref _module) = self.file {
                PySegment::join_src_spans(self.segments().unwrap().iter().map(Rc::deref))
            } else {
                None
            }
        })
    }
}

impl PartialEq for SourceLocation {
    fn eq(&self, other: &Self) -> bool {
        (match (&self.file, &other.file) {
            (Either::Left(ma), Either::Left(mb)) => Rc::ptr_eq(ma, mb),
            (Either::Right(fa), Either::Right(fb)) => fa == fb,
            _ => false,
        }) && self.start_line == other.start_line
            && self.end_line == other.end_line
            && self.start_column == other.start_column
            && self.end_column == other.end_column
    }
}

impl Eq for SourceLocation {}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct FrameSummary {
    pub frame_name: String,
    pub location: SourceLocation,
}

/// Most recent call **first**. (Inverse of Python default)
pub(crate) type StackSummary = Box<[FrameSummary]>;

#[derive(Debug)]
pub(crate) struct PythonError {
    pub class: String,
    pub msg: String,
    /// This is [SourceLocation] for `SyntaxError`s and [StackSummary] for other (runtime) errors,
    /// or [None] if trace information can not be extracted.
    pub trace: Option<Either<StackSummary, SourceLocation>>,
    //TODO: exception chaining (__context__ & __cause__) & ExceptionGroup
}
