use pymeta_proc_macro_backend as backend;
use pymeta_proc_macro_backend::run_proc_macro;

#[proc_macro]
pub fn _pymeta_main(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    run_proc_macro(|| backend::_pymeta_main(input.into()))
        .resolve_to_tokens()
        .into()
}

#[proc_macro]
pub fn pymeta(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    run_proc_macro(|| backend::pymeta(input.into()))
        .resolve_to_tokens()
        .into()
}

#[proc_macro_attribute]
pub fn pymeta_module(params: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    run_proc_macro(|| backend::pymeta_module(params.into(), input.into()))
        .resolve_to_tokens()
        .into()
}

#[proc_macro_attribute]
pub fn pymeta_func(params: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    run_proc_macro(|| backend::pymeta_func(params.into(), input.into()))
        .resolve_to_tokens()
        .into()
}
