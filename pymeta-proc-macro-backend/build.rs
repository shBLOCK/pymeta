use pyo3_build_config::{InterpreterConfig, PythonImplementation, PythonVersion};
use std::process::exit;

fn check_is_supported(config: &InterpreterConfig) {
    if config.implementation() != PythonImplementation::CPython {
        println!("cargo::error=PyMeta: {} is not supported", config.implementation());
        exit(1);
    }

    const MIN_PYTHON_VERSION: PythonVersion = PythonVersion { major: 3, minor: 12 };
    if config.version() < MIN_PYTHON_VERSION {
        println!(
            "cargo::error=PyMeta: Python {} is not supported, minimal supported version is {MIN_PYTHON_VERSION}",
            config.version()
        );
        exit(1);
    }
}

fn main() {
    let config = pyo3_build_config::get();
    check_is_supported(config);
    println!("cargo:rustc-env=PYMETA_LIBPYTHON_NAME={}", config.lib_name().unwrap());
}
