#![doc = include_str!("../README.md")]

pub use pymeta_proc_macro::{pymeta, pymodule};

#[doc(hidden)]
pub mod __internal {
    pub use pymeta_proc_macro::_pymeta_main;

    #[macro_export]
    macro_rules! make_pymodule_macro {
        { $d:tt $name:ident $file:literal $content_block:block } => {
            macro_rules! $name {
                // {
                //     macro_rules! $d name:ident {
                //         $d pattern:block => {
                //             $d mac:path {
                //                 $d($d body:tt)*
                //             }
                //         };
                //     }
                //     $d($d extra:tt)*
                // } => {
                //     macro_rules! $d name {
                //         $d pattern => {
                //             $d mac {
                //                 $d($d body)*
                //                 $content
                //                 $d($d extra)*
                //             }
                //         };
                //     }
                // };
                { $d import_path_block:block $d mac:path { $d($d body:tt)* } $d($d extra:tt)* } => {
                    $d mac {
                        $d($d body)*
                        module $name $file $d import_path_block $content_block
                        $d($d extra)*
                    }
                }
            }
        };
    }

    #[macro_export]
    macro_rules! make_pymacro_macro {
        { $d:tt $name:ident $($body:tt)* } => {
            macro_rules! $name {
                ($d($d tokens:tt)*) => {
                    $crate::__internal::_pymeta_main {
                        $($body)*
                        "macro_input" { $d($d tokens)* }
                    }
                };
            }
        }
    }
}
