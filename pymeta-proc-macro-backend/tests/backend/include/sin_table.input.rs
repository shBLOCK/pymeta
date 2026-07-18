::quote::quote! { //INCLUDE_IGNORE_LINE
    $from math import *;
    $N = 256;
    // `Token.join()` works like `str.join()`.
    // `Punct` is one type (subclass) of `Token`, corresponding to Rust's `TokenTree::Punct`.
    const SIN_TABLE: [f32; $N$] = [$Punct(',').join(sin(i / N * tau) for i in range(N))$];
} //INCLUDE_IGNORE_LINE