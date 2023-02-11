use std::fs;

use schemars::schema_for;
mod shared_types;

fn main() {
	let schema = schema_for!(shared_types::MidAirHapticsAnimationFileFormat);
	let schema_str = serde_json::to_string_pretty(&schema).unwrap();
	println!("{}", schema_str);

	let filename = std::env::args().nth(1).unwrap();
	fs::write(&filename, schema_str).unwrap();
	println!("json schema saved to {}", filename);
}