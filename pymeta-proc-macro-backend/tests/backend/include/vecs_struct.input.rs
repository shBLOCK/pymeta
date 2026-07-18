::quote::quote! { //INCLUDE_IGNORE_LINE
    // Compile-time Python code is denoted by `$`s. The rest are normal Rust code.
    // A single `$` starts a Python statement.
    $for dims in range(2, 5):{ // Indents are insignificant in PyMeta, so braces are required.
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
} //INCLUDE_IGNORE_LINE
