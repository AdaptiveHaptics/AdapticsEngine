use interoptopus::util::NamespaceMappings;
use interoptopus::{Error, Interop};

#[test]
fn bindings_csharp() -> Result<(), Error> {
	use interoptopus_backend_csharp::{Config, Generator};
	use interoptopus_backend_csharp::overloads::{DotNet, Unity};

	let config = Config {
		dll_name: "adaptics_engine".to_string(),
		namespace_mappings: NamespaceMappings::new("com.github.AdaptiveHaptics.AdapticsEngine"),
		..Config::default()
	};

	std::fs::remove_dir_all("bindings").ok();
	std::fs::create_dir_all("bindings/csharp").ok();

	Generator::new(config, adaptics_engine::my_inventory())
		.add_overload_writer(DotNet::new())
		// .add_overload_writer(Unity::new()) //requires use_unsafe or something see https://docs.rs/interoptopus_backend_csharp/latest/interoptopus_backend_csharp/overloads/index.html
		.write_file("bindings/csharp/AdapticsEngine.cs")?;

	Ok(())
}