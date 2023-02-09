cargo build --target wasm32-unknown-unknown --release
wasm-bindgen.exe --target web --out-dir JSWASMOUT ..\target\wasm32-unknown-unknown\release\pattern_evaluator.wasm