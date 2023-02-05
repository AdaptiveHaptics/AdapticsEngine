
const LIBRARY_PATH: &str = "C:/Program Files/Ultraleap Haptics/lib";
const DLL_PATH: &str = "C:/Program Files/Ultraleap Haptics/bin";
const HEADER_PATH: &str = "C:/Program Files/Ultraleap Haptics/include";
const LIBCLANG_PATH: &str = "C:/Program Files/LLVM/bin/";


fn main() {
	std::env::set_var("LIBCLANG_PATH", LIBCLANG_PATH); // force bindgen to use explicitly installed LLVM instead of the oldest visual studio LLVM (see https://github.com/KyleMayes/clang-sys/issues/152)
    println!("cargo:rustc-link-search={}", LIBRARY_PATH); // Tell cargo to look for shared libraries in the specified directory
    println!("cargo:rustc-link-search={}", DLL_PATH);
    println!("cargo:rustc-link-lib=UltraleapHaptics"); // Tell cargo to tell rustc to link UltraleapHaptics.lib

    cxx_build::bridge("src/main.rs")
		.include(HEADER_PATH)
		.include("./src")
        .file("src/ulh3-streaming.cpp")
		.flag_if_supported("-std=c++20")
        .compile("ulh3-streaming");

    println!("cargo:rerun-if-changed=src/");
}