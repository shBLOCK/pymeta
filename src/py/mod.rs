use crate::rust_to_py::py_code_gen::PyMetaExecutable;
use proc_macro2::TokenStream;
use error::PythonError;

//TODO: Python backend feature flags
pub(crate) mod impl_pyo3;
pub(crate) mod impl_rustpython;
mod error;

pub(crate) struct PyMetaExecutionResult {
    pub exe: PyMetaExecutable,
    pub result: Result<TokenStream, PythonError>,
}
