use std::{fs, path::PathBuf};


const ULHAPTICS_LIBRARY_PATH: &str = "C:/Program Files/Ultraleap Haptics/lib";
const ULHAPTICS_DLL_PATH: &str = "C:/Program Files/Ultraleap Haptics/bin";
const ULHAPTICS_HEADER_PATH: &str = "C:/Program Files/Ultraleap Haptics/include";

const LEAP_LIBRARY_PATH: &str = "C:/Program Files/Ultraleap/LeapSDK/lib/x64";
const LEAP_DLL_PATH: &str = "C:/Program Files/Ultraleap/LeapSDK/lib/x64";
const LEAP_HEADER_PATH: &str = "C:/Program Files/Ultraleap/LeapSDK/include";

const LIBCLANG_PATH: &str = "C:/Program Files/LLVM/bin/";

const COPY_DLL_TO_OUT_DIR: bool = false;

fn main() {
    let out_dir = PathBuf::from(&std::env::var("OUT_DIR").unwrap());

    if std::env::var_os("LIBCLANG_PATH").is_none() {
        std::env::set_var("LIBCLANG_PATH", LIBCLANG_PATH); // force bindgen to use explicitly installed LLVM instead of the oldest visual studio LLVM (see https://github.com/KyleMayes/clang-sys/issues/152)
    }

    //*********              build UltraleapHaptics bridge              *********//
    println!("cargo:rustc-link-search={}", ULHAPTICS_LIBRARY_PATH); // Tell cargo to look for shared libraries in the specified directory
    // println!("cargo:rustc-link-search={}", DLL_PATH);
    println!("cargo:rustc-link-lib=UltraleapHaptics"); // Tell cargo to tell rustc to link UltraleapHaptics.lib

    if COPY_DLL_TO_OUT_DIR {
        fs::copy(PathBuf::from(ULHAPTICS_DLL_PATH).join("UltraleapHaptics.dll"), out_dir.join("UltraleapHaptics.dll")).unwrap();
    }

    cxx_build::bridge("src/threads/streaming/ulhaptics/ffi.rs")
		.include(ULHAPTICS_HEADER_PATH)
		.include("./src/threads/streaming/ulhaptics")
        .file("src/threads/streaming/ulhaptics/ulh3-streaming.cpp")
		.flag_if_supported("-std=c++20")
        .compile("ulh3-streaming");

    println!("cargo:rerun-if-changed=src/threads/streaming/ulhaptics/ffi.rs");
    println!("cargo:rerun-if-changed=src/threads/streaming/ulhaptics/ulh3-streaming.cpp");
    println!("cargo:rerun-if-changed=src/threads/streaming/ulhaptics/ulh3-streaming.h");



    //*********              build LeapC bridge              *********//
    if COPY_DLL_TO_OUT_DIR {
        fs::copy(PathBuf::from(LEAP_DLL_PATH).join("LeapC.dll"), out_dir.join("LeapC.dll")).unwrap();
    }
    println!("cargo:rustc-link-search={}", LEAP_LIBRARY_PATH);
    // println!("cargo:rustc-link-search={}", DLL_PATH);
    println!("cargo:rustc-link-lib=LeapC"); // Tell cargo to tell rustc to link LeapC.lib

    cxx_build::bridge("src/threads/tracking/leapmotion/ffi.rs")
		.include(LEAP_HEADER_PATH)
		.include("./src/threads/tracking/leapmotion")
        .file("src/threads/tracking/leapmotion/lmc-track.cpp")
		.flag_if_supported("-std=c++20")
        .compile("lmc-track");

    println!("cargo:rerun-if-changed=src/threads/tracking/leapmotion/ffi.rs");
    println!("cargo:rerun-if-changed=src/threads/tracking/leapmotion/lmc-track.cpp");
    println!("cargo:rerun-if-changed=src/threads/tracking/leapmotion/lmc-track.h");
}