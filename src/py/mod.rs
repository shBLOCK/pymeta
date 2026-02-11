use proc_macro2::TokenStream;
use crate::utils::py_source::PySource;

//TODO: Python backend feature flags
pub(crate) mod impl_pyo3;
pub(crate) mod impl_rustpython;

#[derive(Debug)]
pub(crate) struct PyMetaExecutionError {
    pub tmp_string: String, // TODO: better Python error reporting
}

pub(crate) struct PyMetaExecutionResult {
    pub source: PySource,
    pub result: Result<TokenStream, PyMetaExecutionError>,
}
