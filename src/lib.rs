#![feature(proc_macro_diagnostic)]
#![cfg_attr(feature = "nightly_proc_macro_span", feature(proc_macro_span))]

extern crate proc_macro;
use std::fmt::Write;

use crate::rust_to_py::code_regions::parser::CodeRegionParser;
use crate::rust_to_py::py_code_gen::{PyCodeGen, PyMetaExecutable};
use crate::rust_to_py::PY_MARKER;
use either::Either;
use proc_macro::{Diagnostic, Level as DiagnosticLevel};
use proc_macro2::{Span, TokenStream};
use std::rc::Rc;
use utils::rust_token::TokenBuffer;

mod py;
mod rust_to_py;
mod utils;

#[proc_macro]
#[proc_macro_error2::proc_macro_error]
pub fn pymeta(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    #[cfg(feature = "debug_log")]
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .format_timestamp(None)
        .format_module_path(false)
        .format_source_path(true)
        .try_init();

    let input = TokenBuffer::from_iter(TokenStream::from(input));
    let code_regions = CodeRegionParser::new().parse(input);
    let main = {
        let mut codegen = PyCodeGen::new();
        codegen.append_code_regions(code_regions.iter());
        codegen.finish(String::from("<PyMeta main>"))
    };

    let exe_result = py::impl_pyo3::execute(PyMetaExecutable {
        main: Rc::new(main),
    });

    if let Err(ref error) = exe_result.result {
        let err_text = format!("{}{}: {}", PY_MARKER, error.class, error.msg);

        match &error.trace {
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

                    let filename = frame
                        .location
                        .file
                        .as_ref()
                        .map_left(|m| &m.filename)
                        .into_inner();
                    if let Some(src_span) = frame.location.src_span() {
                        diagnostic = diagnostic.span_note(
                            src_span.unwrap(),
                            format!(
                                "|  File \"{filename}\", line {} (Rust line {}), in {}",
                                frame.location.start_line,
                                src_span.start().line,
                                frame.frame_name
                            ),
                        );
                    } else {
                        diagnostic = diagnostic.note(format!(
                            "|  File \"{filename}\", line {}, in {}",
                            frame.location.start_line, frame.frame_name
                        ));
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
                let mut location_msg = format!(
                    "File \"{file}\", line {line}",
                    file = location.file.as_ref().map_left(|m| &m.filename),
                    line = location.start_line,
                );
                if let Some(src_span) = location.src_span() {
                    write!(location_msg, " (Rust line {})", src_span.start().line).unwrap();
                }
                diagnostic.note(location_msg).emit();
            }
            None => Diagnostic::spanned(
                proc_macro::Span::call_site(),
                DiagnosticLevel::Error,
                err_text,
            )
            .emit(),
        }

        {
            // emit source dumps
            let module = exe_result.exe.main;
            Diagnostic::spanned(
                proc_macro::Span::call_site(),
                DiagnosticLevel::Warning,
                format!(
                    "PyMeta source dump of \"{filename}\":\n{dump}",
                    filename = module.filename,
                    dump = module.source.diagnostic_source_dump()
                ),
            )
            .emit();
        }
    }

    exe_result.result.unwrap_or_else(|_| TokenStream::new()).into()
}
