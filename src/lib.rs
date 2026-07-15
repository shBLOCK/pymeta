#![doc = include_str!("../README.md")]

pub use pymeta_proc_macro::{pymeta, pymeta_func, pymeta_module};

#[doc(hidden)]
pub mod __internal {
    pub use pymeta_proc_macro::_pymeta_main;

    #[doc(hidden)]
    #[macro_export]
    macro_rules! __make_module_macro {
        {
            $d:tt $name:ident $mangled_name:ident $vis:vis,
            [$($macro_attrs:meta),*] [$($reexport_attrs:meta),*],
            $file:literal { $($content:tt)* },
            $($extra:tt)*
        } => {
            $(#[$macro_attrs])*
            #[doc(hidden)]
            macro_rules! $mangled_name {
                { {$d($d import_path:tt)*} $d($d mac_start:ident)?$d(::$d mac_path:ident)* ! { $d($d body:tt)* } $d($d extra:tt)* } => {
                    $d($d mac_start)?$d(::$d mac_path)* ! {
                        $d($d body)*
                        module $name $file {$d($d import_path)*} { $($content)* }
                        $($extra)*
                        $d($d extra)*
                    }
                }
            }

            $(#[$reexport_attrs])*
            #[doc(inline)] // crate private reexports are not documented currently: https://github.com/rust-lang/rust/issues/159109
            $vis use $mangled_name as $name;
        };
    }

    #[doc(hidden)]
    #[macro_export]
    macro_rules! __make_func_macro {
        {
            $d:tt $py:ident $name:ident $mangled_name:ident $vis:vis,
            [$($macro_attrs:meta),*] [$($reexport_attrs:meta),*],
            ( $($param_list:tt)* ) { $($func_body:tt)* },
            $($extra:tt)*
        } => {
            $(#[$macro_attrs])*
            #[doc(hidden)]
            macro_rules! $mangled_name {
                ($d($d params:tt)*) => {
                    ::pymeta::__internal::_pymeta_main! {
                        main {
                            $py with Tokens._none_ctx():{
                                $py def $name($($param_list)*):$py{
                                    $($func_body)*
                                }
                                $py result = $name($d($d params)*);
                            }
                            $py emit(result);
                        }
                        $($extra)*
                    }
                };
            }

            $(#[$reexport_attrs])*
            #[doc(inline)] // crate private reexports are not documented currently: https://github.com/rust-lang/rust/issues/159109
            $vis use $mangled_name as $name;
        };
    }
}
