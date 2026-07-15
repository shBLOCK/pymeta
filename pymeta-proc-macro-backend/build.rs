fn main() {
    let config = pyo3_build_config::get();
    println!("cargo:rustc-env=PYMETA_LIBPYTHON_NAME={}", config.lib_name().unwrap());
}
