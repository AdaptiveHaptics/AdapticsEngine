use std::fs;

use schemars::schema_for;
// use ts_rs::TS;
mod shared_types;

use pattern_evaluator;

fn main() {

	let schema_str = { // schemars
		// let schema = schema_for!(shared_types::MidAirHapticsAnimationFileFormat);
		let schema = schema_for!(pattern_evaluator::PatternEvalWasmPublicTypes);
		let schema_str = serde_json::to_string_pretty(&schema).unwrap();
		println!("{}", schema_str);
		schema_str
	};
	// else { // ts_rs
	// 	let schema_str = MidAirHapticsAnimationFileFormat::export_to_string().unwrap(); #only generates one type (not recursive)
	// 	println!("{}", schema_str);
	// 	schema_str
	// };

	let filename = std::env::args().nth(1).unwrap();
	fs::write(&filename, schema_str).unwrap();
	println!("json schema saved to {}", filename);
}