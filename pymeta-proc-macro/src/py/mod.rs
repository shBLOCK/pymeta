use crate::rust_to_py::py_code_gen::PyMetaExecutable;
use error::PythonError;
use proc_macro2::TokenStream;

//TODO: Python backend feature flags
mod error;
pub(crate) mod impl_pyo3;
pub(crate) mod impl_rustpython;

pub(crate) struct PyMetaExecutionResult {
    pub exe: PyMetaExecutable,
    pub result: Result<TokenStream, PythonError>,
}
