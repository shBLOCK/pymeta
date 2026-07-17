::quote::quote! { //INCLUDE_IGNORE_LINE
    $from math import *;
    $N = 256;
    // `Token.join()` works like `str.join()`.
    const SIN_TABLE: [f32; $N$] = [$Punct(',').join(sin(i / N * tau) for i in range(N))$];
} //INCLUDE_IGNORE_LINE