#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#[allow(clippy::all)]
#[allow(rustdoc::broken_intra_doc_links)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

#[macro_export]
macro_rules! LIBRARY_BASENAME { () => { "LeapC" }; }

#[macro_export]
#[cfg(target_os = "windows")]
macro_rules! LIBRARY_NAME { () => { concat!(LIBRARY_BASENAME!(), ".dll") }; }
#[macro_export]
#[cfg(target_os = "macos")]
/// untested
macro_rules! LIBRARY_NAME { () => { concat!("lib", LIBRARY_BASENAME!(), ".dylib") }; }
#[macro_export]
#[cfg(not(any(target_os = "windows", target_os = "macos")))] // use linux as default
/// untested
macro_rules! LIBRARY_NAME { () => { concat!("lib", LIBRARY_BASENAME!(), ".so") }; }


#[macro_export]
#[cfg(target_os = "windows")]
macro_rules! LIBRARY_DIR { () => { "C:/Program Files/Ultraleap/LeapSDK/lib/x64" }; }
#[macro_export]
#[cfg(target_os = "macos")]
/// untested
macro_rules! LIBRARY_DIR { () => { "/Library/Application Support/Ultraleap/LeapSDK/lib/" }; }
#[macro_export]
#[cfg(not(any(target_os = "windows", target_os = "macos")))] // use linux as default
/// untested
macro_rules! LIBRARY_DIR { () => { "/usr/lib/ultraleap-hand-tracking-service/" }; }

pub const LIBRARY_BASENAME: &str = LIBRARY_BASENAME!();
/// non-windows platforms are untested
pub const LIBRARY_NAME: &str = LIBRARY_NAME!();
/// non-windows platforms are untested
pub const LIBRARY_DIR: &str = LIBRARY_DIR!();
/// non-windows platforms are untested
pub const LIBRARY_FULLPATH: &str = concat!(LIBRARY_DIR!(), "/", LIBRARY_NAME!());

pub use bindings::*;

/// Try to load the LeapC library known locations.
///
/// # Safety
/// Follows the same safety requirements as [`libloading::Library::new`].
///
/// Technically, we could assume that the initializers are safe (since we know what library we are linking against) and make this function safe, but I don't want to make that promise here.
pub unsafe fn load_leapc_library() -> Result<LeapC, libloading::Error> {
    std::env::var_os("LEAPC_LIBRARY_PATH").map_or(Err(libloading::Error::DlOpenUnknown), |env_path| LeapC::new(env_path))
        .or_else(|_e| LeapC::new(LIBRARY_BASENAME))  // LeapC
        .or_else(|_e| LeapC::new(LIBRARY_NAME))      // LeapC.dll
        .or_else(|_e| LeapC::new(LIBRARY_FULLPATH))  // try to load with the full path to default
}
