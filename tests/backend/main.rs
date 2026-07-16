mod basic;

use proc_macro2::{TokenStream, TokenTree};
use std::io::Write;
use std::process::{Command, Stdio};

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

fn rustfmt(text: &str) -> String {
    let mut proc = Command::new("rustfmt")
        .args(["--emit", "stdout", "--edition", "2024"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn rustfmt");
    proc.stdin
        .as_mut()
        .unwrap()
        .write_all(text.as_bytes())
        .expect("failed to write to rustfmt");
    let result = proc.wait_with_output().expect("failed to run rustfmt");

    if result.status.success() {
        String::from_utf8(result.stdout).unwrap()
    } else {
        eprintln!(
            "rustfmt failed with status {}, stderr: {}",
            result.status,
            String::from_utf8(result.stderr).unwrap_or_else(|e| format!("<{e:?}>"))
        );
        String::from(text)
    }
}

#[allow(unused)]
macro_rules! ignore {
    ($($_:tt)*) => {};
}

macro_rules! test_proc_macro_impl {
    {
        $macro_name:ident $(($($param:tt)*))? { $($input:tt)* }
        => { $($output:tt)* }
    } => {
        $(let param = ::quote::quote!($($param)*);)?
        let input = ::quote::quote! { $($input)* };
        let expected_output = ::quote::quote! { $($output)* };
        let result = ::pymeta_proc_macro_backend::run_proc_macro(|| {
            ::pymeta_proc_macro_backend::$macro_name(input $(, param ignore!($($param)*))?)
        });
        let Some(output) = result.tokens else { panic!("no output") };
        if !result.diagnostics.is_empty() {
            eprintln!("Emitted diagnostics: {:#?}", result.diagnostics);
        }
        if !crate::tokens_eq(output.clone(), expected_output.clone()) {
            let output_text = crate::rustfmt(&output.to_string());
            let expected_output_text = crate::rustfmt(&expected_output.to_string());
            let diff = ::pretty_assertions::StrComparison::new(&output_text, &expected_output_text);
            println!("Output:\n{output_text}");
            print!("{diff}");
            panic!("incorrect output");
        }
    };
}
pub(crate) use test_proc_macro_impl;
