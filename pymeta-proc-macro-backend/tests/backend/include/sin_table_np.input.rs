::quote::quote! { //INCLUDE_IGNORE_LINE
    $import numpy as np;
    $N = 256;
    const SIN_TABLE: [f32; $N$] = [$Punct(',').join(np.sin(np.linspace(0, np.pi * 2, N, endpoint=False)))$];
} //INCLUDE_IGNORE_LINE