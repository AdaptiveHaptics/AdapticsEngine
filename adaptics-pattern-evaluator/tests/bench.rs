use pattern_evaluator::*;

use std::{time::{Instant, Duration}, collections::HashMap, path::Path};


fn bench_pattern_evaluator(pe: PatternEvaluator, max_i: u32, max_o: u32) -> Vec<Duration> {
	let step_ms = 0.05;
	let mut all_elapsed = Vec::with_capacity(max_o as usize);

	for o in 0..max_o {
		let now = Instant::now();

		let mut pep = PatternEvaluatorParameters { time: 0.0, user_parameters: Default::default(), geometric_transform: Default::default() };
		let mut last_nep = NextEvalParams::default();
		for i in 0..max_i {
			let time = f64::from(i) * step_ms + f64::from(o) * f64::from(max_i) * step_ms;
			pep.time = time;
			let eval_result = pe.eval_path_at_anim_local_time(&pep, &last_nep);
			if eval_result.ul_control_point.coords.z != 200.0 {
				println!("{:?}", eval_result);
			}
			last_nep = eval_result.next_eval_params;
		}

		let elapsed = now.elapsed() / max_i;
		all_elapsed.push(elapsed);
	}

	// print average elapsed
	let avg_elapsed = all_elapsed.iter().fold(Duration::default(), |acc, x| acc + *x) / max_o;
	let avg_khz = 1.0 / (avg_elapsed.as_secs_f64() * 1000.0);
	println!("avg: {:?} (~{:.0}kHz)", avg_elapsed, avg_khz);

	all_elapsed
}

#[test]
#[ignore="bench"]
fn bench() {
	let max_i = 500; // 40
	let max_o = 30000; // 1000

	let rainbench_pat = include_str!("../tests/old-patterns/BenchRain.adaptics");
	let rainbench_pat_2x = { // 4x is still 500kHz on my machine, but makes the box plot seem like there is an exponential performance falloff. 2x might be better to show the linear slope.
		let mut rainbench_pat_2x: MidAirHapticsAnimationFileFormat = serde_json::from_str(rainbench_pat).unwrap();
		rainbench_pat_2x.keyframes.extend(rainbench_pat_2x.keyframes.clone());
		rainbench_pat_2x
	};

	let bench_pes = [
		("base", PatternEvaluator::new(base_bench_pattern())),
		("rainbench", PatternEvaluator::new_from_json_string(rainbench_pat).unwrap()),
		("rainbench2x", PatternEvaluator::new(rainbench_pat_2x)),
		("rainbenchmoreformulas", PatternEvaluator::new_from_json_string(include_str!("../tests/old-patterns/BenchRainMoreFormulas.adaptics")).unwrap()),
	];

	let csv_file: String = bench_pes.into_iter()
		.map(|(name, pe)| (name, bench_pattern_evaluator(pe, max_i, max_o))) //run bench
		.map(|(name, all_elapsed)| name.to_owned() + "," + &all_elapsed.iter().map(|x| x.as_secs_f64().to_string()).collect::<Vec<_>>().join(",")) //convert rows to csv
		.map(|row| row + "\n")
		.collect();


	let csv_filename = Path::new("benchresults.csv");
	std::fs::write(csv_filename, csv_file).unwrap();
}

fn duration_abs_diff(a: Duration, b: Duration) -> Duration {
	if a > b {
		a - b
	} else {
		b - a
	}
}

#[test]
#[ignore="bench"]
fn bench_buffer_size_curve() {
	// let max_i = 40; // 500 cb/sec @ 20kHz
	let max_o = 30000;

	let rainbench_pat = include_str!("../tests/old-patterns/BenchRain.adaptics");

	let mut csv_file = "buffer_size_ms,avg_elapsed_ns,avg_no_outliers\n".to_string();

	for buffer_size in [1,2,4,8,16,32,64,128,256,512,1024] {
		let pe = PatternEvaluator::new_from_json_string(rainbench_pat).unwrap();

		let max_i = buffer_size;

		let all_elapsed = bench_pattern_evaluator(pe, max_i, max_o);
		let avg_elapsed = all_elapsed.iter().fold(Duration::default(), |acc, x| acc + *x) / max_o;
		let variance = all_elapsed.iter().fold(Duration::default(), |acc, x| acc + duration_abs_diff(*x, avg_elapsed)) / max_o;
		let no_outliers: Vec<_> = all_elapsed.iter().filter(|x| **x < avg_elapsed + variance).collect();
		let avg_no_outliers = no_outliers.iter().fold(Duration::default(), |acc, x| acc + **x) / no_outliers.len() as u32;

		let buffer_size_ms = buffer_size as f64 / 20.0; //=20kHz
		csv_file += &format!("{},{},{}\n", buffer_size_ms, avg_elapsed.as_nanos(), avg_no_outliers.as_nanos());
	}

	let csv_filename = Path::new("benchresults_buffer_size_curve.csv");
	std::fs::write(csv_filename, csv_file).unwrap();
}

fn base_bench_pattern() -> MidAirHapticsAnimationFileFormat {
	MidAirHapticsAnimationFileFormat {
		data_format: MidAirHapticsAnimationFileFormatDataFormatName::DataFormat,
		revision: DataFormatRevision::CurrentRevision,
		name: "example".to_string(),
		keyframes: vec![
			MAHKeyframe::Standard(MAHKeyframeStandard {
				time: 0.0,
				brush: Some(BrushWithTransition {
					brush: MAHBrush::Circle { radius: MAHDynamicF64::F64(10.0), am_freq: MAHDynamicF64::F64(0.0), stm_freq: MAHDynamicF64::F64(100.0) },
					transition: MAHTransition::Linear {  }
				}),
				intensity: Some(IntensityWithTransition {
					intensity: MAHIntensity::Constant { value: MAHDynamicF64::F64(1.0) },
					transition: MAHTransition::Linear { },
				}),
				coords: CoordsWithTransition {
					coords: MAHCoordsConst { x: -10.0, y: 0.0, z: 0.0 },
					transition: MAHTransition::Linear { },
				},
				cjumps: vec![],
			}),
		],
		pattern_transform: Default::default(),
		user_parameter_definitions: HashMap::from([
			("param1".to_string(), MAHUserParameterDefinition { default: 0.0, min: Some(0.0), max: Some(10.0), step: 1.0 }),
			("param2".to_string(), MAHUserParameterDefinition { default: 20.0, min: Some(0.0), max: Some(15.0), step: 15.0 }),
			("param3".to_string(), MAHUserParameterDefinition { default: 0.0, min: Some(0.0), max: Some(10.0), step: -500.0 }),
			("param4".to_string(), MAHUserParameterDefinition { default: 75.0, min: Some(-100.0), max: Some(50.0), step: 13.0 }),
			("param5".to_string(), MAHUserParameterDefinition { default: 1.0, min: Some(0.0), max: Some(4.0), step: 0.05 }),
		]),
	}
}

//#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
// #[test]
// #[ignore="bench"]
// fn benchwasm() {
//     let warmup_iterations = 5;
//     let pe = PatternEvaluator::new_from_json_string(&create_test_pattern_json()).unwrap();
//     for o in 0..3000 {
//         if o == warmup_iterations {
//             println!("Warmup done, starting benchmark..");
//         }

//         let mut pep = PatternEvaluatorParameters { time: 0.0, user_parameters: Default::default(), geometric_transform: Default::default() };
//         let mut last_nep = NextEvalParams { last_eval_pattern_time: 0.0, time_offset: 0.0 };
//         for i in 0..200 {
//             let time = f64::from(i) * 0.05;
//             pep.time = time;
//             let eval_result = pe.eval_path_at_anim_local_time(&pep, &last_nep);
//             if eval_result.ul_control_point.coords.z != 200.0 {
//                 println!("{:?}", eval_result);
//             }
//             last_nep = eval_result.next_eval_params;
//         }
//     }
// }