#![feature(proc_macro_span)]
#![feature(proc_macro_totokens)]
extern crate proc_macro;

use crate::rust_to_py::code_regions::parser::CodeRegionParser;
use crate::rust_to_py::py_code_gen::PyCodeGen;
use proc_macro2::TokenStream;
use proc_macro_error2::abort_call_site;
use utils::rust_token::TokenBuffer;

mod py;
mod rust_to_py;
mod utils;

#[proc_macro]
#[proc_macro_error2::proc_macro_error]
pub fn pymeta(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    #[cfg(feature = "log")]
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .format_timestamp(None)
        .format_module_path(false)
        .format_source_path(true)
        .try_init();

    let input = TokenBuffer::from_iter(TokenStream::from(input));
    let code_regions = CodeRegionParser::new().parse(input);
    let exe = {
        let mut codegen = PyCodeGen::new();
        codegen.append_code_regions(code_regions.iter());
        codegen.finish()
    };

    // let s = exe.source.source_code();
    // let s = format!("{code_regions:#?}");
    // abort_call_site!(s);
    // quote! {#s}.into()

    let exe_result = py::impl_pyo3::execute(exe);
    let output = match exe_result.result {
        Ok(output) => output,
        Err(err) => abort_call_site!("Python error: {}", err.tmp_string),
    };

    output.into()

    // let py_source = PySourceBuilder::new();

    // let tok = input.peek(4).unwrap().span().unwrap();
    // let s = format!("{:?} {:?} {:?}", tok.start(), tok.end(), tok.inner().byte_range());

    // let s = format!("{:?}", std::env::current_exe());
    // let s = format!("{input:#?}");
    // let s = format!("{py_source:#?}");
    // let s = py_source.source_code();
    // let s = format!("{:#?}", CodeRegion::parse(input.clone()));
    // let s = format!("{:#?}", input);
    // let s = format!("{:#?}", code_regions);
    // quote! { #s }.into()

    // let s = py_source.source_code();
    // let ss = s.lines().collect::<Vec<_>>();
    // quote! { #(const _: &str = #ss;)* }.into()

    // use proc_macro::ToTokens;
    // let info = input.into_iter().map(|t| (t.span().column(), t.span().byte_range(), t.span().source_text())).collect::<Vec<_>>();
    // let s = format!("{info:?}");
    // proc_macro::TokenTree::Literal(proc_macro::Literal::string(&s)).into_token_stream()
}
