use crate::test_proc_macro_impl;

#[test]
fn vecs() {
    test_proc_macro_impl! {
        pymeta {
            include_quote!("include/vecs.input.rs")
        } => {
            include!("include/vecs.output.rs")
        }
    }
}

#[test]
fn vecs_ops() {
    test_proc_macro_impl! {
        pymeta {
            include_quote!("include/vecs_ops.input.rs")
        } => {
            include!("include/vecs_ops.output.rs")
        }
    }
}
