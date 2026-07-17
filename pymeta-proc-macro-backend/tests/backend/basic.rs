use crate::test_proc_macro_impl;

#[test]
fn vecs_struct() {
    test_proc_macro_impl! {
        pymeta {
            include_quote!("include/vecs_struct.input.rs")
        } => {
            include!("include/vecs_struct.output.rs")
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

#[test]
fn vecs_swizzle() {
    test_proc_macro_impl! {
        pymeta {
            include_quote!("include/vecs_swizzle.input.rs")
        } => {
            include!("include/vecs_swizzle.output.rs")
        }
    }
}