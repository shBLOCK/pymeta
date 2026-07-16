::quote::quote! { //INCLUDE_IGNORE_LINE
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
} //INCLUDE_IGNORE_LINE
