use std::fmt::Write;
use crate::rust_to_py::PY_MARKER;
use crate::rust_to_py::py_code_gen::PyMetaModule;
use crate::rust_to_py::py_source::PySrcSegment;
use either::Either;
use proc_macro::{Diagnostic, Level as DiagnosticLevel};
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

    pub fn src_span(&self) -> Option<Span> {
        *self.src_span_cache.get_or_init(|| {
            if let Either::Left(ref _module) = self.file {
                PySrcSegment::join_src_spans(self.segments().unwrap().iter().map(Rc::deref))
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
                let mut diagnostic = Diagnostic::spanned(
                    stack_summary[0]
                        .location
                        .src_span()
                        .unwrap_or(Span::call_site())
                        .unwrap(),
                    DiagnosticLevel::Error,
                    err_text,
                );

                diagnostic = diagnostic.note("Traceback (most recent call first):");
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
                            diagnostic = diagnostic.note(format!(
                                "|  [Previous line repeated {not_shown_repeats} more time{}]",
                                if not_shown_repeats > 1 { "s" } else { "" }
                            ));
                            continue;
                        }
                    } else {
                        repeating_frames = 0;
                    }
                    last_frame = Some(frame);

                    let filename = frame.location.file.as_ref().map_left(|m| &m.filename).into_inner();
                    let mut text = format!("|  File \"{filename}\", line {}", frame.location.start_line);
                    #[cfg(feature = "nightly_proc_macro_span")]
                    if let Some(src_span) = frame.location.src_span() {
                        write!(text, " (Rust line {})", src_span.start().line).unwrap();
                    }
                    write!(text, ", in {}", frame.frame_name).unwrap();

                    #[cfg(feature = "nightly_proc_macro_span")]
                    if let Some(src_span) = frame.location.src_span() {
                        diagnostic = diagnostic.span_note(src_span.unwrap(), text);
                    } else {
                        diagnostic = diagnostic.note(text);
                    }

                    #[cfg(not(feature = "nightly_proc_macro_span"))]
                    {
                        diagnostic = diagnostic.note(text);
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
                let diagnostic = Diagnostic::spanned(
                    location.src_span().unwrap_or(Span::call_site()).unwrap(),
                    DiagnosticLevel::Error,
                    err_text,
                );
                #[allow(unused_mut)]
                let mut location_msg = format!(
                    "File \"{file}\", line {line}",
                    file = location.file.as_ref().map_left(|m| &m.filename),
                    line = location.start_line,
                );
                #[cfg(feature = "nightly_proc_macro_span")]
                if let Some(src_span) = location.src_span() {
                    write!(location_msg, " (Rust line {})", src_span.start().line).unwrap();
                }
                diagnostic.note(location_msg).emit();
            }
            None => Diagnostic::spanned(proc_macro::Span::call_site(), DiagnosticLevel::Error, err_text).emit(),
        }
    }
}
