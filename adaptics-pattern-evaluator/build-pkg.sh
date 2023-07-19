set -ex


if ! grep -q 'name = "adaptics-pattern-evaluator"' Cargo.toml; then
	echo "This script must be run from the "subroot" for the adaptics-pattern-evaluator package"
	exit 1
fi

pkg_dir="pkg"
rm -rf "$pkg_dir"
mkdir "$pkg_dir"

json_schema_file="$pkg_dir/rs-shared-types.json"
typescript_defs_file="${json_schema_file%.json}.d.ts"

cargo clippy
cargo test

# schemars
cargo run -- "$json_schema_file" >/dev/null
node -p "import('json-schema-to-typescript').then(j => j.compileFromFile('$json_schema_file', { additionalProperties: false }).then(ts => fs.writeFileSync('$typescript_defs_file', ts)))"

#cargo build --target wasm32-unknown-unknown --release
#wasm-bindgen.exe --target web --weak-refs --reference-types --out-dir "$pkg_dir" ../target/wasm32-unknown-unknown/release/pattern_evaluator.wasm
wasm-pack build --target web --weak-refs --reference-types --out-dir "$pkg_dir" --release

# include all pkg files in npm package
rm "$pkg_dir/.gitignore"
node -e "const fs=require('fs'); const pkg=JSON.parse(fs.readFileSync('$pkg_dir/package.json')); delete pkg.files; fs.writeFileSync('$pkg_dir/package.json', JSON.stringify(pkg, null, 2))"

# add ts-nocheck and eslint-disable to top of generated js file
sed -i '1s/^/\/\/\@ts-nocheck\n\/\* eslint-disable \*\/\n/' "$pkg_dir/pattern_evaluator.js"

wasm-pack pack "$pkg_dir" #--scope adaptics --no-typescript

echo "Successfully built package in $pkg_dir"