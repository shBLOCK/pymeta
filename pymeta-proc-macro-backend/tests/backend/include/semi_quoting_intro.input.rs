::quote::quote! { //INCLUDE_IGNORE_LINE
    $param_name = "name";
    // The `Tokens` class can be used for semi-quoting.
    // (Refer to the vector swizzle example for details on the `Tokens` class.)
    $with Tokens() as signiture:{
        fn say_hallo($param_name$: &str)
    }
    // There's also a dedicated "semi-quoting expression" syntax `{{...}}`.
    $signiture = {{ fn say_hello($param_name$: &str) }};

    trait Hello {
        $signiture$;
    }
    struct MyStruct;
    impl Hello for MyStruct {
        $signiture$ {
            println!("Hello {}!", $param_name$);
        }
    }
} //INCLUDE_IGNORE_LINE
