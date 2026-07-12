#![doc = include_str!("../README.md")]

pub use pymeta_proc_macro::{pymeta, pymeta_module};

#[doc(hidden)]
pub mod __internal {
    pub use pymeta_proc_macro::_pymeta_main;

    #[doc(hidden)]
    #[macro_export]
    macro_rules! __make_module_macro {
        {
            $d:tt $dollar_d:ident,
            $name:ident $mangled_name:ident $file:literal,
            $vis:vis,
            [$($macro_attrs:meta),*] [$($reexport_attrs:meta),*],
            { $($content:tt)* },
            $($extra:tt)*
        } => {
            $(#[$macro_attrs])*
            #[doc(hidden)]
            macro_rules! $mangled_name {
                { $d $dollar_d:tt {$d($d import_path:tt)*} $d($d mac_start:ident)?$d(::$d mac_path:ident)* ! { $d($d body:tt)* } $d($d extra:tt)* } => {
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

    // #[doc(hidden)]
    // #[macro_export]
    // macro_rules! __make_pymacro_macro {
    //     { $d:tt $name:ident $($body:tt)* } => {
    //         macro_rules! $name {
    //             ($d($d tokens:tt)*) => {
    //                 $crate::__internal::_pymeta_main {
    //                     $($body)*
    //                     "macro_input" { $d($d tokens)* }
    //                 }
    //             };
    //         }
    //     }
    // }
}
