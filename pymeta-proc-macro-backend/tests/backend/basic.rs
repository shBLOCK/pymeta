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

#[test]
fn golden_ratio() {
    test_proc_macro_impl! {
        // quote doesn't preserve `**` spacing, so use parse
        pymeta { parse!("$f32((1 + 5 ** 0.5) / 2)$") } => { 1.618034f32 }
    }
}

#[test]
fn sin_table() {
    test_proc_macro_impl! {
        pymeta {
            include_quote!("include/sin_table.input.rs")
        } => {
            include!("include/sin_table.output.rs")
        }
    }
}

#[test]
fn sin_table_np() {
    test_proc_macro_impl! {
        pymeta {
            include_quote!("include/sin_table_np.input.rs")
        } => {
            include!("include/sin_table.output.rs")
        }
    }
}
