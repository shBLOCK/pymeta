pub(crate) mod code_region;
pub(crate) mod py_code_gen;
pub(crate) mod py_source;
pub(crate) mod meta;
pub(crate) mod utils;

pub(crate) const PY_MARKER: char = '$';

/// Used when [PY_MARKER]s can't be used (e.g. in macro definition)
pub(crate) const PY_MARKER_IDENT: &str = "__PYM__";

pub(crate) const CONCAT_MARKER: char = '~';

/// Used to pass values to Python.
/// 
/// This name will be everywhere in generated Python code, so is have to be an obscure name.
pub(crate) const PY_GLOBAL_OBJS_ARRAY_NAME: &str = PY_MARKER_IDENT;
