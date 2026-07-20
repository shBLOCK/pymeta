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

#[test]
fn tokens_parse() {
    test_proc_macro_impl! {
        pymeta {
            $Tokens.parse(r#"
                fn main() {
                    println!("Hello, {}, {}, {}, {}", 1, 1.0, 'a', b'a');
                }
            "#)$
        } => {
            fn main() {
                println!("Hello, {}, {}, {}, {}", 1, 1.0, 'a', b'a');
            }
        }
    }
}

#[test]
fn pymeta_module() {
    test_proc_macro_impl! {
        pymeta_module() {
            common! {
                def entity_struct_name(name: str):{
                    return name + "Entity";
                }

                ENTITY_COMMON_FIELDS = [
                    ("health", "f32"),
                    ("position", "Vec3"),
                ];
            }
        } => {
            ::pymeta::__make_module_macro! {
                $common __pymeta_module_common pub (self),
                [] [doc = "TEST_IGNORE"],
                r#"TEST_IGNORE"# {
                    def entity_struct_name(name: str):{
                        return name + "Entity";
                    }
                    ENTITY_COMMON_FIELDS = [
                        ("health", "f32"),
                        ("position", "Vec3"),
                    ];
                },
            }
        }
    }
}

#[test]
fn import_meta() {
    test_proc_macro_impl! {
        pymeta {
            $import! foo::bar1;
            $import! foo::bar2 as baz;
            $import! foo::bar3.abc;
            $import! foo::bar4.abc as bcd;
            $import! foo::bar5.{self, a, b as c};
            $import! foo::bar6.*;

            $print("Hello world");
        } => {
            foo::bar6! { {foo::bar6}
                foo::bar5! { {foo::bar5}
                    foo::bar4! { {foo::bar4}
                        foo::bar3! { {foo::bar3}
                            foo::bar2! { {foo::bar2}
                                foo::bar1! { {foo::bar1}
                                    ::pymeta::__internal::_pymeta_main! {
                                        main {
                                            $import! foo::bar1;
                                            $import! foo::bar2 as baz;
                                            $import! foo::bar3.abc;
                                            $import! foo::bar4.abc as bcd;
                                            $import! foo::bar5.{self, a, b as c};
                                            $import! foo::bar6.*;
                                            $print("Hello world");
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn pymeta_main_with_module() {
    test_proc_macro_impl! {
        _pymeta_main {
            main {
                $import! py_utils::common;
                $import! py_utils::common.entity_struct_name;
                struct $entity_struct_name("Cat")$ {
                    $for field,typ in common.ENTITY_COMMON_FIELDS:{
                        $field$ : $typ$,
                    }
                    cat_type: CatType,
                }
            }
            module common r#"TEST_IGNORE"# {py_utils::common} {
                def entity_struct_name(name: str):{
                    return name + "Entity";
                }

                ENTITY_COMMON_FIELDS = [
                    ("health", "f32"),
                    ("position", "Vec3"),
                ];

                {{
                    __PYM__ foo = 1;
                }};
            }
        } => {
            struct CatEntity {
                health: f32,
                position: Vec3,
                cat_type: CatType,
            }
        }
    }
}
