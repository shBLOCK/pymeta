::quote::quote! { //INCLUDE_IGNORE_LINE
    // Compile-time Python code are denoted by `$`s. The rest are normal Rust code.
    // A single `$` starts a Python statement.
    // A `:` followed by `{` starts an "indented code block" in Python.
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
} //INCLUDE_IGNORE_LINE
