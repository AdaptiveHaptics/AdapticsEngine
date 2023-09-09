set -ex

if ! grep -q 'name = "adaptics-engine"' Cargo.toml; then
	echo "This script must be run from the "subroot" for the mah-driver-server package"
	exit 1
fi

pkg_dir="pkg"
rm -rf "$pkg_dir"
mkdir "$pkg_dir"


cargo clippy
cargo test --release # also generates bindings (see test/gen-bindings.rs)
cargo build --release

json_schema_file="bindings/websocket/AdapticsWSServerMessage.json"
typescript_defs_file="${json_schema_file%.json}.d.ts"
node -p "import('json-schema-to-typescript').then(j => j.compileFromFile('$json_schema_file', { additionalProperties: false }).then(ts => fs.writeFileSync('$typescript_defs_file', ts)))"

cp ../target/release/adaptics-engine-cli.exe "$pkg_dir/adaptics-engine-cli.exe"

mkdir "$pkg_dir/unity"
cp ../target/release/adaptics_engine.dll "$pkg_dir/unity/adaptics_engine.dll"
cp ./bindings/csharp/AdapticsEngineInterop.cs "$pkg_dir/unity/AdapticsEngineInterop.cs"

mkdir "$pkg_dir/c"
cp ../target/release/adaptics_engine.dll "$pkg_dir/c/adaptics_engine.dll"
cp ../target/release/adaptics_engine.dll.lib "$pkg_dir/c/adaptics_engine.dll.lib"
cp ./bindings/c/adapticsengine.h "$pkg_dir/c/adapticsengine.h"

echo "Successfully built package in $pkg_dir"