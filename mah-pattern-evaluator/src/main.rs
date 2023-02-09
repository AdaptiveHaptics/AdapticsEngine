use std::fs;

use schemars::schema_for;
mod shared_types;

fn main() {
	let schema = schema_for!(shared_types::MidAirHapticsAnimationFileFormat);
	let schema_str = serde_json::to_string_pretty(&schema).unwrap();
	println!("{}", schema_str);
	fs::write("types.json", schema_str).unwrap();
	println!("json schema saved to types.json");
}