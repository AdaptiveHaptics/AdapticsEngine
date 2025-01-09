use std::path::PathBuf;

const LEAP_HEADER_PATH: &str = "C:/Program Files/Ultraleap/LeapSDK/include";

fn main() {
    let out_dir = PathBuf::from(&std::env::var("OUT_DIR").unwrap());

    //*********              build LeapC bindings              *********//
    let bindings = bindgen::Builder::default()
        .header("./wrapper.h")
        .clang_arg(format!("-I{}", LEAP_HEADER_PATH))
		.dynamic_library_name("LeapC")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");

    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Couldn't write bindings!");


}