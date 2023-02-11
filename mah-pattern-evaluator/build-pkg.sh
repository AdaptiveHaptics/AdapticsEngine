pkg_dir="pkg"
mkdir "$pkg_dir"

json_schema_file="$pkg_dir/rs-shared-types.json"
typescript_defs_file="${json_schema_file%.json}.d.ts"
cargo run -- "$json_schema_file"

cargo build --target wasm32-unknown-unknown --release
wasm-bindgen.exe --target web --out-dir "$pkg_dir" ../target/wasm32-unknown-unknown/release/pattern_evaluator.wasm

sed -i '1s/^/\/\/\@ts-nocheck\n\/\* eslint-disable \*\/\n/' "$pkg_dir/pattern_evaluator.js"

node -p "import('json-schema-to-typescript').then(j => j.compileFromFile('$json_schema_file', { additionalProperties: false }).then(ts => fs.writeFileSync('$typescript_defs_file', ts)))"