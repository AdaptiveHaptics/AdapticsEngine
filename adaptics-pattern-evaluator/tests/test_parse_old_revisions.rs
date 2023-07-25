use pattern_evaluator::*;

#[test]
fn test_convert_backwards_compat_format() {

	// 0.0.10-alpha.1
	let converted_to_latest = try_parse_into_latest_version(include_str!("./old-patterns/Heartbeat.adaptics"));
	assert!(converted_to_latest.is_ok());

	//0.1.0-alpha.2
	let converted_to_latest = try_parse_into_latest_version(include_str!("./old-patterns/SpaceshipHeartbeat.adaptics"));
	assert!(converted_to_latest.is_ok());

	let parsed: MidAirHapticsAnimationFileFormat = serde_json::from_str(&converted_to_latest.unwrap()).unwrap();
	assert_eq!(parsed.revision, DataFormatRevision::CurrentRevision); // assert revision updated to latest

}