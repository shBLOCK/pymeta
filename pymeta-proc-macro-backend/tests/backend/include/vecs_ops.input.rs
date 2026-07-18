::quote::quote! { //INCLUDE_IGNORE_LINE
    // We can write arbitrary Python statements.
    $BINARY_OPS = [
        ("Add", "+"),
        ("Sub", "-"),
        ("Mul", "*"),
        ("Div", "/"),
        ("Rem", "%"),
    ]; // Python statements needs to be terminated by semicolons.
    
    $for dims in range(2, 5):{
        $for op_name, op_sym in BINARY_OPS:{
            $for inplace in [False, True]:{
                impl std::ops::$(op_name + ("Assign" if inplace else ""))$ for Vec~$dims$ {
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
                        // Prefixed string literals are reserved syntax in Rust,
                        // so normal Python f-string syntax won't work.
                        // As a workaround, write `f~"string"` instead.
                        // (Here you can also use the identifier concatenation syntax explained earlier instead of f-string)
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
