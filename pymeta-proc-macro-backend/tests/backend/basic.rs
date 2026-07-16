use crate::test_proc_macro_impl;

#[test]
fn vecs() {
    test_proc_macro_impl! {
        pymeta {
            // Compile-time Python code are denoted by `$`s. The rest are normal Rust code.
            // A single `$` starts a Python statement.
            $for dims in range(2, 5):{
                // Rust code inside a Python for-loop will be repeated.
                #[derive(Clone, Copy, Debug, PartialEq)]
                // `$...$` inserts a Python value as Rust code.
                // `~` concatenates the Python value onto `Vec`, so we get `Vec2` and not `Vec 2`.
                struct Vec~$dims$ {
                    $for i in range(dims):{
                        // `$...$` works with any Python expression.
                        $"xyzw"[i]$: f32,
                    }
                }
            }
        } => {
            #[derive(Clone, Copy, Debug, PartialEq)]
            struct Vec2 {
                x: f32,
                y: f32,
            }
            #[derive(Clone, Copy, Debug, PartialEq)]
            struct Vec3 {
                x: f32,
                y: f32,
                z: f32,
            }
            #[derive(Clone, Copy, Debug, PartialEq)]
            struct Vec4 {
                x: f32,
                y: f32,
                z: f32,
                w: f32,
            }
        }
    }
}

#[test]
fn vecs_ops() {
    test_proc_macro_impl! {
        pymeta {
            // We can write arbitrary Python statements.
            // "Logical Python lines" needs to be terminated by semicolons.
            // (If they are not control-flow statements that starts a code block)
            $BINARY_OPS = [
                ("Add", "+"),
                ("Sub", "-"),
                ("Mul", "*"),
                ("Div", "/"),
                ("Rem", "%"),
            ];

            $for dims in range(2, 5):{
                $for op_name, op_sym in BINARY_OPS:{
                    $for inplace in [False, True]:{
                        impl std::ops::$op_name + ("Assign" if inplace else "")$ for Vec~$dims$ {
                            // Python control flows can be used to control code generation.
                            $if not inplace:{ type Output = Vec~$dims$; }

                            $if not inplace:{
                                fn $op_name.lower()$(self, rhs: Self) -> Self {
                                    Self {
                                        $for d in "xyzw"[:dims]:{
                                            $d$: self.$d$ $op_sym$ rhs.$d$,
                                        }
                                    }
                                }
                            } $else:{
                                // Prefixed literals are reserved syntax in Rust,
                                // to work around this, `f"string"` can be written as `f~"string"`.
                                fn $f~"{op_name.lower()}_assign"$(&mut self, rhs: Self) {
                                    $for d in "xyzw"[:dims]:{
                                        self.$d$ $op_sym + "="$ rhs.$d$;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } => {
            include!("expansion/vecs_ops.rs")
        }
    }
}
