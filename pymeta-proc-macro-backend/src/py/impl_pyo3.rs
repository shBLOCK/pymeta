#![allow(non_snake_case)]

use crate::py::PyMetaExecutionResult;
use crate::py::error::{FrameSummary, PythonError, SourceLocation, StackSummary};
use crate::rust_to_py::PY_GLOBAL_OBJS_ARRAY_NAME;
use crate::rust_to_py::meta::stmt::ImportMetaStmt;
use crate::rust_to_py::py_code_gen::{PyMetaExecutable, PyObj};
use crate::utils::span::CSpan;
use either::Either;
use proc_macro2::TokenStream;
use pyo3::IntoPyObjectExt;
use pyo3::exceptions::PySyntaxError;
use pyo3::ffi::c_str;
use pyo3::prelude::*;
use pyo3::types::{PyCode, PyCodeInput, PyCodeMethods, PyDict, PyTraceback, PyTuple};
use std::ffi::CString;
use std::rc::Rc;
use std::sync::OnceLock;

macro_rules! include_cstr {
    ($path:expr) => {
        c_str!(include_str!($path))
    };
}

macro_rules! pylib_path {
    ($path:expr) => {
        concat!(env!("CARGO_MANIFEST_DIR"), "/pylib/src/", $path)
    };
}

macro_rules! importer_files_dict {
    ($py:ident, $dir:expr, {$($key:literal $({$($item:tt)*})?),*}) => {
        {
            let dict = PyDict::new($py);
            $(dict.set_item($key, _importer_files_dict_item!($py, $dir, $key $({$($item)*})?)).unwrap();)*
            dict
        }
    };
}

macro_rules! _importer_files_dict_item {
    ($py:ident, $dir:expr, $key:literal) => {
        include_cstr!(concat!($dir, $key, ".py"))
    };
    ($py:ident, $dir:expr, $key:literal {$($item:tt)*}) => {
        importer_files_dict!($py, concat!($dir, $key, "/"), {$($item)*})
    };
}

fn initialize() {
    static INITIALIZED: OnceLock<()> = OnceLock::new();
    INITIALIZED.get_or_init(|| {
        #[cfg(all(target_os = "linux", feature = "proc_macro", feature = "linux_force_load_python_lib"))]
        unsafe {
            let libpython = c_str!(concat!("lib", env!("PYMETA_LIBPYTHON_NAME"), ".so"));
            libc::dlopen(libpython.as_ptr(), libc::RTLD_NOW | libc::RTLD_GLOBAL);
        }

        Python::initialize();
        Python::attach(|py| {
            let sys = py.import("sys").unwrap();

            sys.getattr("modules")
                .unwrap()
                .set_item("pymeta._.native", pyo3::wrap_pymodule!(pymeta_native)(py))
                .unwrap();

            {
                // register PyMetaBuiltinsImporter
                let globals = PyDict::new(py);
                py.run(include_cstr!(pylib_path!("init.py")), Some(&globals), None)
                    .expect("Failed to run init.py");
                let PyMetaBuiltinsImporter = globals
                    .get_item("PyMetaBuiltinsImporter")
                    .unwrap()
                    .expect("No PyMetaBuiltinsImporter in init.py");

                let builtin_files = importer_files_dict!(py, pylib_path!(""), {
                    "pymeta" {
                        "__init__",
                        "_" {
                            "__init__",
                            "module"
                        }
                    }
                });
                let builtins_importer = PyMetaBuiltinsImporter
                    .call1((builtin_files,))
                    .expect("Failed to create PyMetaBuiltinsImporter");

                sys.getattr("meta_path")
                    .unwrap()
                    .call_method1("append", (builtins_importer,))
                    .unwrap();
            }
        });
    });
}

pub(crate) fn execute(exe: PyMetaExecutable) -> PyMetaExecutionResult {
    initialize();

    let result = Python::attach(|py| -> PyResult<TokenStream> {
        let builtins = py.import("builtins").unwrap();

        // compile code
        let module = &exe.main;
        let mut source_code = module.source.source_code().into_bytes();
        source_code.push(0);
        let source_code = CString::from_vec_with_nul(source_code).expect("Python source code can't contain null bytes");
        let code = PyCode::compile(
            py,
            &source_code,
            &CString::new(module.filename.as_str()).unwrap(),
            PyCodeInput::File,
        )?;

        // register `pymeta_module`s
        let PyMetaModuleImporter = py
            .import("pymeta._.module")
            .unwrap()
            .getattr("PyMetaModuleImporter")
            .unwrap();
        PyMetaModuleImporter
            .call_method0("kill_all")
            .expect("PyMetaModuleImporter.kill_all() failed");
        let pymeta_module_importer = PyMetaModuleImporter
            .call1((ImportMetaStmt::PATH, {
                let modules_dict = PyDict::new(py);
                for module in &exe.modules {
                    modules_dict
                        .set_item(&module.name, module.source.source_code())
                        .unwrap();
                }
                modules_dict
            }))
            .expect("PyMetaModuleImporter() failed");

        // setup context
        builtins
            .setattr(
                PY_GLOBAL_OBJS_ARRAY_NAME,
                PyTuple::new(py, exe.objs.iter()).expect("Failed to construct global objs array"),
            )
            .unwrap();
        let context = PyDict::new(py);
        let pymeta = py.import("pymeta").expect("Failed to import pymeta");
        let tokens = pymeta.call_method0("Tokens").expect("Failed to construct Tokens");
        tokens.call_method0("__enter__").expect("Tokens.__enter__() failed");
        py.run(c"import pymeta\nfrom pymeta import *", Some(&context), None)
            .expect("Failed to import pymeta");

        let result = code.run(Some(&context), None);

        // cleanup
        builtins.delattr(PY_GLOBAL_OBJS_ARRAY_NAME).unwrap();
        pymeta_module_importer
            .call_method0("kill")
            .expect("PyMetaModuleImporter.kill() failed");
        tokens.call_method0("__exit__").expect("Tokens.__exit__() failed");

        result?;

        // extract result
        let tokens: Bound<pymeta_native::PyTokenStream> = tokens
            .call_method0("_to_tokenstream")
            .expect("Tokens._to_tokenstream() failed")
            .cast_into_exact::<pymeta_native::PyTokenStream>()
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

impl<'py> IntoPyObject<'py> for &PyObj {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        match self {
            PyObj::Span(span) => Ok(pymeta_native::PySpan(Rc::clone(span)).into_bound_py_any(py)?),
        }
    }
}

impl<'py> IntoPyObject<'py> for CSpan {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        pymeta_native::PySpan(Rc::new(self)).into_bound_py_any(py)
    }
}

#[pymodule]
mod pymeta_native {
    use crate::utils::span::CSpan;
    use proc_macro2::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};
    use pyo3::exceptions::{PyKeyError, PyRuntimeError, PyValueError};
    use pyo3::prelude::*;
    use pyo3::types::{PyBytes, PyFloat, PyInt, PyString};
    use std::ffi::CString;
    use std::iter;
    use std::path::{Path, PathBuf};
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
            format!("{:?}", self.0.inner())
        }

        #[staticmethod]
        fn call_site() -> CSpan {
            Span::call_site().into()
        }

        #[staticmethod]
        fn mixed_site() -> CSpan {
            Span::mixed_site().into()
        }

        fn start(&self) -> CSpan {
            self.0.start_span()
        }

        fn end(&self) -> CSpan {
            self.0.end_span()
        }

        fn line(&self) -> usize {
            cfg_select! {
                feature = "proc_macro" => self.0.inner().unwrap().line(),
                _ => self.0.inner().start().line,
            }
        }

        fn column(&self) -> usize {
            cfg_select! {
                feature = "proc_macro" => self.0.inner().unwrap().column(),
                _ => self.0.inner().start().column + 1,
            }
        }

        fn file(&self) -> String {
            self.0.inner().file()
        }

        fn local_file(&self) -> Option<PathBuf> {
            self.0.inner().local_file()
        }

        fn resolved_at(&self, other: &Self) -> CSpan {
            self.0.inner().resolved_at(other.0.inner()).into()
        }

        fn located_at(&self, other: &Self) -> CSpan {
            self.0.inner().located_at(other.0.inner()).into()
        }

        fn source_text(&self) -> Option<String> {
            self.0.inner().source_text()
        }

        //TODO: more PySpan methods (e.g. join) when they stabilize
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

    #[cfg(feature = "proc_macro")]
    extern crate proc_macro;

    #[pyfunction]
    fn tracked_path(path: PathBuf) -> PyResult<()> {
        cfg_select! {
            feature = "nightly_tracked" => {
                #[cfg(feature = "proc_macro")]
                proc_macro::tracked::path(path);
                Ok(())
            }
            _ => Err(PyRuntimeError::new_err("PyMeta: the `nightly_tracked` feature has to be enabled to use this function")),
        }
    }

    #[pyfunction]
    fn tracked_env_var(key: &str) -> PyResult<String> {
        use std::env::VarError;
        cfg_select! {
            feature = "nightly_tracked" => {
                cfg_select! {
                    feature = "proc_macro" => proc_macro::tracked::env_var(key),
                    _ => std::env::var(key),
                }.map_err(|e| {
                    match e {
                        VarError::NotPresent => PyKeyError::new_err(String::from(key)),
                        VarError::NotUnicode(_) => PyValueError::new_err(format!("the value of env var `{key}` is not valid Unicode")),
                    }
                })
            }
            _ => Err(PyRuntimeError::new_err("PyMeta: the `nightly_tracked` feature has to be enabled to use this function")),
        }
    }
}
