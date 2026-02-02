macro_rules! make_optional_log_wrapper {
    ($name:ident, $internal_name:ident) => {
        #[allow(unused)]
        macro_rules! $internal_name {
            ($args:tt) => {
                #[cfg(feature = "log")]
                ::log::$name!($args)
            };
        }
        #[allow(unused)]
        pub(crate) use $internal_name as $name;
    };
}

make_optional_log_wrapper!(log, _log);
make_optional_log_wrapper!(trace, _trace);
make_optional_log_wrapper!(debug, _debug);
make_optional_log_wrapper!(info, _info);
make_optional_log_wrapper!(warn, _warn);
make_optional_log_wrapper!(error, _error);
