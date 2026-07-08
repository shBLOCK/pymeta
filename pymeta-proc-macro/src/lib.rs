use proc_macro_error3;
use pymeta_proc_macro_backend as backend;

#[proc_macro]
#[proc_macro_error3::proc_macro_error]
pub fn _pymeta_main(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    backend::_pymeta_main(input.into()).into()
}

#[proc_macro]
#[proc_macro_error3::proc_macro_error]
pub fn pymeta(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    backend::pymeta(input.into()).into()
}

#[proc_macro_attribute]
#[proc_macro_error3::proc_macro_error]
pub fn pymodule(params: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    backend::pymodule(params.into(), input.into()).into()
}