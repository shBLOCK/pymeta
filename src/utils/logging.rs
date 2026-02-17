macro_rules! make_optional_log_wrapper {
    ($dollar:tt, $name:ident, $internal_name:ident) => {
        #[allow(unused)]
        macro_rules! $internal_name {
                                            ($dollar($dollar args:tt)+) => {
                                                #[cfg(feature = "log")]
                                                ::log::$name!($dollar($dollar args)+)
                                            };
                                        }
        #[allow(unused)]
        pub(crate) use $internal_name as $name;
    };
}

make_optional_log_wrapper!($, log, _log);
make_optional_log_wrapper!($, trace, _trace);
make_optional_log_wrapper!($, debug, _debug);
make_optional_log_wrapper!($, info, _info);
make_optional_log_wrapper!($, warn, _warn);
make_optional_log_wrapper!($, error, _error);
