use crate::rust_to_py::PY_MARKER;
use crate::rust_to_py::py_code_gen::PyMetaModule;
use crate::rust_to_py::py_source::PySrcSegment;
use crate::utils::diagnostic::{Diagnostic, DiagnosticLevel};
use crate::utils::span::SpanEx;
use either::Either;
use proc_macro2::Span;
use std::cell::OnceCell;
use std::fmt::Write;
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

    pub fn segments(&self) -> Option<Vec<Rc<PySrcSegment>>> {
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

    pub fn find_preceding_segment(&self) -> Option<Rc<PySrcSegment>> {
        let start_column = self.start_column?;
        if let Either::Left(ref module) = self.file {
            for lineno in (1..=self.start_line).rev() {
                if let Some(line) = module.source.lines.get(lineno - 1) {
                    let mut column = line.indent + line.segments.iter().map(|s| s.code.chars().count()).sum::<usize>();
                    for segment in line.segments.iter().rev() {
                        let seg_end = column;

                        if (lineno < self.start_line || seg_end <= start_column)
                            && !segment.code.chars().all(char::is_whitespace)
                        {
                            return Some(Rc::clone(segment));
                        }

                        column -= segment.code.chars().count();
                    }
                }
            }
            None
        } else {
            None
        }
    }

    pub fn src_span(&self) -> Option<Span> {
        *self.src_span_cache.get_or_init(|| {
            if let Either::Left(ref _module) = self.file {
                let segments = self.segments().unwrap();
                if !segments.is_empty() {
                    PySrcSegment::join_src_spans(segments.iter().map(Rc::deref))
                } else {
                    self.find_preceding_segment()
                        .and_then(|seg| seg.src_span.as_ref().map(|span| span.inner().end_span()))
                }
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

impl PythonError {
    pub fn emit_diagnostics(&self) {
        let err_text = format!("{}{}: {}", PY_MARKER, self.class, self.msg);

        match &self.trace {
            Some(Either::Left(stack_summary)) => {
                assert!(!stack_summary.is_empty());
                let mut diagnostic = Diagnostic::new(
                    stack_summary[0].location.src_span().unwrap_or(Span::call_site()),
                    DiagnosticLevel::Error,
                    err_text,
                );

                diagnostic = diagnostic.add_note(None, "Traceback (most recent call first):");
                let mut last_frame = None;
                const REPEAT_CUTOFF: u32 = 3;
                let mut repeating_frames = 0u32;
                let mut stack_iter = stack_summary.iter().peekable();
                while let Some(frame) = stack_iter.next() {
                    if Some(frame) == last_frame {
                        repeating_frames += 1;
                        if repeating_frames == REPEAT_CUTOFF {
                            while stack_iter.peek() == last_frame.as_ref() {
                                repeating_frames += 1;
                                stack_iter.next().unwrap();
                            }
                            let not_shown_repeats = repeating_frames + 1 - REPEAT_CUTOFF;
                            diagnostic = diagnostic.add_note(
                                None,
                                format!(
                                    "|  [Previous line repeated {not_shown_repeats} more time{}]",
                                    if not_shown_repeats > 1 { "s" } else { "" }
                                ),
                            );
                            continue;
                        }
                    } else {
                        repeating_frames = 0;
                    }
                    last_frame = Some(frame);

                    let filename = frame.location.file.as_ref().map_left(|m| &m.filename).into_inner();
                    let mut text = format!("|  File \"{filename}\", line {}", frame.location.start_line);
                    if let Some(src_span) = frame.location.src_span() {
                        write!(text, " (Rust line {})", src_span.start().line).unwrap();
                    }
                    write!(text, ", in {}", frame.frame_name).unwrap();

                    if let Some(src_span) = frame.location.src_span() {
                        diagnostic = diagnostic.add_note(src_span, text);
                    } else {
                        diagnostic = diagnostic.add_note(None, text);
                    }
                }

                // let module = exe_result.exe.main;
                // diagnostic = diagnostic.span_note(proc_macro::Span::call_site().start(), format!(
                //     "PyMeta source dump of \"{filename}\":\n{dump}",
                //     filename = module.filename,
                //     dump = module.source.diagnostic_source_dump()
                // ));

                diagnostic.emit();
            }
            Some(Either::Right(location)) => {
                let diagnostic = Diagnostic::new(
                    location.src_span().unwrap_or(Span::call_site()),
                    DiagnosticLevel::Error,
                    err_text,
                );
                #[allow(unused_mut)]
                let mut location_msg = format!(
                    "File \"{file}\", line {line}",
                    file = location.file.as_ref().map_left(|m| &m.filename),
                    line = location.start_line,
                );
                if let Some(src_span) = location.src_span() {
                    write!(location_msg, " (Rust line {})", src_span.start().line).unwrap();
                }
                diagnostic.add_note(None, location_msg).emit();
            }
            None => Diagnostic::new(Span::call_site(), DiagnosticLevel::Error, err_text).emit(),
        }
    }
}
