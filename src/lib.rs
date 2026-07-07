#![doc = include_str!("../README.md")]

pub use pymeta_proc_macro::{pymeta, pymodule};

#[doc(hidden)]
pub mod __internal {
    pub use pymeta_proc_macro::_pymeta_main;

    #[doc(hidden)]
    #[macro_export]
    macro_rules! __make_pymodule_macro {
        { $d:tt $name:ident $file:literal { $($content:tt)* } } => {
            #[macro_export] // todo: make this optional
            macro_rules! $name {
                { $d d:tt {$d($d import_path:tt)*} $d($d mac_start:ident)?$d(::$d mac_path:ident)* ! { $d($d body:tt)* } $d($d extra:tt)* } => {
                    $d($d mac_start)?$d(::$d mac_path)* ! {
                        $d($d body)*
                        module $name $file {$d($d import_path)*} { $($content)* }
                        $d($d extra)*
                    }
                }
            }
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
