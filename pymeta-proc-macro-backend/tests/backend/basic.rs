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

#[test]
fn semi_quoting_intro() {
    test_proc_macro_impl! {
        pymeta {
            include_quote!("include/semi_quoting_intro.input.rs")
        } => {
            include!("include/semi_quoting_intro.output.rs")
        }
    }
}

#[test]
fn func_fma() {
    test_proc_macro_impl! {
        pymeta_func(a: int, b: int, c: int) {
            #[public(crate)]
            fma! {
                return a * b + c;
            }
        } => {
            ::pymeta::__make_func_macro! {
                $__PYM__ fma __pymeta_module_fma pub (crate),
                [] [doc = "TEST_IGNORE"],
                (a : int, b : int, c : int) { return a * b + c; },
            }
        }
    }
}

#[test]
fn func_sorted_array() {
    test_proc_macro_impl! {
        pymeta_func(include_quote!("include/func_sorted_array.param.rs")) {
            include!("include/func_sorted_array.input.rs")
        } => {
            ::pymeta::__make_func_macro! {
                $__PYM__ sorted_array __pymeta_module_sorted_array pub (self),
                [] [doc = "TEST_IGNORE"],
                (name: str, typ: Tokens, items_dict: dict, key=None) {
                    key = key or (lambda x: x);
                    items = sorted(items_dict.items(), key = lambda kv : key(kv[0]));
                    return {{
                        const __PYM__ name __PYM__: [__PYM__ typ __PYM__; __PYM__ len(items) __PYM__] =
                        [__PYM__ for k, v in items : { (__PYM__ k __PYM__, __PYM__ v __PYM__), }];
                    }};
                },
            }
        }
    }
}