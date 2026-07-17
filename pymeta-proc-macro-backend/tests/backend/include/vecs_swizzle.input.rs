::quote::quote! { //INCLUDE_IGNORE_LINE
    // Make use of all the Python modules!
    $import itertools;

    $for in_dims in range(2, 5):{
        // `Tokens` is a Python class defined by PyMeta that contains some Rust code (a list of Rust code "tokens").
        // (If you don't know what a Rust code token is, refer to https://doc.rust-lang.org/proc_macro/enum.TokenTree.html)
        // Here we construct temporary `Tokens` objects for the contents of the trait and the impl block.
        // This way we could generate code to populate both of them at the same time (so we don't have to duplicate code).
        $trait_content = Tokens();
        $impl_content = Tokens();

        $for out_dims in range(2, 5):{
            $out_type = f~"Vec{out_dims}";
            // This is an itertools one-liner to generate swizzle arrangements as tuples.
            $for swizzle in itertools.product("xyzw"[:in_dims], repeat=out_dims):{
                // Use the Python `with` statement to temporarily set a `Tokens` object as the current "Tokens context".
                // This means Rust code within the `with` block are added to that `Tokens` object.
                $with trait_content:{
                    fn $"".join(swizzle)$(self) -> $out_type$;
                }
                $with impl_content:{
                    fn $"".join(swizzle)$(self) -> $out_type$ {
                        $out_type$ {
                            $for a, b in zip("xyzw", swizzle):{
                                $a$: self.$b$,
                            }
                        }
                    }
                }
            }
        }

        // Finally, generate the actual trait and impl blocks.
        trait Vec~$in_dims$~Swizzle {
            $trait_content$ // "Paste in" the content we generated earlier.
        }
        impl Vec~$in_dims$~Swizzle for Vec~$in_dims$ {
            $impl_content$
        }
    }
} //INCLUDE_IGNORE_LINE
