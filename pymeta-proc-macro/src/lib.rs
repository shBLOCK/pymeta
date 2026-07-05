#![feature(proc_macro_diagnostic)]
#![cfg_attr(feature = "nightly_proc_macro_span", feature(proc_macro_span))]

extern crate proc_macro;

use crate::rust_to_py::code_region::parser::{CodeRegionParser, CodeRegionParserCtx};
use crate::rust_to_py::py_code_gen::{PyCodeGen, PyMetaExecutable};
use proc_macro2::TokenStream;
use std::rc::Rc;
use utils::rust_token::TokenBuffer;

mod py;
mod rust_to_py;
mod utils;

/// This is the final macro call that will actually execute the Python code.
/// It's expected that all `import!`d modules have been included when calling this macro.
#[proc_macro]
#[proc_macro_error3::proc_macro_error]
pub fn _pymeta_main(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // let input = TokenBuffer::from_iter(TokenStream::from(input));
    // while !input.exhausted() {
    //     input.seek()
    // }

    todo!()
}

/// TODO: detailed documentation will be available here,
/// for now you can check out the examples in the crate's top-level documentation
#[proc_macro]
#[proc_macro_error3::proc_macro_error]
pub fn pymeta(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = TokenBuffer::from_iter(TokenStream::from(input));
    let mut code_region_parser_ctx = CodeRegionParserCtx::new();
    let code_regions = CodeRegionParser::new(&mut code_region_parser_ctx).parse(input);
    
    if !code_region_parser_ctx.import_paths.is_empty() {
        todo!()
    }
    
    let main = PyCodeGen::gen_from_code_regions("<PyMeta main>".into(), code_regions.iter());

    let exe_result = py::impl_pyo3::execute(PyMetaExecutable { main: Rc::new(main) });

    if let Err(ref error) = exe_result.result {
        error.emit_diagnostics();
        exe_result.exe.main.emit_source_dump();
    }

    exe_result.result.unwrap_or_else(|_| TokenStream::new()).into()
}

#[proc_macro_attribute]
pub fn pymodule(params: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    todo!()
}

// #[proc_macro]
// pub fn pymodule_with_rust(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
//     todo!()
// }

// /// ```
// /// pymeta::macro_rules! {
// ///     common {
// ///         ...
// ///     };
// ///     (<pattern1>) => {
// ///         ...
// ///     };
// ///     (<pattern2>) => {
// ///         ...
// ///     };
// /// }
// /// ```
// /// =>
// /// ```python
// /// TODO
// /// ```
// #[proc_macro]
// pub fn macro_rules(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
//     todo!()
// }
