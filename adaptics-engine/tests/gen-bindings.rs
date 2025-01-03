use std::path::Path;

use interoptopus::util::NamespaceMappings;
use interoptopus::Interop;


#[test]
fn generate_bindings() {
	let path = Path::new("bindings");

	std::fs::remove_dir_all(path).ok();

	bindings_csharp(&path.join("csharp"));
	bindings_c(&path.join("c"));
	bindings_websocket(&path.join("websocket"));
}

fn bindings_csharp(path: &Path) {
	std::fs::create_dir_all(path).ok();

	use interoptopus_backend_csharp::{Config, Generator};
	use interoptopus_backend_csharp::overloads::DotNet;
	// use interoptopus_backend_csharp::overloads::Unity;

	let config = Config {
		dll_name: "adaptics_engine".to_string(),
		namespace_mappings: NamespaceMappings::new("com.github.AdaptiveHaptics"),
		class: "AdapticsEngineInterop".to_string(),
		..Config::default()
	};

	Generator::new(config, adaptics_engine::ffi_inventory())
		.add_overload_writer(DotNet::new())
		// .add_overload_writer(Unity::new()) //requires use_unsafe or something see https://docs.rs/interoptopus_backend_csharp/latest/interoptopus_backend_csharp/overloads/index.html
		.write_file(path.join("AdapticsEngineInterop.cs")).unwrap();
}

fn bindings_c(path: &Path) {
	std::fs::create_dir_all(path).ok();

	use interoptopus_backend_c::{Config, Generator};

	let config = Config {
		ifndef: "ADAPTICS_ENGINE_H".to_string(),
		custom_defines: r###"
#ifndef ADAPTICS_EXPORT
#  ifdef _MSC_VER
#    define ADAPTICS_EXPORT __declspec(dllimport)
#  else
#    define ADAPTICS_EXPORT
#  endif
#endif
		"###.to_string(),
		function_attribute: "ADAPTICS_EXPORT ".to_string(),
		prefix: "adaptics_engine_".to_string(),
		type_naming: interoptopus_backend_c::CNamingStyle::SnakeCase,
		..Config::default()
	};

	Generator::new(config, adaptics_engine::ffi_inventory())
		// .add_overload_writer(Unity::new()) //requires use_unsafe or something see https://docs.rs/interoptopus_backend_csharp/latest/interoptopus_backend_csharp/overloads/index.html
		.write_file(path.join("adapticsengine.h")).unwrap();
}

fn bindings_websocket(path: &Path) {
	std::fs::create_dir_all(path).ok();

	use adaptics_engine::AdapticsWSServerMessage;
	let schema = schemars::schema_for!(AdapticsWSServerMessage);
	// let schema_filename = std::env::var("ADAPTICS_ENGINE_CLI_WS_SCHEMA_FILENAME").unwrap();
	let schema_filename = path.join("AdapticsWSServerMessage.json");
	std::fs::write(schema_filename, serde_json::to_string_pretty(&schema).unwrap()).unwrap();
}