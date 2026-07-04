#![doc = include_str!("../README.md")]

pub use pymeta_proc_macro::{pymeta, pymodule};

#[doc(hidden)]
pub mod _internal {
    pub use pymeta_proc_macro::_pymeta_main;

    #[macro_export]
    macro_rules! make_extend_macro {
        { $d:tt $name:ident $($content:tt)* } => {
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
                { $d mac:path { $d($d body:tt)* } $d($d extra:tt)* } => {
                    $d mac {
                        $d($d body)*
                        $content
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
                    $crate::_internal::_pymeta_main {
                        $($body)*
                        "macro_input" { $d($d tokens)* }
                    }
                };
            }
        }
    }
}
