mod basic;

use proc_macro2::{TokenStream, TokenTree};

fn tokens_eq(a: TokenStream, b: TokenStream) -> bool {
    fn token_eq(a: TokenTree, b: TokenTree) -> bool {
        use TokenTree::*;
        match (a, b) {
            (Ident(a), Ident(b)) => a == b,
            (Punct(a), Punct(b)) => a.as_char() == b.as_char() && a.spacing() == b.spacing(),
            (Literal(a), Literal(b)) => a.to_string() == b.to_string(),
            (Group(a), Group(b)) => a.delimiter() == b.delimiter() && tokens_eq(a.stream(), b.stream()),
            _ => false,
        }
    }

    let (mut a, mut b) = (a.into_iter(), b.into_iter());
    loop {
        match (a.next(), b.next()) {
            (Some(a), Some(b)) => {
                if !token_eq(a, b) {
                    return false;
                }
            }
            (None, None) => return true,
            _ => return false,
        }
    }
}

fn tokens_to_code(tokens: TokenStream) -> String {
    let file = syn::parse2::<syn::File>(tokens).unwrap();
    prettyplease::unparse(&file)
    // tokens.to_string()
}

#[allow(unused)]
macro_rules! ignore {
    ($($_:tt)*) => {};
}

macro_rules! quote_or_include_tokens {
    { include!($file:expr) } => {
        (std::str::FromStr::from_str(include_str!($file)) as std::result::Result<::proc_macro2::TokenStream, _>)
            .unwrap_or_else(|e| panic!("Failed to parse included Rust file `{}`: {e:?}", $file))
    };
    { include_quote!($file:expr) } => { include!($file) };
    { parse!($str:expr) } => {
        (std::str::FromStr::from_str($str) as std::result::Result<::proc_macro2::TokenStream, _>)
            .unwrap_or_else(|e| panic!("Failed to parse Rust code: {e:?}\ncode:{}", $str))
    };
    { $($tokens:tt)* } => { ::quote::quote! { $($tokens)* } };
}
#[allow(unused)]
pub(crate) use quote_or_include_tokens;

macro_rules! test_proc_macro_impl {
    {
        $macro_name:ident $(($($param:tt)*))? { $($input:tt)* }
        => { $($output:tt)* }
    } => {
        $(let param = $crate::quote_or_include_tokens! { $($param)* };)?
        let input = $crate::quote_or_include_tokens! { $($input)* };
        let expected_output = $crate::quote_or_include_tokens! { $($output)* };
        let result = ::pymeta_proc_macro_backend::run_proc_macro(|| {
            ::pymeta_proc_macro_backend::$macro_name(input $(, param ignore!($($param)*))?)
        });

        ::pyo3::Python::attach(|py| {
            // Hacky workaround to prevent "PySpan is unsendable, but is being dropped on another thread" errors in tests
            let _ = py.run(c"import gc\ngc.collect()", None, None).map_err(|e| eprintln!("gc.collect() failed: {e:?}"));
        });

        let Some(output) = result.tokens else { panic!("no output") };
        if !result.diagnostics.is_empty() {
            eprintln!("Emitted diagnostics: {:#?}", result.diagnostics);
        }
        if !$crate::tokens_eq(output.clone(), expected_output.clone()) {
            let output_text = $crate::tokens_to_code(output.clone());
            let expected_output_text = $crate::tokens_to_code(expected_output.clone());
            let diff = ::pretty_assertions::StrComparison::new(&output_text, &expected_output_text);
            // let diff = ::pretty_assertions::Comparison::new(&output, &expected_output);
            println!("Output:\n{output_text}");
            print!("{diff}");
            panic!("incorrect output");
        }
    };
}
pub(crate) use test_proc_macro_impl;
