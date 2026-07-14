#![cfg_attr(feature = "nightly_diagnostic", feature(proc_macro_diagnostic))]

use crate::rust_to_py::code_region::parser::{CodeRegionParser, CodeRegionParserCtx, CodeRegionParserSettings};
use crate::rust_to_py::meta::stmt::ImportMetaStmt;
use crate::rust_to_py::py_code_gen::{PyCodeGen, PyCodeGenContext, PyMetaExecutable, PyMetaModule};
use crate::rust_to_py::utils::py_markers_to_py_marker_idents;
use crate::utils::LiteralRawStringExt;
use crate::utils::diagnostic::set_dummy_output;
use crate::utils::parsing::{RustAttribute, RustSimplePath, RustVis};
use crate::utils::rust_token::TokenOptionEx;
use proc_macro2::{Delimiter, Group, Ident, Literal, Span, TokenStream, TokenTree};
use quote::{TokenStreamExt, quote};
use std::collections::HashMap;
use std::iter::repeat_n;
use std::rc::Rc;
use utils::rust_token::TokenBuffer;

mod py;
mod rust_to_py;
pub mod utils;

const MAIN_MODULE_NAME: &str = "__main__";

fn format_module_name(file: &str, name: &str) -> String {
    format!("<{file}>::{name}")
}

const PYMETA_MODULE_PREFIX: &str = "__pymeta_module_";

/// This is the final macro call that will actually execute the Python code.
/// It's expected that all `import!`d modules have been included when calling this macro.
pub fn _pymeta_main(input: TokenStream) -> TokenStream {
    set_dummy_output(quote! { { loop {} } });

    let mut input = TokenBuffer::from_iter(input);
    let mut main_module = None;
    #[allow(clippy::mutable_key_type)]
    let mut modules = HashMap::<Rc<RustSimplePath>, PyMetaModule>::new();
    let mut codegen_ctx = PyCodeGenContext::new();
    while !input.exhausted() {
        match input
            .read_one()
            .ident()
            .ok()
            .unwrap_or_else(|| abort!(input.get_current_span_for_diagnostics(), ""))
            .inner()
            .to_string()
            .as_str()
        {
            "main" => {
                main_module = Some(PyCodeGen::gen_from_code_regions(
                    None,
                    MAIN_MODULE_NAME.into(),
                    format_module_name(&Span::call_site().file(), MAIN_MODULE_NAME),
                    CodeRegionParser::new(CodeRegionParserSettings::default(), &mut CodeRegionParserCtx::new())
                        .parse(input.read_one().expect_group(Delimiter::Brace).unwrap().tokens())
                        .iter(),
                    &mut codegen_ctx,
                ));
            }
            "module" => {
                let name = input.read_one().ident().unwrap().inner().to_string();
                let file = input
                    .read_one()
                    .maybe_unwrap_none_group()
                    .as_ref()
                    .literal()
                    .unwrap()
                    .inner()
                    .raw_string_value();
                let import_path = Rc::new(
                    RustSimplePath::try_parse(&mut input.read_one().expect_group(Delimiter::Brace).unwrap().tokens())
                        .unwrap(),
                );
                modules.entry(import_path.clone()).or_insert_with(|| {
                    let body = input.read_one().expect_group(Delimiter::Brace).unwrap().tokens();
                    let filename = format!("{} ({import_path})", format_module_name(&file, &name));
                    PyCodeGen::gen_from_code_regions(
                        Some(ImportMetaStmt::PATH.into()),
                        ImportMetaStmt::module_name(&import_path),
                        filename,
                        CodeRegionParser::new(
                            CodeRegionParserSettings { pure_python_mode: true },
                            &mut CodeRegionParserCtx::new(),
                        )
                        .parse(body)
                        .iter(),
                        &mut codegen_ctx,
                    )
                });
            }
            it => abort!(
                input.get_current_span_for_diagnostics(),
                "Unknown param block type: {}",
                it
            ),
        }
    }
    run_pymeta_executable(PyMetaExecutable::new(
        Rc::new(main_module.expect("main module not given")),
        modules.into_values().map(Rc::new).collect(),
        codegen_ctx,
    ))
}

/// TODO: detailed documentation will be available here,
/// for now you can check out the examples in the crate's top-level documentation
pub fn pymeta(input: TokenStream) -> TokenStream {
    set_dummy_output(quote! { { loop {} } });

    let input = TokenBuffer::from_iter(input);
    let mut code_region_parser_ctx = CodeRegionParserCtx::new();
    let code_regions =
        CodeRegionParser::new(CodeRegionParserSettings::default(), &mut code_region_parser_ctx).parse(input.clone());

    if !code_region_parser_ctx.import_paths.is_empty() {
        let tokens = quote! {
            ::pymeta::__internal::_pymeta_main! {
                main { #input }
            }
        };
        return wrap_with_import_pymeta_module_macro_calls(tokens, code_region_parser_ctx.import_paths.iter());
    }

    let mut codegen_ctx = PyCodeGenContext::new();
    let main = PyCodeGen::gen_from_code_regions(
        None,
        MAIN_MODULE_NAME.into(),
        format_module_name(&Span::call_site().file(), MAIN_MODULE_NAME),
        code_regions.iter(),
        &mut codegen_ctx,
    );

    run_pymeta_executable(PyMetaExecutable::new(Rc::new(main), [].into(), codegen_ctx))
}

pub fn pymeta_module(params: TokenStream, input: TokenStream) -> TokenStream {
    if let Some(token) = params.into_iter().next() {
        abort!(token.span(), "Unexpected parameters");
    }
    let mut input = TokenBuffer::from_iter(input);

    // attributes
    let mut vis = None;
    let mut macro_attrs = Vec::new();
    let mut reexport_attrs = Vec::new();
    while let Ok(attr) = RustAttribute::try_parse(&mut input) {
        let (apply_to_macro, apply_to_reexport) = match attr.path.to_string().as_str() {
            "public" => {
                if vis.is_some() {
                    abort!(attr.path.total_span(), "duplicate vis specification");
                }
                let _ =
                    vis.insert(RustVis::try_parse("public", &mut attr.group.tokens()).unwrap_or_else(|e| e.abort()));
                continue;
            }
            "macro_export" => abort!(
                attr.path.total_span(),
                "Explicit `#[macro_export]` not allowed, use `#[public]` instead"
            ),
            "allow" | "expect" | "warn" | "deny" | "forbid" => (true, true),
            "deprecated" => (true, true),
            "doc" => (false, true),
            _ => (true, false),
        };
        if apply_to_macro {
            macro_attrs.push(attr.group.inner().stream());
        }
        if apply_to_reexport {
            reexport_attrs.push(attr.group.inner().stream());
        }
    }
    if let Some(ref vis) = vis
        && vis.is_pub()
    {
        macro_attrs.push(quote! { macro_export });
    }

    // names
    let Ok(name_ident) = input.read_one().ident() else {
        abort!(
            input.get_current_span_for_diagnostics(),
            "Expected module name identifier"
        );
    };
    if !input.read_one().eq_punct('!') {
        abort!(input.get_current_span_for_diagnostics(), "Expected `!`");
    }
    let name = name_ident.inner().to_string();
    let mangled_name_ident = Ident::new(&format!("{PYMETA_MODULE_PREFIX}{name}"), name_ident.span().inner());
    let file = Span::call_site().file();
    let file_literal = Literal::raw_string(&file);

    // body
    let Ok(body_group) = input.read_one().expect_group(Delimiter::Brace) else {
        abort!(input.get_current_span_for_diagnostics(), "Expected `{<module body>}`");
    };
    let mut code_region_parser_ctx = CodeRegionParserCtx::new();
    CodeRegionParser::new(
        CodeRegionParserSettings { pure_python_mode: true },
        &mut code_region_parser_ctx,
    )
    .parse(body_group.tokens());
    let body = py_markers_to_py_marker_idents(body_group.inner().stream());
    {
        // source doc
        let body_tokens = body_group.inner().stream().into_iter().collect::<Vec<_>>();
        let source = if let (Some(first), Some(last)) = (body_tokens.first(), body_tokens.last()) {
            first
                .span()
                .join(last.span())
                .and_then(|s| s.source_text())
                .map(|source| {
                    // strip common indent
                    let lines = source
                        .lines()
                        .map(|mut line| {
                            let mut indent: usize = 0;
                            while !line.is_empty() {
                                let space_size = match line.as_bytes()[0] {
                                    b' ' => 1,
                                    b'\t' => 4,
                                    _ => break,
                                };
                                indent += space_size;
                                line = &line[1..];
                            }
                            (indent, line)
                        })
                        .collect::<Vec<_>>();
                    let common_indent = lines.iter().map(|(indent, _)| *indent).skip(1).min().unwrap_or(0);

                    let mut result = String::new();
                    for (indent, line) in lines {
                        let indent = indent.saturating_sub(common_indent);
                        result.extend(repeat_n(' ', indent));
                        result.push_str(line);
                        result.push('\n');
                    }
                    result
                })
        } else {
            None
        };
        let source = source.unwrap_or(body_group.inner().stream().to_string());
        let source_doc = Literal::raw_string(
            format!("\n\n[pymeta_module][::pymeta::pymeta_module] `{name}` definition\n---\n```\n{source}\n```")
                .as_str(),
        );
        reexport_attrs.push(quote! { doc = #source_doc });
    }

    // output
    let tokens = quote! {
        ::pymeta::__make_module_macro! {
            $,
            #name_ident #mangled_name_ident #file_literal,
            #vis,
            [#(#macro_attrs),*] [#(#reexport_attrs),*],
            { #body },
        }
    };
    wrap_with_import_pymeta_module_macro_calls(tokens, code_region_parser_ctx.import_paths.iter())
}

fn run_pymeta_executable(exe: PyMetaExecutable) -> TokenStream {
    let exe_result = py::impl_pyo3::execute(exe);

    if let Err(ref error) = exe_result.result {
        error.emit_diagnostics();
        #[cfg(feature = "dump_source_on_error")]
        {
            exe_result.exe.main.emit_source_dump();
            exe_result.exe.modules.iter().for_each(|it| it.emit_source_dump());
        }
    }

    exe_result.result.unwrap_or_else(|_| abort!())
}

fn wrap_with_import_pymeta_module_macro_calls<'a>(
    mut tokens: TokenStream,
    import_paths: impl Iterator<Item = &'a Rc<RustSimplePath>>,
) -> TokenStream {
    for import_path in import_paths {
        tokens = quote! {
            #import_path! { {#import_path} #tokens }
        };
    }
    tokens
}
