#![feature(proc_macro_diagnostic)]
#![cfg_attr(feature = "nightly_proc_macro_span", feature(proc_macro_span))]

extern crate proc_macro;

use crate::rust_to_py::code_region::parser::{CodeRegionParser, CodeRegionParserCtx};
use crate::rust_to_py::py_code_gen::{PyCodeGen, PyMetaExecutable, PyMetaModule};
use crate::utils::LiteralRawStringExt;
use crate::utils::parsing::SimpleRustPath;
use crate::utils::rust_token::TokenOptionEx;
use proc_macro_error3::abort;
use proc_macro2::{Delimiter, Group, Ident, Literal, Span, TokenStream, TokenTree};
use quote::{TokenStreamExt, quote};
use std::collections::HashMap;
use std::rc::Rc;
use utils::rust_token::TokenBuffer;

mod py;
mod rust_to_py;
mod utils;

const MAIN_MODULE_NAME: &str = "__main__";

fn format_module_name(file: &str, name: &str) -> String {
    format!("<{file}>::{name}")
}

/// This is the final macro call that will actually execute the Python code.
/// It's expected that all `import!`d modules have been included when calling this macro.
#[proc_macro]
#[proc_macro_error3::proc_macro_error]
pub fn _pymeta_main(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut input = TokenBuffer::from_iter(TokenStream::from(input));
    let mut main_module = None;
    #[allow(clippy::mutable_key_type)]
    let mut modules = HashMap::<Rc<SimpleRustPath>, PyMetaModule>::new();
    while !input.exhausted() {
        match input
            .read_one()
            .ident()
            .unwrap_or_else(|| abort!(input.get_current_span_for_diagnostics(), ""))
            .inner()
            .to_string()
            .as_str()
        {
            "main" => {
                main_module = Some(PyCodeGen::gen_from_code_regions(
                    format_module_name(&Span::call_site().file(), MAIN_MODULE_NAME),
                    CodeRegionParser::new(&mut CodeRegionParserCtx::new())
                        .parse(input.read_one().expect_group(Delimiter::Brace).unwrap().tokens())
                        .iter(),
                ));
            }
            "module" => {
                let name = input.read_one().ident().unwrap().inner().to_string();
                let file = input.read_one().literal().unwrap().inner().raw_string_value();
                let import_path = Rc::new(
                    SimpleRustPath::parse(&mut input.read_one().expect_group(Delimiter::Brace).unwrap().tokens())
                        .unwrap(),
                );
                modules.entry(import_path.clone()).or_insert_with(|| {
                    let body = input.read_one().expect_group(Delimiter::Brace).unwrap().tokens();
                    let module = PyCodeGen::gen_from_code_regions(
                        format!("{} ({import_path})", format_module_name(&file, &name)),
                        CodeRegionParser::new(&mut CodeRegionParserCtx::new())
                            .parse(body)
                            .iter(),
                    );
                    module
                });
            }
            it => abort!(
                input.get_current_span_for_diagnostics(),
                "Unknown param block type: {}",
                it
            ),
        }
    }
    run_pymeta_executable(PyMetaExecutable {
        main: Rc::new(main_module.expect("main module not given")),
        //TODO: modules
    })
    .into()
}

/// TODO: detailed documentation will be available here,
/// for now you can check out the examples in the crate's top-level documentation
#[proc_macro]
#[proc_macro_error3::proc_macro_error]
pub fn pymeta(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = TokenBuffer::from_iter(TokenStream::from(input));
    let mut code_region_parser_ctx = CodeRegionParserCtx::new();
    let code_regions = CodeRegionParser::new(&mut code_region_parser_ctx).parse(input.clone());

    if !code_region_parser_ctx.import_paths.is_empty() {
        let tokens = quote! {
            ::pymeta::__internal::_pymeta_main! {
                main { #input }
            }
        };
        return wrap_with_import_pymodule_macro_calls(tokens, code_region_parser_ctx.import_paths.iter()).into();
    }

    let main = PyCodeGen::gen_from_code_regions(
        format_module_name(&Span::call_site().file(), MAIN_MODULE_NAME),
        code_regions.iter(),
    );

    run_pymeta_executable(PyMetaExecutable { main: Rc::new(main) }).into()
}

#[proc_macro_attribute]
#[proc_macro_error3::proc_macro_error]
pub fn pymodule(_params: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut input = TokenBuffer::from_iter(TokenStream::from(input));
    let Some(name_ident) = input.read_one().ident() else {
        abort!(
            input.get_current_span_for_diagnostics(),
            "Expected module name identifier"
        );
    };
    if !input.read_one().eq_punct('!') {
        abort!(input.get_current_span_for_diagnostics(), "Expected `!`");
    }
    let Some(body_group) = input.read_one().expect_group(Delimiter::Brace) else {
        abort!(input.get_current_span_for_diagnostics(), "Expected `{<module body>}`");
    };

    let mut code_region_parser_ctx = CodeRegionParserCtx::new();
    CodeRegionParser::new(&mut code_region_parser_ctx).parse(body_group.tokens());

    let file = Span::call_site().file();
    let file_literal = Literal::raw_string(&file);
    let body = replace_dollar_with_meta_var(body_group.inner().stream());

    let tokens = quote! {
        ::pymeta::__make_pymodule_macro! {
            $ #name_ident #file_literal { #body }
        }
    };
    wrap_with_import_pymodule_macro_calls(tokens, code_region_parser_ctx.import_paths.iter()).into()
}

fn run_pymeta_executable(exe: PyMetaExecutable) -> TokenStream {
    let exe_result = py::impl_pyo3::execute(exe);

    if let Err(ref error) = exe_result.result {
        error.emit_diagnostics();
        exe_result.exe.main.emit_source_dump();
    }

    exe_result.result.unwrap_or_else(|_| TokenStream::new())
}

/// replace `$` with `$d`
fn replace_dollar_with_meta_var(tokens: TokenStream) -> TokenStream {
    let d = Ident::new("d", Span::call_site());
    let mut new_tokens = TokenStream::new();
    for token in tokens {
        match token {
            TokenTree::Punct(dollar) if dollar.as_char() == '$' => {
                new_tokens.append(dollar);
                new_tokens.append(d.clone());
            }
            TokenTree::Group(group) => {
                let mut new_group = Group::new(group.delimiter(), replace_dollar_with_meta_var(group.stream()));
                new_group.set_span(group.span());
                new_tokens.append(new_group);
            }
            token => new_tokens.append(token),
        }
    }
    new_tokens
}

fn wrap_with_import_pymodule_macro_calls<'a>(
    mut tokens: TokenStream,
    import_paths: impl Iterator<Item = &'a Rc<SimpleRustPath>>,
) -> TokenStream {
    for import_path in import_paths {
        tokens = quote! {
            #import_path! { $ {#import_path} #tokens }
        };
    }
    tokens
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
