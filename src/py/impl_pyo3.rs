use crate::py::PyMetaExecutionResult;
use crate::py::error::{FrameSummary, PythonError, SourceLocation, StackSummary};
use crate::rust_to_py::py_code_gen::PyMetaExecutable;
use either::Either;
use proc_macro2::TokenStream;
use pyo3::exceptions::PySyntaxError;
use pyo3::ffi::c_str;
use pyo3::prelude::*;
use pyo3::types::{PyCode, PyCodeInput, PyCodeMethods, PyDict, PyTraceback, PyTuple};
use std::ffi::CString;
use std::rc::Rc;

pub(crate) fn execute(exe: PyMetaExecutable) -> PyMetaExecutionResult {
    Python::initialize();
    Python::attach(|py| -> PyResult<()> {
        let sys = py.import("sys")?;
        let sys_modules: Bound<PyDict> = sys.getattr("modules")?.cast_into_exact()?;
        sys_modules.set_item("_pymeta", pyo3::wrap_pymodule!(_pymeta)(py))?;
        PyModule::from_code(py, c_str!(include_str!("pymeta.py")), c"pymeta.py", c"pymeta")?;
        Ok(())
    })
    .unwrap_or_else(|e| panic!("Failed to initialize Python libs: {e:?}"));

    let result = Python::attach(|py| -> PyResult<TokenStream> {
        // compile code
        let module = &exe.main;
        let mut source_code = module.source.source_code().into_bytes();
        source_code.push(0);
        let source_code = CString::from_vec_with_nul(source_code).expect("Python source code can't contain null bytes");
        let code = PyCode::compile(
            py,
            &source_code,
            &CString::new(&module.filename[..]).unwrap(),
            PyCodeInput::File,
        )?;

        // setup context
        let context = PyDict::new(py);
        context
            .set_item(
                "__spans",
                PyTuple::new(py, exe.main.spans.iter().map(|span| _pymeta::PySpan(Rc::clone(span))))
                    .expect("Failed to construct __spans tuple"),
            )
            .expect("Failed to add __spans to context");
        let pymeta = py.import("pymeta").expect("Failed to import pymeta");
        let tokens = pymeta.call_method0("Tokens").expect("Failed to construct Tokens");
        tokens.call_method0("__enter__").expect("Tokens.__enter__() failed");
        py.run(c"import pymeta\nfrom pymeta import *", Some(&context), None)
            .expect("Failed to import pymeta");

        code.run(Some(&context), None)?;

        tokens.call_method0("__exit__").expect("Tokens.__exit__() failed");

        // extract result
        let tokens: Bound<_pymeta::PyTokenStream> = tokens
            .call_method0("_to_tokenstream")
            .expect("Tokens._to_tokenstream() failed")
            .cast_into_exact::<_pymeta::PyTokenStream>()
            .expect("Expected Tokens._to_tokenstream() to return a TokenStream");

        Ok(tokens.borrow_mut().0.take().unwrap())
    })
    .map_err(|err| {
        // report exception
        Python::attach(|py| {
            let exception = err.into_value(py).into_bound(py);

            let trace = if let Ok(traceback) = exception
                .getattr("__traceback__")
                .and_then(|tb| Ok(tb.cast_into_exact::<PyTraceback>()?))
            {
                let traceback_mod = py.import("traceback").expect("Should be able to import `traceback`");
                let mut stack_summary: StackSummary = traceback_mod
                    .call_method1("extract_tb", (traceback,))
                    .expect("traceback.extract_tb()")
                    .try_iter()
                    .expect("traceback.extract_tb() should return an iterable object")
                    .map(|frame| {
                        let frame = frame.unwrap();
                        let filename = frame.getattr("filename").unwrap().extract::<String>().unwrap();
                        let file = exe
                            .find_module_from_filename(&filename)
                            .map(|m| Either::Left(Rc::clone(m)))
                            .unwrap_or(Either::Right(filename));
                        FrameSummary {
                            frame_name: frame.getattr("name").unwrap().extract().unwrap(),
                            location: SourceLocation::new(
                                file,
                                frame.getattr("lineno").unwrap().extract::<usize>().unwrap(),
                                frame.getattr("colno").unwrap().extract::<Option<usize>>().unwrap(),
                                frame.getattr("end_lineno").unwrap().extract::<Option<usize>>().unwrap(),
                                frame
                                    .getattr("end_colno")
                                    .unwrap()
                                    .extract::<Option<usize>>()
                                    .unwrap()
                                    .map(|c| c - 1),
                            ),
                        }
                    })
                    .collect();
                stack_summary.reverse();
                Some(Either::Left(stack_summary))
            } else if let Ok(syntax_error) = exception.cast::<PySyntaxError>() {
                let filename = syntax_error.getattr("filename").unwrap().extract::<String>().unwrap();
                let file = exe
                    .find_module_from_filename(&filename)
                    .map(|m| Either::Left(Rc::clone(m)))
                    .unwrap_or(Either::Right(filename));
                Some(Either::Right(SourceLocation::new(
                    file,
                    syntax_error.getattr("lineno").unwrap().extract::<usize>().unwrap(),
                    syntax_error
                        .getattr("offset")
                        .unwrap()
                        .extract::<Option<usize>>()
                        .unwrap()
                        .map(|c| c - 1),
                    syntax_error
                        .getattr("end_lineno")
                        .unwrap()
                        .extract::<Option<usize>>()
                        .unwrap(),
                    syntax_error
                        .getattr("end_offset")
                        .unwrap()
                        .extract::<Option<usize>>()
                        .unwrap()
                        .map(|c| c - 2),
                )))
            } else {
                None
            };

            PythonError {
                class: exception.get_type().fully_qualified_name().unwrap().extract().unwrap(),
                msg: exception
                    .str()
                    .and_then(|s| s.extract::<String>())
                    .unwrap_or_else(|_| String::from("<failed to format exception>")),
                trace,
            }
        })
    });

    PyMetaExecutionResult { exe, result }
}

#[pymodule]
mod _pymeta {
    use crate::utils::span::CSpan;
    use proc_macro2::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};
    use pyo3::exceptions::PyValueError;
    use pyo3::prelude::*;
    use pyo3::types::{PyBytes, PyFloat, PyInt, PyString};
    use std::ffi::CString;
    use std::iter;
    use std::rc::Rc;
    use unicode_ident::{is_xid_continue, is_xid_start};

    #[pyfunction]
    fn is_ident_start(char: char) -> bool {
        is_xid_start(char) || char == '_'
    }

    #[pyfunction]
    fn is_ident_continue(char: char) -> bool {
        is_xid_continue(char)
    }

    #[pyclass(name = "Span", unsendable)]
    pub(super) struct PySpan(pub Rc<CSpan>);

    #[pymethods]
    impl PySpan {
        fn __repr__(&self) -> String {
            String::from("Span()") // TODO
        }

        #[staticmethod]
        fn call_site() -> Self {
            Self(Rc::new(CSpan::from(Span::call_site())))
        }

        //TODO: more PySpan methods
    }

    #[pyclass(name = "TokenStream", unsendable)]
    pub(super) struct PyTokenStream(pub Option<TokenStream>);

    impl PyTokenStream {
        fn inner_mut(&mut self) -> PyResult<&mut TokenStream> {
            match self.0 {
                Some(ref mut inner) => Ok(inner),
                None => Err(PyValueError::new_err("This TokenStream has already been consumed")),
            }
        }

        fn append_token(&mut self, mut token: TokenTree, span: Option<&mut PySpan>) -> PyResult<()> {
            span.inspect(|s| token.set_span(s.0.inner()));
            self.inner_mut()?.extend(iter::once(token));
            Ok(())
        }
    }

    #[pymethods]
    impl PyTokenStream {
        #[new]
        fn __new__() -> Self {
            Self(Some(TokenStream::new()))
        }

        fn append_group(
            &mut self,
            delimiter: &str,
            tokens: &mut PyTokenStream,
            span: Option<&mut PySpan>,
        ) -> PyResult<()> {
            let delimiter = match delimiter {
                "()" => Delimiter::Parenthesis,
                "[]" => Delimiter::Bracket,
                "{}" => Delimiter::Brace,
                "" => Delimiter::None,
                _ => {
                    return Err(PyValueError::new_err(format!("Invalid delimiter: \"{delimiter}\"")));
                }
            };
            let group = Group::new(
                delimiter,
                tokens
                    .0
                    .take()
                    .ok_or(PyValueError::new_err("The given TokenStream has already been consumed"))?,
            );
            self.append_token(group.into(), span)
        }

        fn append_punct(&mut self, char: char, spacing: &str, span: Option<&mut PySpan>) -> PyResult<()> {
            let spacing = match spacing {
                "alone" => Spacing::Alone,
                "joint" => Spacing::Joint,
                _ => {
                    return Err(PyValueError::new_err(format!("Invalid spacing: \"{spacing}\"")));
                }
            };
            let punct = Punct::new(char, spacing);
            self.append_token(punct.into(), span)
        }

        fn append_ident(&mut self, string: &str, span: Option<&mut PySpan>) -> PyResult<()> {
            let ident = Ident::new(string, span.map(|s| s.0.inner()).unwrap_or_else(Span::call_site));
            self.inner_mut()?.extend(iter::once(ident));
            Ok(())
        }

        fn append_int_literal(
            &mut self,
            value: &Bound<'_, PyInt>,
            suffix: Option<&str>,
            span: Option<&mut PySpan>,
        ) -> PyResult<()> {
            let literal = match suffix {
                Some(suffix) => match suffix {
                    "u8" => Literal::u8_suffixed(value.extract()?),
                    "u16" => Literal::u16_suffixed(value.extract()?),
                    "u32" => Literal::u32_suffixed(value.extract()?),
                    "u64" => Literal::u64_suffixed(value.extract()?),
                    "u128" => Literal::u128_suffixed(value.extract()?),
                    "usize" => Literal::usize_suffixed(value.extract()?),
                    "i8" => Literal::i8_suffixed(value.extract()?),
                    "i16" => Literal::i16_suffixed(value.extract()?),
                    "i32" => Literal::i32_suffixed(value.extract()?),
                    "i64" => Literal::i64_suffixed(value.extract()?),
                    "i128" => Literal::i128_suffixed(value.extract()?),
                    "isize" => Literal::isize_suffixed(value.extract()?),
                    _ => {
                        return Err(PyValueError::new_err(format!(
                            "Invalid int literal suffix: \"{suffix}\""
                        )));
                    }
                },
                None => {
                    if let Ok(value) = value.extract::<i128>() {
                        Literal::i128_unsuffixed(value)
                    } else if let Ok(value) = value.extract::<u128>() {
                        Literal::u128_unsuffixed(value)
                    } else {
                        return Err(PyValueError::new_err(format!("Int literal value overflow: {value}")));
                    }
                }
            };
            self.append_token(literal.into(), span)
        }

        fn append_float_literal(
            &mut self,
            value: Bound<'_, PyFloat>,
            suffix: Option<&str>,
            span: Option<&mut PySpan>,
        ) -> PyResult<()> {
            let literal = match suffix {
                Some("f32") => {
                    let value = value.extract::<f32>()?;
                    if !value.is_finite() {
                        return Err(PyValueError::new_err(format!(
                            "Invalid float literal value: {value}f32"
                        )));
                    }
                    Literal::f32_suffixed(value)
                }
                Some("f64") => {
                    let value = value.extract::<f64>()?;
                    if !value.is_finite() {
                        return Err(PyValueError::new_err(format!(
                            "Invalid float literal value: {value}f64"
                        )));
                    }
                    Literal::f64_suffixed(value)
                }
                Some(suffix) => {
                    return Err(PyValueError::new_err(format!(
                        "Invalid float literal suffix: \"{suffix}\""
                    )));
                }
                None => {
                    let value = value.extract::<f64>()?;
                    if !value.is_finite() {
                        return Err(PyValueError::new_err(format!("Invalid float literal value: {value}")));
                    }
                    Literal::f64_unsuffixed(value)
                }
            };
            self.append_token(literal.into(), span)
        }

        fn append_str_literal(
            &mut self,
            r#type: &str,
            value: Bound<'_, PyString>,
            span: Option<&mut PySpan>,
        ) -> PyResult<()> {
            let literal = match r#type {
                "str" => Literal::string(value.to_str()?),
                "chr" => Literal::character(value.extract::<char>()?),
                _ => {
                    return Err(PyValueError::new_err(format!("Invalid str literal type: \"{type}\"")));
                }
            };
            self.append_token(literal.into(), span)
        }

        fn append_bytes_literal(
            &mut self,
            r#type: &str,
            value: Bound<'_, PyBytes>,
            span: Option<&mut PySpan>,
        ) -> PyResult<()> {
            let literal = match r#type {
                "bytes" => Literal::byte_string(value.as_bytes()),
                "byte" => {
                    let bytes = value.as_bytes();
                    if bytes.len() != 1 {
                        return Err(PyValueError::new_err(format!(
                            "Expect one byte, got {} bytes",
                            bytes.len()
                        )));
                    }
                    Literal::byte_character(bytes[0])
                }
                "cstr" => Literal::c_string(
                    &CString::new(value.as_bytes())
                        .map_err(|e| PyValueError::new_err(format!("Invalid c string bytes: {e:?}")))?,
                ),
                _ => {
                    return Err(PyValueError::new_err(format!("Invalid bytes literal type: {}", r#type)));
                }
            };
            self.append_token(literal.into(), span)
        }
    }
}
